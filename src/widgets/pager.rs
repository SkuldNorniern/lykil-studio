//! `‹ 2 / 4 ›` under something split into pages.

use aurea::AureaResult;

use crate::app::Hit;
use crate::components::button::arrow;
use crate::draw::{Area, Hits, Pen, color};

/// Centred in `area`; the arrows are `page(n)` for the page they go to.
pub fn pager(
    pen: &mut Pen<'_>,
    area: Area,
    (at, pages): (usize, usize),
    page: fn(usize) -> Hit,
    hits: &mut Hits,
) -> AureaResult<()> {
    let text = format!("{} / {pages}", at + 1);
    pen.centred(&text, area, &pen.bold(12.0), color::DIM)?;
    let (mid, side) = (area.x + area.w / 2.0, area.h);
    let half = pen.s(44.0);
    let arrows = [
        (at.checked_sub(1), mid - half - side, -1.0),
        (Some(at + 1).filter(|p| *p < pages), mid + half, 1.0),
    ];
    for (to, x, dir) in arrows {
        let Some(to) = to else {
            continue;
        };
        let a = Area::new(x, area.y, side, side);
        arrow(pen, a, dir, page(to))?;
        hits.push((a, page(to)));
    }
    Ok(())
}
