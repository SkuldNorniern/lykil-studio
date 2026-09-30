//! How a page splits its space.

use crate::draw::{Area, Pen};

/// Where the effects, the picker and the sliders of a lighting page go.
/// Wide: three columns. Narrow: effects on top, picker and sliders below.
/// Panels stop at what they hold instead of stretching down a tall window.
pub fn lighting_panels(pen: &Pen<'_>, rest: Area, gap: f32) -> (Area, Area, Area) {
    let picker_w = pen.s(360.0);
    let side_min = pen.s(260.0);
    let effects_min = pen.s(380.0);
    if rest.w >= effects_min + picker_w + side_min + 2.0 * gap {
        let h = rest.h.min(pen.s(380.0));
        let effects_w = (rest.w * 0.42).max(effects_min);
        let effects = Area::new(rest.x, rest.y, effects_w, h);
        let picker = Area::new(effects.right() + gap, rest.y, picker_w, h);
        let side = Area::new(
            picker.right() + gap,
            rest.y,
            rest.right() - picker.right() - gap,
            h,
        );
        return (effects, picker, side);
    }
    let effects_h = pen.s(300.0).min(rest.h * 0.5);
    let effects = Area::new(rest.x, rest.y, rest.w, effects_h);
    let below = rest.y + effects_h + gap;
    let h = (rest.bottom() - below).min(pen.s(380.0));
    let picker_w = picker_w.min((rest.w - gap) * 0.55);
    let picker = Area::new(rest.x, below, picker_w, h);
    let side = Area::new(
        picker.right() + gap,
        below,
        rest.right() - picker.right() - gap,
        h,
    );
    (effects, picker, side)
}

/// `cols` columns across `area`, `gap` apart: the `i`th cell of height `h`
/// in reading order.
pub fn cell(area: Area, (cols, gap, h): (usize, f32, f32), i: usize) -> Area {
    #[allow(clippy::cast_precision_loss)]
    let w = (area.w - gap * (cols - 1) as f32) / cols as f32;
    #[allow(clippy::cast_precision_loss)]
    let (col, row) = ((i % cols) as f32, (i / cols) as f32);
    Area::new(area.x + col * (w + gap), area.y + row * (h + gap), w, h)
}
