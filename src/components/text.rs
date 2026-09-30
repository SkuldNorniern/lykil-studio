//! Text: section labels, headings and wrapped lines.

use aurea::AureaResult;
use aurea::render::Font;

use crate::draw::{Area, Pen, color};

/// A small caps caption over a section, such as "EFFECT".
pub fn label(pen: &mut Pen<'_>, text: &str, x: f32, y: f32) -> AureaResult<()> {
    pen.text(text, x, y, &pen.bold(11.0), color::FAINT)
}

/// A card's title.
pub fn heading(pen: &mut Pen<'_>, text: &str, x: f32, y: f32) -> AureaResult<()> {
    pen.text(text, x, y, &pen.bold(17.0), color::TEXT)
}

/// A faint line saying why something is empty or what to do, shrunk to
/// fit `area`.
pub fn hint(pen: &mut Pen<'_>, area: Area, text: &str) -> AureaResult<()> {
    pen.fitted_left(text, area, 11.0, 8.0, color::FAINT)
}

/// `text` split at its line breaks and wherever a line would run past
/// `room`.
pub fn wrap(pen: &mut Pen<'_>, text: &str, font: &Font, room: f32) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.split('\n') {
        let mut current = String::new();
        for ch in line.chars() {
            current.push(ch);
            if pen.width(&current, font) > room && current.chars().count() > 1 {
                current.pop();
                out.push(std::mem::take(&mut current));
                current.push(ch);
            }
        }
        out.push(current);
    }
    out
}
