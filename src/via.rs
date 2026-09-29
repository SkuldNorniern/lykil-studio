//! Keyboards that speak only VIA.
//!
//! Such a keyboard does not describe itself, so Studio needs its VIA
//! definition (the JSON VIA's Design tab loads), looked up by USB ids in
//! [`folder`]. The keymap is read and written in QMK keycodes, which
//! `lykil-qmk` turns into Lykil bindings and back; a binding VIA cannot
//! hold is refused. Lighting and macros stay Lykil only.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use aurea::render::CanvasId;
use lykil::binding::Binding;
use lykil_device::{DeviceError, ViaDevice};
use lykil_protocol::describe::{Description, Key};
use lykil_qmk::import::{ViaDefinition, via_definition};
use lykil_qmk::{AbiVersion, decode, encode, instantiate, project};

use crate::app::Shared;
use crate::device::{Command, Connection, Keyboard, update};

/// How often a keyboard without a definition is looked for again.
const RESCAN: Duration = Duration::from_secs(1);

/// A VIA keyboard as Studio shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Via {
    pub protocol: u16,
    /// USB vendor and product id.
    pub ids: (u16, u16),
}

/// Where VIA definitions go: `%APPDATA%\Lykil Studio\via`.
pub fn folder() -> PathBuf {
    let base = std::env::var_os("APPDATA").map_or_else(|| PathBuf::from("."), PathBuf::from);
    base.join("Lykil Studio").join("via")
}

/// The definition in `folder` for USB ids `ids`.
fn find(ids: (u16, u16)) -> Option<ViaDefinition> {
    let dir = folder();
    let _ = std::fs::create_dir_all(&dir);
    std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("json"))
        })
        .filter_map(|e| via_definition(&std::fs::read_to_string(e.path()).ok()?).ok())
        .find(|d| (d.vendor_id, d.product_id) == ids)
}

/// The QMK keycode set the keyboard speaks; the newest when it does not
/// say.
fn abi(bcd: Option<u32>) -> AbiVersion {
    let numbers = |v: AbiVersion| {
        let [major, minor, patch] = v.numbers();
        lykil_protocol::via::keycodes_version_bcd(major, minor, patch)
    };
    bcd.and_then(|b| AbiVersion::ALL.into_iter().find(|v| numbers(*v) == b))
        .unwrap_or(AbiVersion::LATEST)
}

/// The keyboard as Studio draws it, from its definition.
fn description(def: &ViaDefinition, layers: u8) -> Description {
    Description {
        name: def.name.clone(),
        columns_driven: false,
        layers: (0..layers).map(|l| format!("Layer {l}")).collect(),
        keys: def
            .keys
            .iter()
            .map(|k| Key {
                id: format!("{},{}", k.row, k.col),
                geometry: Some([k.x, k.y, k.w, k.h]),
                cell: Some((k.row, k.col)),
                led: None,
            })
            .collect(),
    }
}

/// Keycodes (`[layer][row * cols + col]`) as bindings by description
/// key. A keycode Lykil has no form for reads as no binding.
fn bindings(codes: &[Vec<u16>], def: &ViaDefinition, abi: AbiVersion) -> Vec<Vec<Binding>> {
    codes
        .iter()
        .map(|layer| {
            def.keys
                .iter()
                .map(|k| {
                    let at = usize::from(k.row) * usize::from(def.cols) + usize::from(k.col);
                    layer
                        .get(at)
                        .and_then(|code| decode(abi, *code).ok())
                        .and_then(|q| instantiate(&q).value())
                        .unwrap_or(Binding::None)
                })
                .collect()
        })
        .collect()
}

