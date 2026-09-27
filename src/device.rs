//! The keyboard, polled from a background thread into shared state.

use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use aurea::render::{CanvasId, request_canvas_redraw};
use lykil_device::Device;
use lykil_protocol::lcp::{Diagnostics, Hello};

/// How often the matrix is read while connected.
const MATRIX_EVERY: Duration = Duration::from_millis(20);
const DIAGNOSTICS_EVERY: Duration = Duration::from_secs(1);
const RETRY_EVERY: Duration = Duration::from_secs(1);

/// What the view shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    pub connection: Connection,
    pub name: String,
    pub hello: Option<Hello>,
    pub diagnostics: Option<Diagnostics>,
    /// Raw matrix of the last scan, per driven line.
    pub matrix: Vec<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Connection {
    #[default]
    Searching,
    Connected,
    /// Lost, with the reason.
    Lost(String),
}

/// Starts the polling thread. It never ends; the process does.
pub fn spawn(state: Arc<Mutex<State>>, canvas: CanvasId) {
    thread::spawn(move || {
        loop {
            match Device::open() {
                Ok(device) => {
                    let reason = poll(device, &state, canvas);
                    update(&state, canvas, |s| {
                        *s = State {
                            connection: Connection::Lost(reason),
                            ..State::default()
                        };
                    });
                }
                Err(_) => update(&state, canvas, |s| {
                    if s.connection == Connection::Connected {
                        s.connection = Connection::Searching;
                    }
                }),
            }
            thread::sleep(RETRY_EVERY);
        }
    });
}

/// Polls until the device fails; returns why.
fn poll(mut device: Device, state: &Arc<Mutex<State>>, canvas: CanvasId) -> String {
    let hello = *device.hello();
    let name = device.name().to_string();
    update(state, canvas, |s| {
        s.connection = Connection::Connected;
        s.name.clone_from(&name);
        s.hello = Some(hello);
    });
    let mut next_diagnostics = Instant::now();
    loop {
        let matrix = match device.matrix() {
            Ok(m) => m,
            Err(e) => return e.to_string(),
        };
        update(state, canvas, |s| s.matrix = matrix);
        if Instant::now() >= next_diagnostics {
            next_diagnostics += DIAGNOSTICS_EVERY;
            match device.diagnostics() {
                Ok(d) => update(state, canvas, |s| s.diagnostics = Some(d)),
                Err(e) => return e.to_string(),
            }
        }
        thread::sleep(MATRIX_EVERY);
    }
}

/// Applies `change`; asks for a redraw only if something changed.
fn update(state: &Arc<Mutex<State>>, canvas: CanvasId, change: impl FnOnce(&mut State)) {
    let changed = {
        let mut s = state.lock().unwrap_or_else(PoisonError::into_inner);
        let before = s.clone();
        change(&mut s);
        *s != before
    };
    if changed {
        request_canvas_redraw(canvas);
    }
}
