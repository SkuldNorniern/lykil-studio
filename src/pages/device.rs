//! The device page: key test, layout, firmware and counters.

use aurea::AureaResult;
use lykil_protocol::lcp;

use crate::app::Shared;
use crate::draw::Hits;
use crate::draw::{Area, Pen, color};
use crate::keyboard::{self, Keys};
use crate::widgets::{group, label, uptime};

pub fn device_tab(
    pen: &mut Pen<'_>,
    body: Area,
    shared: &mut Shared,
    hits: &mut Hits,
) -> AureaResult<String> {
    let lang = pen.lang;
    let kb = &shared.keyboard;
    let kb_area = Area::new(body.x, body.y + pen.s(8.0), body.w, body.h * 0.46);
    let layers = kb.layer_names();
    let empty = Vec::new();
    let keys = Keys::Device {
        bindings: kb.keymap.first().unwrap_or(&empty),
        layers: &layers,
    };
    let used = keyboard::keyboard(pen, kb_area, kb, &keys, false, hits)?;
    let mut cards: Vec<(&str, String)> = Vec::new();
    if let Some(h) = kb.hello {
        cards.push((
            "LAYOUT",
            lang.fill(
                "{} keys, {} layers",
                &[&h.keys.to_string(), &h.layers.to_string()],
            ),
        ));
        cards.push(("MATRIX", format!("{} x {}", h.matrix_rows, h.matrix_cols)));
        cards.push(("PROTOCOL", format!("LCP {}", h.version)));
    }
    if let Some(v) = kb.via {
        cards.push(("PROTOCOL", format!("VIA {}", v.protocol)));
        let (vid, pid) = v.ids;
        cards.push(("USB ID", format!("{vid:04X}:{pid:04X}")));
    }
    match &kb.firmware {
        Some(fw) => {
            cards.push(("FIRMWARE", format!("{} {}", fw.name, fw.version)));
            cards.push(("LYKIL", fw.lykil.to_string()));
            cards.push(("CHIP ID", fw.id.clone()));
        }
        None => cards.push(("FIRMWARE", lang.tr("update it to see").to_string())),
    }
    if let Some(d) = &kb.diagnostics {
        cards.push(("UPTIME", uptime(d.uptime_ms)));
        cards.push(("SCANS", group(d.scans)));
        cards.push((
            "KEY CHANGES",
            lang.fill(
                "{} ({} raw)",
                &[&group(d.stable_transitions), &group(d.raw_transitions)],
            ),
        ));
        cards.push(("FAULTS", d.faults.to_string()));
        cards.push(("WATCHDOG RESETS", d.watchdog_resets.to_string()));
        cards.push((
            "LAST START",
            lang.tr(lcp::reset::name(d.reset_cause)).to_string(),
        ));
        cards.push((
            "STORAGE",
            lang.tr(match d.storage {
                1 => "ok",
                2 => "failed",
                _ => "none",
            })
            .to_string(),
        ));
    }
    let top = used.bottom() + pen.s(20.0);
    let cols = 5.0;
    let gap = pen.s(10.0);
    let cw = (body.w - gap * (cols - 1.0)) / cols;
    let ch = pen.s(62.0);
    for (i, (name, value)) in cards.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let (col, row) = ((i % 5) as f32, (i / 5) as f32);
        let a = Area::new(body.x + col * (cw + gap), top + row * (ch + gap), cw, ch);
        if a.bottom() > body.bottom() {
            break;
        }
        pen.round(a, pen.s(10.0), color::SURFACE)?;
        label(pen, lang.tr(name), a.x + pen.s(14.0), a.y + pen.s(12.0))?;
        let v = Area::new(
            a.x + pen.s(14.0),
            a.y + pen.s(30.0),
            a.w - pen.s(28.0),
            pen.s(20.0),
        );
        let c = match *name {
            "FAULTS" | "WATCHDOG RESETS" if value != "0" => color::BAD,
            _ => color::TEXT,
        };
        pen.fitted_left(value, v, 15.0, 9.0, c)?;
        let _ = &hits;
    }
    Ok(lang
        .tr("Keys light up while pressed: a quick way to check every switch.")
        .into())
}
