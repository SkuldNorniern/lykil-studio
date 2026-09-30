//! Picking a colour: the square with the hue strip under it, and rows of
//! swatches.

use aurea::AureaResult;
use lykil::lighting::{Hsv, Rgb};

use crate::app::Hit;
use crate::components::colour::{hue_strip, square};
use crate::components::dot::swatch;
use crate::draw::{Area, Hits, Pen};

/// Saturation across and brightness up in `sq`, hue in the strip below
/// it. Dragging them is `square_hit` and `hue_hit`.
pub fn picker(
    pen: &mut Pen<'_>,
    sq: Area,
    c: Hsv,
    (square_hit, hue_hit): (Hit, Hit),
    hits: &mut Hits,
) -> AureaResult<()> {
    square(pen, sq, c, square_hit, hits)?;
    let bar = Area::new(sq.x, sq.bottom() + pen.s(12.0), sq.w, pen.s(14.0));
    hue_strip(pen, bar, c.h, hue_hit, hits)
}

/// `colours` as dots `size` across, in rows inside `area`, the one that is
/// `now` circled. Returns the height they took.
pub fn swatches(
    pen: &mut Pen<'_>,
    area: Area,
    (colours, size): (&[Rgb], f32),
    now: Rgb,
    hits: &mut Hits,
) -> AureaResult<f32> {
    let (dot, gap) = (pen.s(size), pen.s(6.0));
    let per_row = ((area.w + gap) / (dot + gap)).floor().max(1.0);
    let mut used = 0.0;
    #[allow(clippy::cast_precision_loss)]
    for (i, c) in colours.iter().enumerate() {
        let (col, row) = (i as f32 % per_row, (i as f32 / per_row).floor());
        let a = Area::new(
            area.x + col * (dot + gap),
            area.y + row * (dot + gap),
            dot,
            dot,
        );
        if a.bottom() > area.bottom() {
            break;
        }
        swatch(pen, a, *c, now, hits)?;
        used = a.bottom() - area.y;
    }
    Ok(used)
}
