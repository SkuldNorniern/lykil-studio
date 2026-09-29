//! Other Dynamic Lighting devices through `Windows.Devices.Lights`.
//! Windows lets an unpackaged app light them only while it is in front.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use lykil::lighting::{Effect, Moment, Point, Settings, shade};
use lykil::time::Tick;

use crate::app::Shared;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lamp {
    pub id: String,
    pub name: String,
    pub kind: &'static str,
    pub lamps: u32,
    /// May Studio set its colours right now (Studio in front, user
    /// settings)?
    pub available: bool,
    pub follow: bool,
    /// Windows has handed the device over; until then it cannot be lit.
    pub open: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LampCommand {
    Sync(bool),
    Follow(String, bool),
}

const RESCAN: Duration = Duration::from_secs(3);
const FRAME: Duration = Duration::from_millis(33);

pub fn spawn(shared: Arc<Mutex<Shared>>) -> Sender<LampCommand> {
    let (tx, rx) = channel();
    thread::spawn(move || run(&shared, &rx));
    tx
}

#[cfg(windows)]
fn run(shared: &Arc<Mutex<Shared>>, rx: &Receiver<LampCommand>) {
    let mut devices = os::Devices::default();
    let mut sync = false;
    let mut next_scan = Instant::now();
    let start = Instant::now();
    loop {
        while let Ok(command) = rx.try_recv() {
            match command {
                LampCommand::Sync(on) => sync = on,
                LampCommand::Follow(id, on) => devices.follow(&id, on),
            }
        }
        if Instant::now() >= next_scan {
            next_scan = Instant::now() + RESCAN;
            devices.rescan();
        }
        devices.settle();
        let settings = {
            let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
            s.lamps = devices.views();
            s.lamp_sync = sync;
            s.lighting()
        };
        if sync && let Some(settings) = settings {
            #[allow(clippy::cast_possible_truncation)]
            let now = Tick(start.elapsed().as_millis() as u32);
            devices.show(|at| colour(settings, at, now));
        }
        thread::sleep(if sync { FRAME } else { RESCAN / 6 });
    }
}

#[cfg(not(windows))]
fn run(_shared: &Arc<Mutex<Shared>>, rx: &Receiver<LampCommand>) {
    while rx.recv().is_ok() {}
}

/// A lamp's colour under `settings` at `now`. Per-key and off leave
/// other devices dark; the press effects show their resting glow.
fn colour(settings: Settings, at: Point, now: Tick) -> lykil::lighting::Rgb {
    let s = match settings.effect {
        Effect::PerKey => Settings {
            effect: Effect::Solid,
            ..settings
        },
        _ => settings,
    };
    let moment = Moment {
        now,
        ..Moment::default()
    };
    shade(s, at, &moment).to_rgb()
}

#[cfg(windows)]
mod os {
    use lykil::lighting::{Point, Rgb};
    use windows::Devices::Enumeration::DeviceInformation;
    use windows::Devices::Lights::{LampArray, LampArrayKind};
    use windows::UI::Color;
    use windows_future::{AsyncStatus, IAsyncOperation};

    use super::Lamp;

    struct Device {
        pub id: String,
        name: String,
        array: LampArray,
        points: Vec<Point>,
        indices: Vec<i32>,
    }

    /// A device Windows has not handed over yet. `FromIdAsync` only
    /// finishes for a process with a window in front, so it is never
    /// waited on.
    struct Opening {
        id: String,
        name: String,
        op: IAsyncOperation<LampArray>,
    }

    #[derive(Default)]
    pub struct Devices {
        open: Vec<Device>,
        opening: Vec<Opening>,
        /// Followed ids, kept across opening and unplugging.
        followed: Vec<String>,
    }

    impl Devices {
        pub fn rescan(&mut self) {
            let Ok(found) = LampArray::GetDeviceSelector()
                .and_then(|s| DeviceInformation::FindAllAsyncAqsFilter(&s))
                .and_then(|op| op.join())
            else {
                return;
            };
            let mut seen = Vec::new();
            for info in found {
                let Ok(id) = info.Id() else { continue };
                let id_text = id.to_string();
                seen.push(id_text.clone());
                let known = self.open.iter().any(|d| d.id == id_text)
                    || self.opening.iter().any(|o| o.id == id_text);
                if known {
                    continue;
                }
                let name = info
                    .Name()
                    .map_or_else(|_| "unnamed".into(), |n| n.to_string());
                if let Ok(op) = LampArray::FromIdAsync(&id) {
                    self.opening.push(Opening {
                        id: id_text,
                        name,
                        op,
                    });
                }
            }
            self.open.retain(|d| seen.contains(&d.id));
            self.opening.retain(|o| seen.contains(&o.id));
        }

        pub fn settle(&mut self) {
            let mut still = Vec::new();
            for o in self.opening.drain(..) {
                match o.op.Status() {
                    Ok(AsyncStatus::Completed) => {
                        if let Some(d) =
                            o.op.GetResults()
                                .ok()
                                .and_then(|a| Device::new(&o.id, o.name.clone(), a))
                        {
                            self.open.push(d);
                        }
                    }
                    Ok(AsyncStatus::Started) => still.push(o),
                    // Failed or cancelled: the next scan tries again.
                    _ => {}
                }
            }
            self.opening = still;
        }

        pub fn follow(&mut self, id: &str, on: bool) {
            self.followed.retain(|f| f != id);
            if on {
                self.followed.push(id.to_string());
            }
        }

        pub fn views(&self) -> Vec<Lamp> {
            let follows = |id: &str| self.followed.iter().any(|f| f == id);
            let mut out: Vec<Lamp> = self
                .open
                .iter()
                .map(|d| Lamp {
                    follow: follows(&d.id),
                    ..d.view()
                })
                .collect();
            out.extend(self.opening.iter().map(|o| Lamp {
                id: o.id.clone(),
                name: o.name.clone(),
                kind: "device",
                lamps: 0,
                available: false,
                follow: follows(&o.id),
                open: false,
            }));
            out.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
            out
        }

        pub fn show(&self, colour: impl Fn(Point) -> Rgb) {
            for d in &self.open {
                if self.followed.contains(&d.id) {
                    d.show(&colour);
                }
            }
        }
    }

    impl Device {
        fn new(id: &str, name: String, array: LampArray) -> Option<Self> {
            let count = array.LampCount().ok()?;
            let bounds = array.BoundingBox().ok()?;
            // Across the widest side, as the firmware scales its LEDs.
            let span = bounds.X.max(bounds.Y).max(1e-6);
            let points = (0..count)
                .map(|i| {
                    let p = array
                        .GetLampInfo(i)
                        .and_then(|l| l.Position())
                        .unwrap_or_default();
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let scale = |v: f32| (v / span * 255.0).clamp(0.0, 255.0) as u8;
                    Point::new(scale(p.X), scale(p.Y))
                })
                .collect();
            Some(Self {
                id: id.to_string(),
                name,
                array,
                points,
                indices: (0..count).collect(),
            })
        }

        pub fn view(&self) -> Lamp {
            let kind = self.array.LampArrayKind().unwrap_or_default();
            Lamp {
                id: self.id.clone(),
                name: self.name.clone(),
                kind: kind_name(kind),
                lamps: u32::try_from(self.indices.len()).unwrap_or(0),
                available: self.array.IsAvailable().unwrap_or(false),
                follow: false,
                open: true,
            }
        }

        fn show(&self, colour: &impl Fn(Point) -> Rgb) {
            if !self.array.IsAvailable().unwrap_or(false) {
                return;
            }
            let colors: Vec<Color> = self
                .points
                .iter()
                .map(|p| {
                    let c = colour(*p);
                    Color {
                        A: 255,
                        R: c.r,
                        G: c.g,
                        B: c.b,
                    }
                })
                .collect();
            let _ = self.array.SetColorsForIndices(&colors, &self.indices);
        }
    }

    fn kind_name(kind: LampArrayKind) -> &'static str {
        match kind {
            LampArrayKind::Keyboard => "keyboard",
            LampArrayKind::Mouse => "mouse",
            LampArrayKind::GameController => "game controller",
            LampArrayKind::Peripheral => "peripheral",
            LampArrayKind::Scene => "scene",
            LampArrayKind::Notification => "notification",
            LampArrayKind::Chassis => "chassis",
            LampArrayKind::Wearable => "wearable",
            LampArrayKind::Furniture => "furniture",
            LampArrayKind::Art => "art",
            LampArrayKind::Headset => "headset",
            LampArrayKind::Microphone => "microphone",
            LampArrayKind::Speaker => "speaker",
            _ => "device",
        }
    }
}
