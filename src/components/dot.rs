//! Round marks: status dots, the rings on colour controls and colour
//! swatches.

use aurea::AureaResult;
use aurea::render::Color;
use lykil::lighting::Rgb;

use crate::app::Hit;
use crate::colour::rgb;
use crate::draw::{Area, Pen, color};

/// A small dot in front of a line, its colour telling the state.
pub fn status(pen: &mut Pen<'_>, x: f32, cy: f32, c: Color) -> AureaResult<()> {
    pen.circle(x + pen.s(4.0), cy, pen.s(4.0), c)
}

/// The ring marking a spot on a colour control, filled with `c`.
pub fn ring(pen: &mut Pen<'_>, x: f32, y: f32, r: f32, c: Color) -> AureaResult<()> {
    pen.circle(x, y, r + pen.s(1.0), Color::rgb(0, 0, 0))?;
    pen.circle(x, y, r, color::TEXT)?;
    pen.circle(x, y, r - pen.s(2.5), c)
}

/// A colour to click, circled when it is `now`.
pub fn swatch(
    pen: &mut Pen<'_>,
    a: Area,
    c: Rgb,
    now: Rgb,
    hits: &mut crate::draw::Hits,
) -> AureaResult<()> {
    let hit = Hit::Swatch(c);
    let t = pen.hover(a, hit);
    let r = a.w / 2.0 + pen.s(2.0) * t;
    let (cx, cy) = (a.x + a.w / 2.0, a.y + a.h / 2.0);
    if c == now {
        pen.circle(cx, cy, r + pen.s(3.0), color::TEXT)?;
        pen.circle(cx, cy, r + pen.s(1.5), color::SURFACE)?;
    }
    pen.circle(cx, cy, r, rgb(c))?;
    hits.push((a, hit));
    Ok(())
}

/// A small accent dot marking the row that holds the current choice.
pub fn mark(pen: &mut Pen<'_>, x: f32, cy: f32) -> AureaResult<()> {
    pen.circle(x, cy, pen.s(3.0), color::ACCENT)
}

/// A round dot of LED colour `c` filling `a`.
pub fn led(pen: &mut Pen<'_>, a: Area, c: Rgb) -> AureaResult<()> {
    pen.round(a, a.w.min(a.h) / 2.0, rgb(c))
}
