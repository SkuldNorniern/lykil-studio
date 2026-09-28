//! The keyboard, on a background thread.
//!
//! The thread finds a Lykil keyboard, reads what it is (description,
//! keymap, lighting) and then keeps the matrix and counters fresh. The UI
//! asks for changes with [`Command`]s; the thread sends them between
//! polls, and a run of lighting changes (a slider being dragged) goes out
//! as one.

use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use aurea::render::{CanvasId, request_canvas_redraw};
use lykil::binding::Binding;
use lykil::lighting::Settings;
use lykil_device::{Device, DeviceError};
use lykil_protocol::describe::Description;
use lykil_protocol::lcp::{Diagnostics, Hello, LightingInfo, capability};

use crate::app::Shared;

/// How often the matrix is read while connected.
const MATRIX_EVERY: Duration = Duration::from_millis(20);
const DIAGNOSTICS_EVERY: Duration = Duration::from_secs(1);
const RETRY_EVERY: Duration = Duration::from_secs(1);

/// What is known about the connected keyboard.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Keyboard {
    pub connection: Connection,
    pub name: String,
    pub hello: Option<Hello>,
    /// What the keyboard says about itself: keys, layers, geometry.
    pub description: Option<Description>,
    pub diagnostics: Option<Diagnostics>,
    /// Raw matrix of the last scan, per driven line.
    pub matrix: Vec<u32>,
    /// `keymap[layer][key]` as the keyboard has it.
    pub keymap: Vec<Vec<Binding>>,
    pub lighting: Option<LightingInfo>,
    /// The last change the keyboard refused, for the status line.
    pub error: Option<String>,
}

impl Keyboard {
    pub fn layer_names(&self) -> Vec<String> {
        self.description
            .as_ref()
            .map(|d| d.layers.clone())
            .unwrap_or_default()
    }

    /// Is matrix cell `(row, col)` closed?
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
    /// Lost, with the reason.
    Lost(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    SetBinding {
        layer: u8,
        key: u16,
        binding: Binding,
    },
    ResetKeymap,
    SetLighting(Settings),
}

/// Starts the device thread; commands go to the returned sender.
pub fn spawn(shared: Arc<Mutex<Shared>>, canvas: CanvasId) -> Sender<Command> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        loop {
            match Device::open() {
                Ok(device) => {
                    let reason = poll(device, &rx, &shared, canvas);
                    update(&shared, canvas, |k| {
                        *k = Keyboard {
                            connection: Connection::Lost(reason),
                            ..Keyboard::default()
                        };
                    });
                }
                Err(_) => update(&shared, canvas, |k| {
                    if k.connection == Connection::Connected {
                        k.connection = Connection::Searching;
                    }
                }),
            }
            // Commands for a keyboard that is gone are dropped.
            while rx.try_recv().is_ok() {}
            thread::sleep(RETRY_EVERY);
        }
    });
    tx
}

/// Reads the keyboard, then polls until it fails; returns why.
fn poll(
    mut device: Device,
    rx: &Receiver<Command>,
    shared: &Arc<Mutex<Shared>>,
    canvas: CanvasId,
) -> String {
    let hello = *device.hello();
    let name = device.name().to_string();
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
        Ok((description, keymap, lighting))
    })();
    let (description, keymap, lighting) = match loaded {
        Ok(l) => l,
        Err(e) => return e.to_string(),
    };
    update(shared, canvas, |k| {
        *k = Keyboard {
            connection: Connection::Connected,
            name: name.clone(),
            hello: Some(hello),
            description: Some(description),
            keymap,
            lighting,
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
        }
        thread::sleep(MATRIX_EVERY);
    }
}

/// Sends what the UI asked for. Only a connection failure ends the poll;
/// a refused change is shown and the rest go on.
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
            // Only the newest lighting settings matter.
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

/// A status from the keyboard is shown; anything else ends the poll.
fn refused(
    result: Result<(), DeviceError>,
    shared: &Arc<Mutex<Shared>>,
    canvas: CanvasId,
) -> Result<(), String> {
    match result {
        Ok(()) => Ok(()),
        Err(DeviceError::Status(s)) => {
            update(shared, canvas, |k| {
                k.error = Some(format!("the keyboard refused the change: {s:?}"));
            });
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Applies `change`; asks for a redraw only if something changed.
fn update(shared: &Arc<Mutex<Shared>>, canvas: CanvasId, change: impl FnOnce(&mut Keyboard)) {
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
