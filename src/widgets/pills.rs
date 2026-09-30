//! Rows of pills: buttons side by side, or small choices wrapping in an
//! area. Each item is its text, its hit and whether it is on.

use aurea::AureaResult;

use crate::app::Hit;
use crate::components::button::{Tone, pill, pill_width};
use crate::draw::{Area, Hits, Pen};

pub type Item = (String, Hit, bool);

/// Buttons left to right from `x`, 30 px tall. Returns where the last
/// one ends.
pub fn pills(
    pen: &mut Pen<'_>,
    x: f32,
    y: f32,
    items: &[Item],
    hits: &mut Hits,
) -> AureaResult<f32> {
    let font = pen.bold(12.0);
    let mut x = x;
    for (text, hit, on) in items {
        let w = pill_width(pen, text, &font, 12.0);
        let a = Area::new(x, y, w, pen.s(30.0));
        let tone = if *on { Tone::On } else { Tone::Plain };
        pill(pen, a, (text, &font), *hit, tone)?;
        hits.push((a, *hit));
        x += w + pen.s(8.0);
    }
    Ok(x)
}

/// As [`pills`], ending at `right`. Returns where the first one starts.
pub fn pills_to(
    pen: &mut Pen<'_>,
    right: f32,
    y: f32,
    items: &[Item],
    hits: &mut Hits,
) -> AureaResult<f32> {
    let font = pen.bold(12.0);
    let gap = pen.s(8.0);
    let w: f32 = items
        .iter()
        .map(|(text, ..)| pill_width(pen, text, &font, 12.0) + gap)
        .sum();
    let start = right - w + gap;
    pills(pen, start, y, items, hits)?;
    Ok(start)
}

/// Small choices left to right, wrapping inside `area` and stopping at its
/// bottom. Returns the height they took.
pub fn pill_flow(
    pen: &mut Pen<'_>,
    area: Area,
    items: &[Item],
    hits: &mut Hits,
) -> AureaResult<f32> {
    let font = pen.font(11.0);
    let (height, gap) = (pen.s(24.0), pen.s(5.0));
    let (mut x, mut y) = (area.x, area.y);
    for (text, hit, on) in items {
        let w = pill_width(pen, text, &font, 9.0).min(area.w);
        if x + w > area.right() && x > area.x {
            x = area.x;
            y += height + gap;
        }
        if y + height > area.bottom() {
            break;
        }
        let a = Area::new(x, y, w, height);
        let tone = if *on { Tone::On } else { Tone::Quiet };
        pill(pen, a, (text, &font), *hit, tone)?;
        hits.push((a, *hit));
        x += w + gap;
    }
    Ok(y + height - area.y)
}
