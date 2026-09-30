//! Things to click: pills, round arrows and keycap chips.

use aurea::AureaResult;
use aurea::render::Font;

use crate::app::Hit;
use crate::draw::{Area, Pen, color};

/// How a pill looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Raised, white text.
    Plain,
    /// Raised, dim text until hovered.
    Quiet,
    /// Filled with the accent: picked, or the main action.
    On,
    /// Filled red: a second click does something that cannot be undone.
    Danger,
}

/// The width a pill needs for `text` in `font`, `pad` each side.
pub fn pill_width(pen: &mut Pen<'_>, text: &str, font: &Font, pad: f32) -> f32 {
    pen.width(text, font) + pen.s(pad) * 2.0
}

/// A rounded button with `text` centred.
pub fn pill(
    pen: &mut Pen<'_>,
    a: Area,
    (text, font): (&str, &Font),
    hit: Hit,
    tone: Tone,
) -> AureaResult<()> {
    let t = pen.hover(a, hit);
    let raised = color::mix(color::RAISED, color::HOVER, t);
    let (bg, fg) = match tone {
        Tone::Plain => (raised, color::TEXT),
        Tone::Quiet => (raised, color::mix(color::DIM, color::TEXT, t)),
        Tone::On => (color::ACCENT, color::ACCENT_TEXT),
        Tone::Danger => (color::BAD, color::ACCENT_TEXT),
    };
    pen.round(a, a.h / 2.0, bg)?;
    pen.centred(text, a, font, fg)
}

/// A round button with a triangle pointing left (`dir` -1) or right (1).
pub fn arrow(pen: &mut Pen<'_>, a: Area, dir: f32, hit: Hit) -> AureaResult<()> {
    let t = pen.hover(a, hit);
    pen.round(a, a.h / 2.0, color::mix(color::RAISED, color::HOVER, t))?;
    let (cx, cy, r) = (a.x + a.w / 2.0, a.y + a.h / 2.0, pen.s(4.0));
    pen.polygon(
        &[
            (cx - dir * r * 0.6, cy - r),
            (cx + dir * r * 0.8, cy),
            (cx - dir * r * 0.6, cy + r),
        ],
        color::TEXT,
    )
}

/// A keycap to pick: `main` legend, and `sub` under it if there is one.
/// It lifts a little on hover.
pub fn keycap(
    pen: &mut Pen<'_>,
    (a, hit): (Area, Hit),
    main: &str,
    sub: Option<&str>,
    picked: bool,
) -> AureaResult<()> {
    let t = pen.hover(a, hit);
    let face = if picked {
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
    let fg = if picked {
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
                if picked { fg } else { color::ACCENT },
            )
        }
        None => pen.fitted(main, inner, 12.0, 7.0, fg),
    }
}

/// A big accent keycap showing `legend`, for the key being edited.
pub fn cap_face(pen: &mut Pen<'_>, cap: Area, legend: &str) -> AureaResult<()> {
    pen.round(
        cap,
        pen.s(8.0),
        color::mix(color::ACCENT, color::BACKGROUND, 0.5),
    )?;
    pen.round(cap.inset(pen.s(3.0)), pen.s(7.0), color::ACCENT)?;
    pen.fitted(legend, cap.inset(pen.s(8.0)), 16.0, 8.0, color::ACCENT_TEXT)
}
