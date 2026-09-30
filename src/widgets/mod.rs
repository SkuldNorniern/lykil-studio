//! Controls more than one page draws: labels, pills, chips, sliders,
//! segmented bars and text fields.

pub mod colour;

use aurea::AureaResult;
use aurea::render::Color;
use lykil::lighting::Hsv;

use crate::anim::{Key, rate};
use crate::app::{Hit, Shared};
use crate::draw::{Area, Hits, Pen, color};

pub fn label(pen: &mut Pen<'_>, text: &str, x: f32, y: f32) -> AureaResult<()> {
    pen.text(text, x, y, &pen.bold(11.0), color::FAINT)
}

/// Small chips left to right, wrapping inside `area`, the active ones
/// filled. Returns the height they took.
pub fn chip_flow(
    pen: &mut Pen<'_>,
    area: Area,
    items: &[(String, Hit, bool)],
    hits: &mut Hits,
) -> AureaResult<f32> {
    let font = pen.font(11.0);
    let (chip_h, pad, gap) = (pen.s(24.0), pen.s(10.0), pen.s(6.0));
    let (mut left, mut row) = (area.x, area.y);
    for (name, hit, on) in items {
        let chip_w = pen.width(name, &font) + pad * 2.0;
        if left + chip_w > area.right() && left > area.x {
            left = area.x;
            row += chip_h + gap;
        }
        let chip = Area::new(left, row, chip_w.min(area.w), chip_h);
        let t = pen.hover(chip, *hit);
        let bg = if *on {
            color::ACCENT
        } else {
            color::mix(color::RAISED, color::BORDER, t)
        };
        let fg = if *on { color::ACCENT_TEXT } else { color::TEXT };
        pen.round(chip, chip_h / 2.0, bg)?;
        pen.centred(name, chip, &font, fg)?;
        hits.push((chip, *hit));
        left += chip_w + gap;
    }
    Ok(row + chip_h - area.y)
}

pub fn pills(
    pen: &mut Pen<'_>,
    x: f32,
    y: f32,
    items: &[(String, Hit, bool)],
    hits: &mut Hits,
) -> AureaResult<f32> {
    let font = pen.bold(12.0);
    let mut x = x;
    for (label, hit, active) in items {
        let w = pen.width(label, &font) + pen.s(24.0);
        let a = Area::new(x, y, w, pen.s(30.0));
        let hover = pen.hover(a, *hit);
        let bg = if *active {
            color::ACCENT
        } else {
            color::mix(color::RAISED, color::HOVER, hover)
        };
        pen.round(a, pen.s(15.0), bg)?;
        let fg = if *active {
            color::ACCENT_TEXT
        } else {
            color::TEXT
        };
        pen.centred(label, a, &font, fg)?;
        hits.push((a, *hit));
        x += w + pen.s(8.0);
    }
    Ok(x)
}

pub fn chip(
    pen: &mut Pen<'_>,
    (a, hit): (Area, Hit),
    main: &str,
    sub: Option<&str>,
    selected: bool,
) -> AureaResult<()> {
    let t = pen.hover(a, hit);
    let face = if selected {
        color::ACCENT
    } else {
        color::mix(color::RAISED, color::HOVER, t)
    };
    let a = Area::new(a.x, a.y - pen.s(1.5) * t, a.w, a.h);
    pen.round(a, pen.s(7.0), color::mix(face, color::BACKGROUND, 0.45))?;
    let top = Area::new(
        a.x + pen.s(2.0),
        a.y + pen.s(1.0),
        a.w - pen.s(4.0),
        a.h - pen.s(5.0),
    );
    pen.round(top, pen.s(6.0), face)?;
    let fg = if selected {
        color::ACCENT_TEXT
    } else {
        color::TEXT
    };
    let inner = top.inset(pen.s(3.0));
    match sub {
        Some(sub) => {
            let upper = Area::new(inner.x, inner.y, inner.w, inner.h * 0.62);
            let lower = Area::new(inner.x, inner.y + inner.h * 0.58, inner.w, inner.h * 0.38);
            pen.fitted(main, upper, 12.0, 7.0, fg)?;
            pen.fitted(
                sub,
                lower,
                9.0,
                6.0,
                if selected { fg } else { color::ACCENT },
            )
        }
        None => pen.fitted(main, inner, 12.0, 7.0, fg),
    }
}

/// `text` split at its line breaks and wherever a line would run past
/// `room`.
pub fn wrap(pen: &mut Pen<'_>, text: &str, font: &aurea::render::Font, room: f32) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.split('\n') {
        let mut current = String::new();
        for ch in line.chars() {
            current.push(ch);
            if pen.width(&current, font) > room && current.chars().count() > 1 {
                current.pop();
                out.push(std::mem::take(&mut current));
                current.push(ch);
            }
        }
        out.push(current);
    }
    out
}

