//! Options side by side in one bar; the chosen one's background slides.

use aurea::AureaResult;

use crate::anim::{Key, rate};
use crate::app::Hit;
use crate::draw::{Area, Hits, Pen, color};

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

/// [`segmented`] under a label: `name`, then the bar `h` tall. Returns
/// the height it took.
pub fn labelled(
    pen: &mut Pen<'_>,
    (x, y, w, h): (f32, f32, f32, f32),
    name: &str,
    (items, id): (&[(&str, Hit)], usize),
    chosen: usize,
    hits: &mut Hits,
) -> AureaResult<f32> {
    crate::components::text::label(pen, name, x, y)?;
    let bar = Area::new(x, y + pen.s(16.0), w, h);
    segmented(pen, bar, (items, id), chosen, hits)?;
    Ok(bar.bottom() - y)
}
