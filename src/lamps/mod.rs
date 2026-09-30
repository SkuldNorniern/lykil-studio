//! Other Dynamic Lighting devices through `Windows.Devices.Lights`.
//! Windows lets an unpackaged app light them only while it is in front.

pub mod desk;

#[cfg(windows)]
use std::sync::PoisonError;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
#[cfg(windows)]
use std::time::{Duration, Instant};

use lykil::lighting::{Effect, Moment, Point, Rgb, Settings, shade};
use lykil::time::Tick;

use crate::app::Shared;
use crate::lamps::desk::Place;

#[derive(Clone, Debug, PartialEq)]
pub struct Lamp {
    pub id: String,
    pub name: String,
    pub kind: &'static str,
    /// May Studio set its colours right now (Studio in front, user
    /// settings)?
    pub available: bool,
    /// Windows has handed the device over; until then it cannot be lit.
    pub open: bool,
    pub place: Place,
    /// Width and depth in metres.
    pub size: (f32, f32),
    /// Each lamp's place on the device, in metres from its top left.
    pub lamps: Vec<(f32, f32)>,
}

/// Effects a device can run on its own: none of them need key presses.
pub const OWN_EFFECTS: [Effect; 6] = [
    Effect::Solid,
    Effect::Breathing,
    Effect::Cycle,
    Effect::Wave,
    Effect::Starlight,
    Effect::Rain,
];

#[derive(Clone, Debug, PartialEq)]
pub enum LampCommand {
    Sync(bool),
    Follow(String, bool),
    /// Moves a device on the desk, in metres.
    Place(String, f32, f32),
    Level(String, u8),
    /// A device's own effect, or `None` to show the keyboard's.
    Own(String, Option<Settings>),
}

#[cfg(windows)]
const RESCAN: Duration = Duration::from_secs(3);
#[cfg(windows)]
const FRAME: Duration = Duration::from_millis(33);
/// Size of a device Windows has not described yet.
#[cfg(windows)]
const UNKNOWN_SIZE: (f32, f32) = (0.12, 0.06);

pub fn spawn(shared: Arc<Mutex<Shared>>) -> Sender<LampCommand> {
    let (tx, rx) = channel();
    thread::spawn(move || run(&shared, &rx));
    tx
}

