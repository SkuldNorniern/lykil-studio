//! A one-line box to type in.

use aurea::AureaResult;

use crate::app::Hit;
use crate::draw::{Area, Hits, Pen, color};

/// `text` in a box; while `editing`, outlined with a blinking caret.
pub fn input(
    pen: &mut Pen<'_>,
    a: Area,
    text: &str,
    editing: bool,
    hit: Hit,
    hits: &mut Hits,
) -> AureaResult<()> {
    let t = pen.hover(a, hit);
    pen.round(
        a,
        pen.s(6.0),
        color::mix(color::BACKGROUND, color::RAISED, t * 0.6),
    )?;
    let shown = if editing {
        let caret = (pen.anim.time() * 2.0).fract() < 0.5;
        format!("{text}{}", if caret { "|" } else { " " })
    } else {
        text.to_string()
    };
    if editing {
        pen.outline(a, pen.s(6.0), pen.s(1.5), color::ACCENT)?;
    }
    let inner = Area::new(a.x + pen.s(8.0), a.y, a.w - pen.s(12.0), a.h);
    pen.fitted_left(&shown, inner, 13.0, 8.0, color::TEXT)?;
    hits.push((a, hit));
    Ok(())
}
