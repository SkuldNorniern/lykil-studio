//! The lighting page for a keyboard that speaks only VIA: the settings
//! its definition's menus offer, one card per menu section.

use aurea::AureaResult;
use lykil_qmk::import::ViaControlKind;

use crate::app::{Hit, Shared};
use crate::draw::Hits;
use crate::draw::{Area, Pen, color};
use crate::via::ViaSetting;
use crate::widgets::colour::hue_bar;
use crate::widgets::{self, segmented, slider};

pub fn tab(pen: &mut Pen<'_>, body: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<String> {
    let lang = pen.lang;
    let settings = &shared.keyboard.via_settings;
    if settings.is_empty() {
        pen.centred(
            lang.tr("This keyboard's VIA definition has no lighting menu"),
            body,
            &pen.font(15.0),
            color::DIM,
        )?;
        return Ok(String::new());
    }
    let mut groups: Vec<&str> = Vec::new();
    for s in settings {
        if !groups.contains(&s.control.group.as_str()) {
            groups.push(&s.control.group);
        }
    }
    let gap = pen.s(16.0);
    let columns = groups.len().clamp(1, 3);
    #[allow(clippy::cast_precision_loss)]
    let w = (body.w - gap * (columns - 1) as f32) / columns as f32;
    let top = body.y + pen.s(8.0);
    for (i, group) in groups.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let card = Area::new(
            body.x + (w + gap) * (i % columns) as f32,
            top,
            w,
            body.bottom() - top,
        );
        pen.round(card, pen.s(12.0), color::SURFACE)?;
        let inner = card.inset(pen.s(16.0));
        let title = if group.is_empty() {
            lang.tr("Lighting")
        } else {
            group
        };
        pen.text(title, inner.x, inner.y, &pen.bold(15.0), color::TEXT)?;
        let mut y = inner.y + pen.s(34.0);
        for (index, s) in settings.iter().enumerate() {
            if s.control.group != *group {
                continue;
            }
            let area = Area::new(inner.x, y, inner.w, inner.bottom() - y);
            y += control(pen, area, (index, s), shared, hits)? + pen.s(18.0);
        }
    }
    Ok(lang
        .tr("VIA settings from the keyboard's definition. Saved on the keyboard when you let go.")
        .into())
}

/// Draws one setting at the top of `area`; returns the height it took.
fn control(
    pen: &mut Pen<'_>,
    area: Area,
    (index, s): (usize, &ViaSetting),
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<f32> {
    let label = s.control.label.to_uppercase();
    match &s.control.kind {
        ViaControlKind::Range { min, max } => {
            let span = f32::from(max.saturating_sub(*min)).max(1.0);
            let t = f32::from(s.byte().saturating_sub(*min)) / span;
            slider(pen, area, &label, t, Hit::ViaRange(index), shared, hits)?;
            Ok(pen.s(34.0))
        }
        ViaControlKind::Toggle => {
            widgets::label(pen, &label, area.x, area.y)?;
            let bar = Area::new(
                area.x,
                area.y + pen.s(18.0),
                area.w.min(pen.s(220.0)),
                pen.s(30.0),
            );
            let lang = pen.lang;
            let items = [
                (lang.tr("Off"), Hit::ViaToggle(index, false)),
                (lang.tr("On"), Hit::ViaToggle(index, true)),
            ];
            segmented(
                pen,
                bar,
                (&items, 3000 + index),
                usize::from(s.byte() != 0),
                hits,
            )?;
            Ok(pen.s(48.0))
        }
        ViaControlKind::Color => {
            widgets::label(pen, &label, area.x, area.y)?;
            let (h, sat) = (s.byte(), s.value.get(1).copied().unwrap_or(255));
            let bar = Area::new(area.x, area.y + pen.s(22.0), area.w, pen.s(14.0));
            hue_bar(pen, bar, h, Hit::ViaHue(index), hits)?;
            let row = Area::new(area.x, bar.bottom() + pen.s(14.0), area.w, area.h);
            let t = f32::from(sat) / 255.0;
            slider(pen, row, "SATURATION", t, Hit::ViaSat(index), shared, hits)?;
            Ok(pen.s(84.0))
        }
        ViaControlKind::Dropdown(options) => {
            widgets::label(pen, &label, area.x, area.y)?;
            let chosen = s.byte();
            let font = pen.font(11.0);
            let (chip_h, pad, gap) = (pen.s(24.0), pen.s(10.0), pen.s(6.0));
            let (mut left, mut row) = (area.x, area.y + pen.s(20.0));
            for (name, value) in options {
                let chip_w = pen.width(name, &font) + pad * 2.0;
                if left + chip_w > area.right() && left > area.x {
                    left = area.x;
                    row += chip_h + gap;
                }
                let chip = Area::new(left, row, chip_w.min(area.w), chip_h);
                let hit = Hit::ViaOption(index, *value);
                let t = pen.hover(chip, hit);
                let on = *value == chosen;
                let bg = if on {
                    color::ACCENT
                } else {
                    color::mix(color::RAISED, color::BORDER, t)
                };
                let fg = if on { color::ACCENT_TEXT } else { color::TEXT };
                pen.round(chip, chip_h / 2.0, bg)?;
                pen.centred(name, chip, &font, fg)?;
                hits.push((chip, hit));
                left += chip_w + gap;
            }
            Ok(row + chip_h - area.y)
        }
    }
}
