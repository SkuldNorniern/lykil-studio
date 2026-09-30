//! The keyboard, on a background thread.

use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use aurea::render::{CanvasId, request_canvas_redraw};
use lykil::binding::Binding;
use lykil::lighting::{Rgb, Settings};
use lykil::macros::{MACROS, Step};
use lykil_device::{Device, DeviceError, FirmwareInfo, Found, open_any};
use lykil_protocol::describe::Description;
use lykil_protocol::lcp::{Diagnostics, Hello, LightingInfo, capability};

use crate::app::Shared;

const MATRIX_EVERY: Duration = Duration::from_millis(20);
const DIAGNOSTICS_EVERY: Duration = Duration::from_secs(1);
const RETRY_EVERY: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Keyboard {
    pub connection: Connection,
    pub name: String,
    pub hello: Option<Hello>,
    pub via: Option<crate::via::Via>,
    pub firmware: Option<FirmwareInfo>,
    pub description: Option<Description>,
    pub diagnostics: Option<Diagnostics>,
    pub matrix: Vec<u32>,
    pub keymap: Vec<Vec<Binding>>,
    pub lighting: Option<LightingInfo>,
    pub key_colors: Vec<Rgb>,
    pub macros: Vec<Vec<Step>>,
    /// The status of the last change the keyboard refused, for the status
    /// line.
    pub error: Option<String>,
    /// While searching: raw HID interfaces that are there but did not
    /// answer, and why.
    pub seen: Vec<String>,
    /// A VIA keyboard's settings from its definition's menus, with what
    /// they are set to.
    pub via_settings: Vec<crate::via::ViaSetting>,
}

impl Keyboard {
    pub fn layer_names(&self) -> Vec<String> {
        self.description
            .as_ref()
            .map(|d| d.layers.clone())
            .unwrap_or_default()
    }

    pub fn closed(&self, (row, col): (u8, u8)) -> bool {
        let columns = self.description.as_ref().is_some_and(|d| d.columns_driven);
        let (line, bit) = if columns { (col, row) } else { (row, col) };
        self.matrix
            .get(usize::from(line))
            .is_some_and(|bits| bit < 32 && bits >> bit & 1 == 1)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Connection {
    #[default]
    Searching,
    Connected,
    /// A VIA keyboard whose definition is not in [`crate::via::folder`].
    NeedsDefinition,
    Lost(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    SetBinding {
        layer: u8,
        key: u16,
        binding: Binding,
    },
    ResetKeymap,
    SetLighting(Settings),
    SetKeyColors {
        start: u16,
        colors: Vec<Rgb>,
    },
    SetMacro {
        id: u8,
        steps: Vec<Step>,
    },
    /// A VIA setting, by its place in [`Keyboard::via_settings`].
    SetVia {
        setting: usize,
        value: Vec<u8>,
    },
}

pub fn spawn(shared: Arc<Mutex<Shared>>, canvas: CanvasId) -> Sender<Command> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        loop {
            // LYKIL_STUDIO_VIA=1 talks VIA even to a keyboard that speaks
            // LCP, to try the VIA path on a Lykil keyboard.
            let found = if std::env::var_os("LYKIL_STUDIO_VIA").is_some() {
                lykil_device::via::open().map(Found::Via)
            } else {
                open_any()
            };
            if let Ok(found) = found {
                let reason = match found {
                    Found::Lykil(device) => poll(device, &rx, &shared, canvas),
                    Found::Via(device) => crate::via::poll(device, &rx, &shared, canvas),
                };
                update(&shared, canvas, |k| {
                    *k = Keyboard {
                        connection: Connection::Lost(reason),
                        ..Keyboard::default()
                    };
                });
            } else {
                let seen = unanswered();
                update(&shared, canvas, |k| {
                    if k.connection == Connection::Connected {
                        k.connection = Connection::Searching;
                    }
                    k.seen = seen;
                });
            }
            while rx.try_recv().is_ok() {}
            thread::sleep(RETRY_EVERY);
        }
    });
    tx
}

/// Every raw HID interface with the VIA usage and why it did not answer.
fn unanswered() -> Vec<String> {
    lykil_device::probe()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| {
            let why = p.answer.err()?;
            Some(format!(
                "{} ({:04x}:{:04x}): {why}",
                p.name, p.vendor_id, p.product_id
            ))
        })
        .collect()
}

