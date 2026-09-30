//! The connected device, on a background thread: what Studio knows about
//! it ([`Keyboard`]), what it asks of it ([`Command`]), and one driver per
//! protocol (`lcp` for Lykil, `via` for VIA).

pub mod lcp;
pub mod via;

use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use aurea::render::{CanvasId, request_canvas_redraw};
use lykil::binding::Binding;
use lykil::lighting::{Rgb, Settings};
use lykil::macros::Step;
use lykil_device::{FirmwareInfo, Found, open_any};
use lykil_protocol::describe::Description;
use lykil_protocol::lcp::{Diagnostics, Hello, LightingInfo};

use crate::app::Shared;

const RETRY_EVERY: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Keyboard {
    pub connection: Connection,
    pub name: String,
    pub hello: Option<Hello>,
    pub via: Option<crate::devices::via::Via>,
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
    pub via_settings: Vec<crate::devices::via::ViaSetting>,
    /// USB vendor and product id, when known.
    pub ids: Option<(u16, u16)>,
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
    /// A VIA keyboard whose definition is not in [`crate::devices::via::folder`].
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
                    Found::Lykil(device) => lcp::poll(device, &rx, &shared, canvas),
                    Found::Via(device) => crate::devices::via::poll(device, &rx, &shared, canvas),
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
