//! The backgrounds things sit on: panels, wells, pickable cards and list
//! rows.

use aurea::AureaResult;
use aurea::render::Color;

use crate::anim::{Key, rate};
use crate::app::Hit;
use crate::draw::{Area, Pen, color};

/// A raised panel a page is split into.
pub fn panel(pen: &mut Pen<'_>, a: Area) -> AureaResult<()> {
    pen.round(a, pen.s(12.0), color::SURFACE)
}

/// A sunken area inside a panel, for text to type or a drawing.
pub fn well(pen: &mut Pen<'_>, a: Area, focused: bool) -> AureaResult<()> {
    pen.round(a, pen.s(8.0), color::BACKGROUND)?;
    if focused {
        pen.outline(a, pen.s(8.0), pen.s(1.5), color::ACCENT)?;
    }
    Ok(())
}

/// A box that stands out over what is under it.
pub fn banner(pen: &mut Pen<'_>, a: Area, border: bool) -> AureaResult<()> {
    let r = pen.s(if border { 12.0 } else { 8.0 });
    pen.round(a, r, color::RAISED)?;
    if border {
        pen.outline(a, r, pen.s(1.0), color::BORDER)?;
    }
    Ok(())
}

/// One of several cards to pick from: lighter on hover, accent tinted and
/// outlined when picked. `id` keeps its outline's fade apart from other
/// cards. Returns the colour its text should be.
pub fn choice(
    pen: &mut Pen<'_>,
    a: Area,
    (hit, id): (Hit, usize),
    picked: bool,
) -> AureaResult<Color> {
    let t = pen.hover(a, hit);
    let bg = if picked {
        color::mix(color::RAISED, color::ACCENT, 0.18)
    } else {
        color::mix(color::RAISED, color::HOVER, t)
    };
    pen.round(a, pen.s(10.0), bg)?;
    let sel = pen.anim.to(
        Key::Selected(1000 + id),
        if picked { 1.0 } else { 0.0 },
        rate::HOVER,
    );
    if sel > 0.01 {
        pen.outline(
            a,
            pen.s(10.0),
            pen.s(2.0),
            color::mix(bg, color::ACCENT, sel),
        )?;
    }
    Ok(if picked { color::TEXT } else { color::DIM })
}

/// A row in a list down a panel, with an accent mark when picked. Returns
/// the colour its text should be.
pub fn list_row(pen: &mut Pen<'_>, a: Area, hit: Hit, picked: bool) -> AureaResult<Color> {
    let t = pen.hover(a, hit);
    let bg = if picked {
        color::RAISED
    } else {
        color::mix(color::SURFACE, color::RAISED, 0.5 * t)
    };
    pen.round(a, pen.s(6.0), bg)?;
    if picked {
        pen.round(
            Area::new(a.x, a.y + pen.s(5.0), pen.s(3.0), a.h - pen.s(10.0)),
            pen.s(1.5),
            color::ACCENT,
        )?;
    }
    Ok(if picked { color::TEXT } else { color::DIM })
}

/// Covers `a` half way, for something that cannot be used here.
pub fn dim(pen: &mut Pen<'_>, a: Area) -> AureaResult<()> {
    pen.veil(a, color::SURFACE, 0.65)
}