pub fn text_field(pen: &mut Pen<'_>, field: Area, text: &str, editing: bool) -> AureaResult<()> {
    let lang = pen.lang;
    let font = pen.font(15.0);
    let mut ty = field.y + pen.s(12.0);
    let shown = if editing {
        format!("{text}|")
    } else {
        text.to_string()
    };
    let room = field.w - pen.s(24.0);
    for line in wrap(pen, &shown, &font, room) {
        if ty > field.bottom() - pen.s(20.0) {
            break;
        }
        pen.text(&line, field.x + pen.s(12.0), ty, &font, color::TEXT)?;
        ty += pen.s(22.0);
    }
    if text.is_empty() && !editing {
        pen.text(
            lang.tr("Start typing: this macro will type the same text."),
            field.x + pen.s(12.0),
            field.y + pen.s(12.0),
            &font,
            color::FAINT,
        )?;
    }
    Ok(())
}

/// Options side by side in one bar; the chosen one's background slides.
/// `id` tells the bars apart for the slide.
pub fn segmented(
    pen: &mut Pen<'_>,
    a: Area,
    (items, id): (&[(&str, Hit)], usize),
    chosen: usize,
    hits: &mut Hits,
) -> AureaResult<()> {
    pen.round(a, pen.s(8.0), color::BACKGROUND)?;
    #[allow(clippy::cast_precision_loss)]
    let w = (a.w - pen.s(4.0)) / items.len() as f32;
    #[allow(clippy::cast_precision_loss)]
    let at = pen
        .anim
        .to(Key::Selected(2000 + id), chosen as f32, rate::SLIDE);
    let knob = Area::new(
        a.x + pen.s(2.0) + w * at,
        a.y + pen.s(2.0),
        w,
        a.h - pen.s(4.0),
    );
    pen.round(knob, pen.s(6.0), color::ACCENT)?;
    let font = pen.bold(11.0);
    for (i, (text, hit)) in items.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let cell = Area::new(a.x + pen.s(2.0) + w * i as f32, a.y, w, a.h);
        #[allow(clippy::cast_precision_loss)]
        let near = 1.0 - (at - i as f32).abs().min(1.0);
        let t = pen.hover(cell, *hit);
        let fg = color::mix(
            color::mix(color::DIM, color::TEXT, t),
            color::ACCENT_TEXT,
            near,
        );
        let inner = cell.inset(pen.s(6.0));
        let mut f = font.clone();
        while pen.width(text, &f) > inner.w && f.size > pen.s(7.0) {
            f.size -= pen.s(0.5);
        }
        pen.centred(text, cell, &f, fg)?;
        hits.push((cell, *hit));
    }
    Ok(())
}

pub fn ring(pen: &mut Pen<'_>, x: f32, y: f32, r: f32, c: Color) -> AureaResult<()> {
    pen.circle(x, y, r + pen.s(1.0), Color::rgb(0, 0, 0))?;
    pen.circle(x, y, r, color::TEXT)?;
    pen.circle(x, y, r - pen.s(2.5), c)
}

/// A track with a knob at `t` (`0..=1`) and the percentage on the right.
pub fn slider(
    pen: &mut Pen<'_>,
    area: Area,
    name: &str,
    t: f32,
    hit: Hit,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    label(pen, name, area.x, area.y)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pct = format!("{}%", (t * 100.0).round() as u32);
    let font = pen.font(11.0);
    let vw = pen.width(&pct, &font);
    pen.text(&pct, area.right() - vw, area.y, &font, color::DIM)?;
    let track = Area::new(area.x, area.y + pen.s(20.0), area.w, pen.s(10.0));
    pen.round(track, track.h / 2.0, color::RAISED)?;
    let filled = Area::new(track.x, track.y, track.w * t, track.h);
    pen.round(
        filled,
        track.h / 2.0,
        colour::rgb(Hsv::new(170, 120, 200).to_rgb()),
    )?;
    let key = Key::HitKnob(hit);
    if shared.ui.dragging(hit) {
        pen.anim.set(key, t);
    }
    let v = pen.anim.to(key, t, rate::KNOB);
    let grab = Area::new(
        track.x - pen.s(8.0),
        track.y - pen.s(10.0),
        track.w + pen.s(16.0),
        track.h + pen.s(20.0),
    );
    let hover = pen.hover(grab, hit);
    let knob = track.x + track.w * v;
    let r = pen.s(8.0) + pen.s(2.0) * hover;
    pen.circle(knob, track.y + track.h / 2.0, r, color::TEXT)?;
    pen.circle(
        knob,
        track.y + track.h / 2.0,
        r - pen.s(3.0),
        color::BACKGROUND,
    )?;
    hits.push((grab, hit));
    Ok(())
}

pub fn uptime(ms: u32) -> String {
    let s = ms / 1000;
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, s / 60 % 60)
    } else {
        format!("{}m {:02}s", s / 60, s % 60)
    }
}

pub fn group(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_times() {
        assert_eq!(group(1_234_567), "1,234,567");
        assert_eq!(group(12), "12");
        assert_eq!(uptime(61_000), "1m 01s");
        assert_eq!(uptime(3_725_000), "1h 02m");
    }
}
