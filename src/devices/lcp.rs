//! A Lykil keyboard over LCP: keymap, lighting, macros, diagnostics.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use aurea::render::CanvasId;
use lykil::macros::MACROS;
use lykil_device::{Device, DeviceError};
use lykil_protocol::lcp::capability;

use crate::app::Shared;

use super::{Command, Connection, Keyboard, update};

pub const MATRIX_EVERY: Duration = Duration::from_millis(20);

pub const DIAGNOSTICS_EVERY: Duration = Duration::from_secs(1);

pub fn poll(
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

pub fn commands(
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

pub fn refused(
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