/// Talks to a VIA keyboard until it goes; returns why.
pub fn poll(
    mut device: ViaDevice,
    rx: &Receiver<Command>,
    shared: &Arc<Mutex<Shared>>,
    canvas: CanvasId,
) -> String {
    let ids = device.ids();
    let via = Some(Via {
        protocol: device.protocol(),
        ids,
    });
    let def = loop {
        if let Some(def) = find(ids) {
            break def;
        }
        update(shared, canvas, |k| {
            *k = Keyboard {
                connection: Connection::NeedsDefinition,
                name: device.name().to_string(),
                via,
                ..Keyboard::default()
            };
        });
        thread::sleep(RESCAN);
        // Gone meanwhile?
        if let Err(e) = device.uptime_ms() {
            return e.to_string();
        }
        while rx.try_recv().is_ok() {}
    };
    let loaded = (|| -> Result<_, DeviceError> {
        let layers = device.layer_count()?;
        let abi = abi(device.keycodes_version()?);
        let codes = device.keymap(layers, def.rows, def.cols)?;
        Ok((layers, abi, codes))
    })();
    let (layers, abi, codes) = match loaded {
        Ok(l) => l,
        Err(e) => return e.to_string(),
    };
    update(shared, canvas, |k| {
        *k = Keyboard {
            connection: Connection::Connected,
            name: def.name.clone(),
            description: Some(description(&def, layers)),
            keymap: bindings(&codes, &def, abi),
            via,
            ..Keyboard::default()
        };
    });
    loop {
        let command = match rx.recv_timeout(RESCAN) {
            Ok(c) => c,
            Err(RecvTimeoutError::Timeout) => {
                // Still there?
                if let Err(e) = device.uptime_ms() {
                    return e.to_string();
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => {
                return "studio closed".into();
            }
        };
        let result = match command {
            Command::SetBinding {
                layer,
                key,
                binding,
            } => set(&mut device, &def, abi, (layer, key, binding)),
            Command::ResetKeymap => device.reset_keymap().map(|()| None),
            _ => Ok(Some(
                "VIA keyboards have no lighting or macros in Studio".to_string(),
            )),
        };
        let refused = match result {
            Ok(r) => r,
            Err(DeviceError::Protocol(what)) => Some(what.to_string()),
            Err(e) => return e.to_string(),
        };
        // Read back what the keyboard has now, refused or not.
        let codes = match device.keymap(layers, def.rows, def.cols) {
            Ok(c) => c,
            Err(e) => return e.to_string(),
        };
        update(shared, canvas, |k| {
            k.keymap = bindings(&codes, &def, abi);
            k.error = refused;
        });
    }
}

/// Writes one binding; `Ok(Some(why))` if VIA cannot hold it.
fn set(
    device: &mut ViaDevice,
    def: &ViaDefinition,
    abi: AbiVersion,
    (layer, key, binding): (u8, u16, Binding),
) -> Result<Option<String>, DeviceError> {
    let Some(k) = def.keys.get(usize::from(key)) else {
        return Ok(Some("no such key".into()));
    };
    let Some(code) = project(abi, binding)
        .value()
        .and_then(|q| encode(abi, &q).ok())
    else {
        return Ok(Some("VIA has no keycode for this binding".into()));
    };
    device.set_keycode(layer, k.row, k.col, code).map(|()| None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keycodes_become_bindings_by_key() {
        let def = via_definition(
            r#"{"name":"T","vendorId":"0x1","productId":"0x2","matrix":{"rows":1,"cols":2},
                "layouts":{"keymap":[["0,1","0,0"]]}}"#,
        )
        .unwrap();
        let latest = AbiVersion::LATEST;
        // KC_A (4) at 0,0 and KC_TRNS (1) at 0,1; keys are listed 0,1 first.
        let b = bindings(&[vec![4, 1]], &def, latest);
        assert_eq!(b[0][0], Binding::Transparent);
        assert!(matches!(b[0][1], Binding::Key(_)));
        let d = description(&def, 2);
        assert_eq!(d.layers, ["Layer 0", "Layer 1"]);
        assert_eq!(d.keys[0].cell, Some((0, 1)));
        assert_eq!(abi(None), AbiVersion::LATEST);
        assert_eq!(abi(Some(0x0007)), AbiVersion::V0_0_7);
    }
}
