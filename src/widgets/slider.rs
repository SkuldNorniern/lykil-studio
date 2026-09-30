//! A labelled slider: its name and value over a track with a knob.

use aurea::AureaResult;

use crate::anim::{Key, rate};
use crate::app::{Hit, Shared};
use crate::components::text::label;
use crate::components::track::{Track, knob, track};
use crate::draw::{Area, Hits, Pen, color};

/// `name` and `value` over a track at `t` (`0..=1`). Dragging it is `hit`.
pub fn slider(
    pen: &mut Pen<'_>,
    area: Area,
    (name, value): (&str, &str),
    (t, hit, look): (f32, Hit, Track<'_>),
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    label(pen, name, area.x, area.y)?;
    let font = pen.font(11.0);
    let vw = pen.width(value, &font);
    pen.text(value, area.right() - vw, area.y, &font, color::DIM)?;
    let bar = Area::new(area.x, area.y + pen.s(20.0), area.w, pen.s(10.0));
    track(pen, bar, t, look)?;
    let key = Key::HitKnob(hit);
    if shared.ui.dragging(hit) {
        pen.anim.set(key, t);
    }
    let v = pen.anim.to(key, t, rate::KNOB);
    let grab = Area::new(
        bar.x - pen.s(8.0),
        bar.y - pen.s(10.0),
        bar.w + pen.s(16.0),
        bar.h + pen.s(20.0),
    );
    let hover = pen.hover(grab, hit);
    knob(pen, bar, v, hover)?;
    hits.push((grab, hit));
    Ok(())
}
