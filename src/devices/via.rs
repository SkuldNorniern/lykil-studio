//! Keyboards that speak only VIA, laid out from their VIA definition.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use aurea::render::CanvasId;
use lykil::binding::Binding;
use lykil::macros::Step;
use lykil_device::{DeviceError, ViaDevice};
use lykil_protocol::describe::{Description, Key};
use lykil_qmk::import::{ViaControl, ViaControlKind, ViaDefinition, via_definition};
use lykil_qmk::send_string::{self, Item};
use lykil_qmk::{AbiVersion, decode, encode, instantiate, project};

use crate::app::Shared;
use crate::devices::{Command, Connection, Keyboard, update};

const RESCAN: Duration = Duration::from_secs(1);
/// A changed channel is saved once it has been left alone this long:
/// saving writes EEPROM.
const SAVE_AFTER: Duration = Duration::from_millis(600);

/// One setting of a VIA keyboard and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViaSetting {
    pub control: ViaControl,
    pub value: Vec<u8>,
}

impl ViaSetting {
    pub fn byte(&self) -> u8 {
        self.value.first().copied().unwrap_or(0)
    }
}

const fn value_len(kind: &ViaControlKind) -> usize {
    match kind {
        ViaControlKind::Color => 2,
        _ => 1,
    }
}

/// The definition's settings the keyboard answers, with their values.
fn read_settings(device: &mut ViaDevice, def: &ViaDefinition) -> Vec<ViaSetting> {
    def.controls
        .iter()
        .filter_map(|c| {
            let value = device
                .custom_value(c.channel, c.value, value_len(&c.kind))
                .ok()?;
            Some(ViaSetting {
                control: c.clone(),
                value,
            })
        })
        .collect()
}

/// Each macro's steps. Text becomes the steps that type it; a macro the
/// buffer holds badly reads as empty.
fn read_macros(buffer: &[u8], count: u8) -> Vec<Vec<Step>> {
    let mut out = Vec::new();
    let mut rest = buffer;
    for _ in 0..count {
        let mut steps = Vec::new();
        let mut text = String::new();
        let mut ok = true;
        for item in send_string::decode(rest) {
            match item {
                Ok(Item::Char(c)) => text.push(char::from(c)),
                Ok(Item::Step(s)) => {
                    flush(&mut text, &mut steps, &mut ok);
                    steps.push(s);
                }
                Err(_) => ok = false,
            }
        }
        flush(&mut text, &mut steps, &mut ok);
        out.push(if ok { steps } else { Vec::new() });
        let end = rest
            .iter()
            .position(|b| *b == 0)
            .map_or(rest.len(), |i| i + 1);
        rest = &rest[end..];
    }
    out
}

fn flush(text: &mut String, steps: &mut Vec<Step>, ok: &mut bool) {
    if text.is_empty() {
        return;
    }
    match lykil_config::text::steps(text) {
        Ok(s) => steps.extend(s),
        Err(_) => *ok = false,
    }
    text.clear();
}

/// Changed lighting channels and when, each saved once left alone.
#[derive(Default)]
struct Unsaved(BTreeMap<u8, Instant>);

impl Unsaved {
    fn changed(&mut self, channel: u8) {
        self.0.insert(channel, Instant::now());
    }

    fn wait(&self) -> Duration {
        if self.0.is_empty() {
            RESCAN
        } else {
            SAVE_AFTER / 3
        }
    }

    fn save_due(&mut self, device: &mut ViaDevice) -> Result<(), DeviceError> {
        let due: Vec<u8> = self
            .0
            .iter()
            .filter(|(_, at)| at.elapsed() >= SAVE_AFTER)
            .map(|(ch, _)| *ch)
            .collect();
        for ch in due {
            self.0.remove(&ch);
            device.save_custom(ch)?;
        }
        Ok(())
    }
}

