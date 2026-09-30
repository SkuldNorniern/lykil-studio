//! The device page: key test, layout, firmware and counters.

use aurea::AureaResult;
use lykil_protocol::lcp;

use crate::app::Shared;
use crate::draw::Hits;
use crate::draw::{Area, Pen, color};
use crate::format::{group, uptime};
use crate::keyboard::{self, Keys};
use crate::widgets::layout::cell;
use crate::widgets::stat::stat;

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
    let grid = Area::new(body.x, top, body.w, body.bottom() - top);
    for (i, (name, value)) in cards.iter().enumerate() {
        let a = cell(grid, (5, pen.s(10.0), pen.s(62.0)), i);
        if a.bottom() > body.bottom() {
            break;
        }
        let tone = match *name {
            "FAULTS" | "WATCHDOG RESETS" if value != "0" => color::BAD,
            _ => color::TEXT,
        };
        stat(pen, a, lang.tr(name), (value, tone))?;
    }
    let _ = &hits;
    Ok(lang
        .tr("Keys light up while pressed: a quick way to check every switch.")
        .into())
}