fn poll(
    mut device: Device,
    rx: &Receiver<Command>,
    shared: &Arc<Mutex<Shared>>,
    canvas: CanvasId,
) -> String {
    let hello = *device.hello();
    // Only a lost connection matters here; a refused query just leaves
    // the firmware unknown.
    let firmware = match device.firmware() {
        Ok(f) => f,
        Err(DeviceError::Status(_) | DeviceError::Protocol(_)) => None,
        Err(e) => return e.to_string(),
    };
    let loaded = (|| -> Result<_, DeviceError> {
        let description = device.describe()?;
        let keymap = (0..hello.layers)
            .map(|l| device.layer(l))
            .collect::<Result<Vec<_>, _>>()?;
        let lighting = if hello.capabilities & capability::LIGHTING != 0 {
            Some(device.lighting()?)
        } else {
            None
        };
        let key_colors = match lighting {
            Some(info) => device.key_colors(info.leds)?,
            None => Vec::new(),
        };
        let macros = if hello.capabilities & capability::MACROS != 0 {
            (0..MACROS)
                .map(|id| device.macro_steps(u8::try_from(id).unwrap_or(u8::MAX)))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        Ok((description, keymap, lighting, key_colors, macros))
    })();
    let (description, keymap, lighting, key_colors, macros) = match loaded {
        Ok(l) => l,
        Err(e) => return e.to_string(),
    };
    update(shared, canvas, |k| {
        *k = Keyboard {
            connection: Connection::Connected,
            // The description's name: Windows may report an interface name
            // as the product string.
            name: description.name.clone(),
            hello: Some(hello),
            firmware,
            description: Some(description),
            keymap,
            lighting,
            key_colors,
            macros,
            ..Keyboard::default()
        };
    });
    let mut next_diagnostics = Instant::now();
    loop {
        if let Err(e) = commands(&mut device, rx, shared, canvas) {
            return e;
        }
        let matrix = match device.matrix() {
            Ok(m) => m,
            Err(e) => return e.to_string(),
        };
        update(shared, canvas, |k| k.matrix = matrix);
        if Instant::now() >= next_diagnostics {
            next_diagnostics += DIAGNOSTICS_EVERY;
            match device.diagnostics() {
                Ok(d) => update(shared, canvas, |k| k.diagnostics = Some(d)),
                Err(e) => return e.to_string(),
            }
            // Lighting can change on the keyboard itself (lighting keys,
            // Windows), and the drivers can come and go.
            if hello.capabilities & capability::LIGHTING != 0 {
                match device.lighting() {
                    Ok(info) => update(shared, canvas, |k| k.lighting = Some(info)),
                    Err(e) => return e.to_string(),
                }
            }
        }
        thread::sleep(MATRIX_EVERY);
    }
}

fn commands(
    device: &mut Device,
    rx: &Receiver<Command>,
    shared: &Arc<Mutex<Shared>>,
    canvas: CanvasId,
) -> Result<(), String> {
    let mut lighting = None;
    loop {
        let command = match rx.try_recv() {
            Ok(c) => c,
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => return Err("studio closed".into()),
        };
        let result = match command {
            Command::SetLighting(s) => {
                lighting = Some(s);
                Ok(())
            }
            Command::SetBinding {
                layer,
                key,
                binding,
            } => device.set_bindings(layer, key, &[binding]).map(|()| {
                update(shared, canvas, |k| {
                    if let Some(slot) = k
                        .keymap
                        .get_mut(usize::from(layer))
                        .and_then(|l| l.get_mut(usize::from(key)))
                    {
                        *slot = binding;
                    }
                });
            }),
            Command::SetKeyColors { start, colors } => device.set_key_colors(start, &colors),
            Command::SetMacro { id, steps } => device.set_macro(id, &steps).map(|()| {
                update(shared, canvas, |k| {
                    if let Some(slot) = k.macros.get_mut(usize::from(id)) {
                        *slot = steps;
                    }
                });
            }),
            // Only VIA keyboards have these.
            Command::SetVia { .. } => Ok(()),
            Command::ResetKeymap => device.reset_keymap().and_then(|()| {
                let keymap = (0..device.hello().layers)
                    .map(|l| device.layer(l))
                    .collect::<Result<Vec<_>, _>>()?;
                update(shared, canvas, |k| k.keymap = keymap);
                Ok(())
            }),
        };
        refused(result, shared, canvas)?;
    }
    if let Some(s) = lighting {
        let result = device.set_lighting(s).and_then(|()| device.lighting());
        let result = result.map(|info| update(shared, canvas, |k| k.lighting = Some(info)));
        refused(result, shared, canvas)?;
    }
    Ok(())
}

fn refused(
    result: Result<(), DeviceError>,
    shared: &Arc<Mutex<Shared>>,
    canvas: CanvasId,
) -> Result<(), String> {
    match result {
        Ok(()) => Ok(()),
        Err(DeviceError::Status(s)) => {
            update(shared, canvas, |k| {
                k.error = Some(format!("{s:?}"));
            });
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn update(shared: &Arc<Mutex<Shared>>, canvas: CanvasId, change: impl FnOnce(&mut Keyboard)) {
    let changed = {
        let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
        let before = s.keyboard.clone();
        change(&mut s.keyboard);
        s.keyboard != before
    };
    if changed {
        request_canvas_redraw(canvas);
    }
}