/// Puts `steps` in macro `id` and writes the whole buffer.
fn set_macro(
    device: &mut ViaDevice,
    macros: &mut [Vec<Step>],
    id: u8,
    steps: Vec<Step>,
) -> Result<Option<String>, DeviceError> {
    let Some(slot) = macros.get_mut(usize::from(id)) else {
        return Ok(Some("no such macro".into()));
    };
    let before = std::mem::replace(slot, steps);
    let written = match write_macros(macros) {
        Some(bytes) => device.set_macro_buffer(&bytes).map(|()| None),
        None => Ok(Some("the macro does not fit".to_string())),
    };
    if !matches!(written, Ok(None)) {
        macros[usize::from(id)] = before;
    }
    written
}

/// The macro buffer for `macros`, each ended by `0`.
fn write_macros(macros: &[Vec<Step>]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    for steps in macros {
        let mut bytes = vec![0; steps.len() * 8 + 1];
        let n = send_string::encode(steps, &mut bytes)?;
        out.extend_from_slice(&bytes[..n]);
        out.push(0);
    }
    Some(out)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Via {
    pub protocol: u16,
    pub ids: (u16, u16),
    /// The QMK keycode set it speaks.
    pub abi: AbiVersion,
}

impl Via {
    /// Whether a VIA keycode can hold `binding`.
    pub fn holds(self, binding: Binding) -> bool {
        project(self.abi, binding)
            .value()
            .and_then(|q| encode(self.abi, &q).ok())
            .is_some()
    }
}

/// Where VIA definitions go: `via` in Studio's data folder.
pub fn folder() -> PathBuf {
    crate::data_dir().join("via")
}

fn find(ids: (u16, u16), device: &str) -> Option<ViaDefinition> {
    let dir = folder();
    let _ = std::fs::create_dir_all(&dir);
    let defs: Vec<ViaDefinition> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("json"))
        })
        .filter_map(|e| via_definition(&std::fs::read_to_string(e.path()).ok()?).ok())
        .collect();
    pick(defs, ids, device)
}

fn pick(defs: Vec<ViaDefinition>, ids: (u16, u16), device: &str) -> Option<ViaDefinition> {
    if let Some(i) = defs.iter().position(|d| (d.vendor_id, d.product_id) == ids) {
        return defs.into_iter().nth(i);
    }
    let device = device.to_lowercase();
    defs.into_iter()
        .filter(|d| d.vendor_id == ids.0 && names(&d.name, &device))
        .max_by_key(|d| d.name.len())
}

fn names(name: &str, device: &str) -> bool {
    let name = name.trim().to_lowercase();
    !name.is_empty()
        && device
            .strip_prefix(&name)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(|c: char| !c.is_alphanumeric()))
}

fn abi(bcd: Option<u32>) -> AbiVersion {
    let numbers = |v: AbiVersion| {
        let [major, minor, patch] = v.numbers();
        lykil_protocol::via::keycodes_version_bcd(major, minor, patch)
    };
    bcd.and_then(|b| AbiVersion::ALL.into_iter().find(|v| numbers(*v) == b))
        .unwrap_or(AbiVersion::LATEST)
}

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