#[cfg(windows)]
fn run(shared: &Arc<Mutex<Shared>>, rx: &Receiver<LampCommand>) {
    let mut devices = os::Devices::new(crate::lamps::desk::Desk::load());
    let mut sync = false;
    let mut next_scan = Instant::now();
    let start = Instant::now();
    loop {
        while let Ok(command) = rx.try_recv() {
            match command {
                LampCommand::Sync(on) => sync = on,
                command => devices.command(&command),
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
            devices.show(|own, at| colour(own.unwrap_or(settings), at, now));
        }
        thread::sleep(if sync { FRAME } else { RESCAN / 6 });
    }
}

#[cfg(not(windows))]
fn run(_shared: &Arc<Mutex<Shared>>, rx: &Receiver<LampCommand>) {
    while rx.recv().is_ok() {}
}

/// A lamp's colour under `settings` at `now`. Per-key shows the plain
/// colour; the press effects show their resting glow.
pub fn colour(settings: Settings, at: Point, now: Tick) -> Rgb {
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

/// Every lamp's colour, per device in `lamps`, lit in desk space.
pub fn colours(lamps: &[Lamp], settings: Settings, now: Tick) -> Vec<Vec<Rgb>> {
    let frame = crate::lamps::desk::Frame::around(
        lamps
            .iter()
            .filter(|l| l.place.follow)
            .map(|l| (l.place, l.size)),
    );
    lamps
        .iter()
        .map(|l| {
            l.lamps
                .iter()
                .map(|at| {
                    let s = l.place.own.unwrap_or(settings);
                    colour(s, frame.point(l.place, *at), now).scale(l.place.level)
                })
                .collect()
        })
        .collect()
}

#[cfg(windows)]
mod os {
    use lykil::lighting::{Point, Rgb, Settings};
    use windows::Devices::Enumeration::DeviceInformation;
    use windows::Devices::Lights::{LampArray, LampArrayKind};
    use windows::UI::Color;
    use windows_future::{AsyncStatus, IAsyncOperation};

    use super::{Lamp, LampCommand, UNKNOWN_SIZE};
    use crate::lamps::desk::{Desk, Frame, Place};

    struct Device {
        id: String,
        name: String,
        array: LampArray,
        size: (f32, f32),
        lamps: Vec<(f32, f32)>,
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

    pub struct Devices {
        open: Vec<Device>,
        opening: Vec<Opening>,
        desk: Desk,
        /// Newly found devices since the last untangle.
        fresh: bool,
    }

    impl Devices {
        pub fn new(desk: Desk) -> Self {
            Self {
                open: Vec::new(),
                opening: Vec::new(),
                desk,
                fresh: false,
            }
        }

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
                    self.fresh = true;
                    self.opening.push(Opening {
                        id: id_text,
                        name,
                        op,
                    });
                }
            }
            let before = self.open.len() + self.opening.len();
            self.open.retain(|d| seen.contains(&d.id));
            self.opening.retain(|o| seen.contains(&o.id));
            if self.open.len() + self.opening.len() != before || self.fresh {
                self.fresh = false;
                self.untangle();
            }
        }

        /// Every device with its size, open or not.
        fn sized(&self) -> Vec<(String, (f32, f32))> {
            self.open
                .iter()
                .map(|d| (d.id.clone(), d.size))
                .chain(self.opening.iter().map(|o| (o.id.clone(), UNKNOWN_SIZE)))
                .collect()
        }

        /// Slides devices off each other, first come first kept.
        fn untangle(&mut self) {
            let mut placed: Vec<(Place, (f32, f32))> = Vec::new();
            let mut moved = false;
            for (id, size) in self.sized() {
                let Some(place) = self.desk.get(&id) else {
                    continue;
                };
                let spot = crate::lamps::desk::free_spot((place, size), &placed);
                if spot != place {
                    self.desk.set(&id, spot);
                    moved = true;
                }
                placed.push((spot, size));
            }
            if moved {
                self.desk.save();
            }
        }

        pub fn settle(&mut self) {
            let mut still = Vec::new();
            let opening = std::mem::take(&mut self.opening);
            for o in opening {
                match o.op.Status() {
                    Ok(AsyncStatus::Completed) => {
                        if let Some(d) =
                            o.op.GetResults()
                                .ok()
                                .and_then(|a| Device::new(&o.id, o.name.clone(), a))
                        {
                            self.seat(&d.id, d.size);
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

        /// Moves a device whose real size just arrived off any it now
        /// overlaps.
        fn seat(&mut self, id: &str, size: (f32, f32)) {
            let Some(place) = self.desk.get(id) else {
                return;
            };
            let others: Vec<(Place, (f32, f32))> = self
                .open
                .iter()
                .filter(|d| d.id != id)
                .filter_map(|d| Some((self.desk.get(&d.id)?, d.size)))
                .collect();
            let spot = crate::lamps::desk::free_spot((place, size), &others);
            if spot != place {
                self.desk.set(id, spot);
                self.desk.save();
            }
        }

        pub fn command(&mut self, command: &LampCommand) {
            let id = match command {
                LampCommand::Follow(id, _)
                | LampCommand::Place(id, ..)
                | LampCommand::Level(id, _)
                | LampCommand::Own(id, _) => id.clone(),
                LampCommand::Sync(_) => return,
            };
            let mut place = self.desk.get(&id).unwrap_or_default();
            match command {
                LampCommand::Follow(_, on) => place.follow = *on,
                LampCommand::Place(_, x, y) => (place.x, place.y) = (*x, *y),
                LampCommand::Level(_, level) => place.level = *level,
                LampCommand::Own(_, own) => place.own = *own,
                LampCommand::Sync(_) => {}
            }
            self.desk.set(&id, place);
            self.desk.save();
        }

        pub fn views(&mut self) -> Vec<Lamp> {
            let before = self.desk.clone();
            let mut out: Vec<Lamp> = Vec::new();
            for (id, name, kind, available, open, size, lamps) in self
                .open
                .iter()
                .map(|d| {
                    let kind = d.array.LampArrayKind().unwrap_or_default();
                    (
                        d.id.clone(),
                        d.name.clone(),
                        kind_name(kind),
                        d.array.IsAvailable().unwrap_or(false),
                        true,
                        d.size,
                        d.lamps.clone(),
                    )
                })
                .chain(self.opening.iter().map(|o| {
                    (
                        o.id.clone(),
                        o.name.clone(),
                        "device",
                        false,
                        false,
                        UNKNOWN_SIZE,
                        Vec::new(),
                    )
                }))
                .collect::<Vec<_>>()
            {
                let placed: Vec<(Place, f32)> = out.iter().map(|l| (l.place, l.size.0)).collect();
                let place = self.desk.place_new(&id, &placed);
                out.push(Lamp {
                    id,
                    name,
                    kind,
                    available,
                    open,
                    place,
                    size,
                    lamps,
                });
            }
            if self.desk != before {
                self.desk.save();
            }
            out.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
            out
        }

        /// Lights every followed device, in desk space, with its own
        /// effect if it has one.
        pub fn show(&self, colour: impl Fn(Option<Settings>, Point) -> Rgb) {
            let followed = |d: &&Device| self.desk.get(&d.id).is_some_and(|p| p.follow);
            let frame = Frame::around(
                self.open
                    .iter()
                    .filter(followed)
                    .filter_map(|d| Some((self.desk.get(&d.id)?, d.size))),
            );
            for d in self.open.iter().filter(followed) {
                let Some(place) = self.desk.get(&d.id) else {
                    continue;
                };
                d.show(|at| colour(place.own, frame.point(place, at)).scale(place.level));
            }
        }
    }

    impl Device {
        fn new(id: &str, name: String, array: LampArray) -> Option<Self> {
            let count = array.LampCount().ok()?;
            let bounds = array.BoundingBox().ok()?;
            let lamps = (0..count)
                .map(|i| {
                    let p = array
                        .GetLampInfo(i)
                        .and_then(|l| l.Position())
                        .unwrap_or_default();
                    (p.X, p.Y)
                })
                .collect();
            Some(Self {
                id: id.to_string(),
                name,
                array,
                size: (bounds.X.max(0.01), bounds.Y.max(0.01)),
                lamps,
                indices: (0..count).collect(),
            })
        }

        fn show(&self, colour: impl Fn((f32, f32)) -> Rgb) {
            if !self.array.IsAvailable().unwrap_or(false) {
                return;
            }
            let colors: Vec<Color> = self
                .lamps
                .iter()
                .map(|at| {
                    let c = colour(*at);
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
