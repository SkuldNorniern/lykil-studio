//! A small panel with a label and a value, such as the firmware version.

use aurea::AureaResult;
use aurea::render::Color;

use crate::components::surface::panel;
use crate::components::text::label;
use crate::draw::{Area, Pen};

pub fn stat(
    pen: &mut Pen<'_>,
    a: Area,
    name: &str,
    (value, tone): (&str, Color),
) -> AureaResult<()> {
    panel(pen, a)?;
    label(pen, name, a.x + pen.s(14.0), a.y + pen.s(12.0))?;
    let v = Area::new(
        a.x + pen.s(14.0),
        a.y + pen.s(30.0),
        a.w - pen.s(28.0),
        pen.s(20.0),
    );
    pen.fitted_left(value, v, 15.0, 9.0, tone)
}