/// The keyboard's definition, once one is in the folder.
fn definition(
    device: &mut ViaDevice,
    via: Option<Via>,
    rx: &Receiver<Command>,
    shared: &Arc<Mutex<Shared>>,
    canvas: CanvasId,
) -> Result<ViaDefinition, DeviceError> {
    loop {
        if let Some(def) = find(device.ids(), device.name()) {
            return Ok(def);
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
        device.uptime_ms()?;
        while rx.try_recv().is_ok() {}
    }
}

pub fn poll(
    mut device: ViaDevice,
    rx: &Receiver<Command>,
    shared: &Arc<Mutex<Shared>>,
    canvas: CanvasId,
) -> String {
    let ids = device.ids();
    let mut via = Some(Via {
        protocol: device.protocol(),
        ids,
        abi: AbiVersion::LATEST,
    });
    let def = match definition(&mut device, via, rx, shared, canvas) {
        Ok(d) => d,
        Err(e) => return e.to_string(),
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
    if let Some(v) = via.as_mut() {
        v.abi = abi;
    }
    // Keyboards without dynamic macros refuse these; that is no macros.
    let mut macros = device
        .macro_count()
        .and_then(|n| Ok(read_macros(&device.macro_buffer()?, n)))
        .unwrap_or_default();
    let settings = read_settings(&mut device, &def);
    update(shared, canvas, |k| {
        *k = Keyboard {
            connection: Connection::Connected,
            name: def.name.clone(),
            description: Some(description(&def, layers)),
            keymap: bindings(&codes, &def, abi),
            macros: macros.clone(),
            via_settings: settings.clone(),
            via,
            ..Keyboard::default()
        };
    });
    let mut unsaved = Unsaved::default();
    loop {
        let command = match rx.recv_timeout(unsaved.wait()) {
            Ok(c) => c,
            Err(RecvTimeoutError::Timeout) => {
                if let Err(e) = unsaved
                    .save_due(&mut device)
                    .and_then(|()| device.uptime_ms())
                {
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
            Command::SetVia { setting, value } => {
                let Some(c) = settings.get(setting).map(|s| &s.control) else {
                    continue;
                };
                match device.set_custom_value(c.channel, c.value, &value) {
                    Ok(()) => {
                        unsaved.changed(c.channel);
                        continue;
                    }
                    Err(e) => Err(e),
                }
            }
            Command::SetMacro { id, steps } => {
                let r = set_macro(&mut device, &mut macros, id, steps);
                let m = macros.clone();
                update(shared, canvas, |k| k.macros = m);
                r
            }
            _ => Ok(Some(
                "Lykil only: a VIA keyboard cannot do this".to_string(),
            )),
        };
        let refused = match result {
            Ok(r) => r,
            Err(DeviceError::Protocol(what)) => Some(what.to_string()),
            Err(e) => return e.to_string(),
        };
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
    fn macros_read_and_write() {
        let hi = lykil_config::text::steps("Hi").unwrap();
        let bytes = write_macros(&[hi.clone(), vec![Step::Delay(50)], Vec::new()]).unwrap();
        let back = read_macros(&bytes, 4);
        assert_eq!(back[0], hi);
        assert_eq!(back[1], [Step::Delay(50)]);
        assert!(back[2].is_empty() && back[3].is_empty());
        // Text as VIA's own editor writes it.
        let typed = read_macros(b"ok\0", 1);
        assert_eq!(typed[0], lykil_config::text::steps("ok").unwrap());
        assert!(read_macros(b"\x01\x09\0", 1)[0].is_empty());
    }

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

    #[test]
    fn a_dongle_takes_its_keyboards_definition_by_name() {
        let def = |name: &str, product: &str| {
            via_definition(&format!(
                r#"{{"name":"{name}","vendorId":"0x36B0","productId":"{product}",
                    "matrix":{{"rows":1,"cols":1}},"layouts":{{"keymap":[["0,0"]]}}}}"#
            ))
            .unwrap()
        };
        let defs = || vec![def("EVO", "0x3001"), def("EVO80", "0x300E")];
        let name = |d: Option<ViaDefinition>| d.map(|d| d.name);
        assert_eq!(
            name(pick(defs(), (0x36b0, 0x300e), "x")),
            Some("EVO80".into())
        );
        assert_eq!(
            name(pick(defs(), (0x36b0, 0x3002), "EVO80 2.4G")),
            Some("EVO80".into())
        );
        assert_eq!(name(pick(defs(), (0x36b0, 0x3002), "EVO800")), None);
        assert_eq!(name(pick(defs(), (0x1234, 0x3002), "EVO80 2.4G")), None);
    }
}
