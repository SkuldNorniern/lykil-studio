//! A line of state with a coloured dot in front, loose or in a bar.

use aurea::AureaResult;
use aurea::render::Color;

use crate::app::Hit;
use crate::components::dot;
use crate::components::surface::banner;
use crate::draw::{Area, Hits, Pen, color};
use crate::widgets::pills::pills_to;

/// `text` after a `tone` dot, shrunk to fit `line`.
pub fn status_line(pen: &mut Pen<'_>, line: Area, text: &str, tone: Color) -> AureaResult<()> {
    let x = line.x + pen.s(4.0);
    dot::status(pen, x, line.y + line.h / 2.0, tone)?;
    pen.fitted_left(
        text,
        Area::new(x + pen.s(16.0), line.y, line.w - pen.s(20.0), line.h),
        12.0,
        8.0,
        color::DIM,
    )
}

/// [`status_line`] in a raised bar.
pub fn notice(pen: &mut Pen<'_>, bar: Area, text: &str, tone: Color) -> AureaResult<()> {
    banner(pen, bar, false)?;
    let line = Area::new(bar.x + pen.s(10.0), bar.y, bar.w - pen.s(20.0), bar.h);
    status_line(pen, line, text, tone)
}

/// A bar across the middle of `over`, which is dimmed: `text`, and a
/// button if there is one.
pub fn overlay(
    pen: &mut Pen<'_>,
    over: Area,
    text: &str,
    button: Option<(&str, Hit)>,
    hits: &mut Hits,
) -> AureaResult<()> {
    let font = pen.bold(13.0);
    let bw = button.map_or(0.0, |(b, _)| pen.width(b, &pen.bold(12.0)) + pen.s(28.0));
    let w = (pen.width(text, &font) + bw + pen.s(48.0)).min(over.w);
    let bar = Area::new(
        over.x + (over.w - w) / 2.0,
        over.y + over.h / 2.0 - pen.s(26.0),
        w,
        pen.s(52.0),
    );
    pen.veil(over, color::BACKGROUND, 0.55)?;
    banner(pen, bar, true)?;
    let room = Area::new(bar.x + pen.s(20.0), bar.y, bar.w - bw - pen.s(36.0), bar.h);
    pen.fitted_left(text, room, 13.0, 9.0, color::TEXT)?;
    if let Some((b, hit)) = button {
        let items = [(b.to_string(), hit, true)];
        pills_to(
            pen,
            bar.right() - pen.s(16.0),
            bar.y + pen.s(11.0),
            &items,
            hits,
        )?;
    }
    Ok(())
}
