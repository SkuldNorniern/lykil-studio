//! A box of text to type in, wrapped over several lines.

use aurea::AureaResult;

use crate::components::surface::well;
use crate::components::text::wrap;
use crate::draw::{Area, Pen, color};

/// `text` in `field`, a caret at its end while `editing`, and
/// `placeholder` while it is empty and not being typed in.
pub fn text_field(
    pen: &mut Pen<'_>,
    field: Area,
    text: &str,
    editing: bool,
    placeholder: &str,
) -> AureaResult<()> {
    well(pen, field, editing)?;
    let font = pen.font(15.0);
    let x = field.x + pen.s(12.0);
    let mut y = field.y + pen.s(12.0);
    if text.is_empty() && !editing {
        return pen.text(placeholder, x, y, &font, color::FAINT);
    }
    let shown = if editing {
        format!("{text}|")
    } else {
        text.to_string()
    };
    for line in wrap(pen, &shown, &font, field.w - pen.s(24.0)) {
        if y > field.bottom() - pen.s(20.0) {
            break;
        }
        pen.text(&line, x, y, &font, color::TEXT)?;
        y += pen.s(22.0);
    }
    Ok(())
}
