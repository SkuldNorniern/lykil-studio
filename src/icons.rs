//! Small drawn icons, so no image files are needed.

use aurea::AureaResult;
use aurea::render::Color;

use crate::app::Tab;
use crate::draw::{Area, Pen};

/// The icon of `tab`, filling the square `a`.
#[allow(clippy::many_single_char_names)]
pub fn tab(pen: &mut Pen<'_>, tab: Tab, a: Area, c: Color) -> AureaResult<()> {
    let u = a.w / 14.0;
    let at = |x: f32, y: f32| (a.x + x * u, a.y + y * u);
    match tab {
        // A keyboard: a frame, two rows of keys and a space bar.
        Tab::Keymap => {
            pen.outline(
                Area::new(a.x, a.y + 2.0 * u, a.w, 10.0 * u),
                2.0 * u,
                1.4 * u,
                c,
            )?;
            for row in 0..2 {
                for col in 0..4 {
                    #[allow(clippy::cast_precision_loss)]
                    let (x, y) = at(2.5 + 2.5 * col as f32, 4.3 + 2.2 * row as f32);
                    pen.fill(Area::new(x, y, 1.5 * u, 1.3 * u), c)?;
                }
            }
            let (x, y) = at(4.0, 8.9);
            pen.fill(Area::new(x, y, 6.0 * u, 1.3 * u), c)
        }
        // A play mark and lines: something that types for you.
        Tab::Macros => {
            pen.polygon(&[at(1.0, 2.0), at(6.5, 5.0), at(1.0, 8.0)], c)?;
            for (y, w) in [(3.0, 5.5), (6.5, 5.5), (10.0, 12.0)] {
                let (x, y) = at(if y > 9.0 { 1.0 } else { 8.0 }, y);
                pen.round(Area::new(x, y, w * u, 1.4 * u), 0.7 * u, c)?;
            }
            Ok(())
        }
        // A sun: a disc and rays.
        Tab::Lighting => {
            let (cx, cy) = at(7.0, 7.0);
            pen.circle(cx, cy, 3.0 * u, c)?;
            for i in 0..8 {
                #[allow(clippy::cast_precision_loss)]
                let angle = i as f32 * std::f32::consts::FRAC_PI_4;
                let (s, co) = angle.sin_cos();
                pen.circle(cx + co * 5.6 * u, cy + s * 5.6 * u, 0.9 * u, c)?;
            }
            Ok(())
        }
        // A chip: a square with pins on both sides.
        Tab::Device => {
            pen.outline(
                Area::new(a.x + 3.0 * u, a.y + 3.0 * u, 8.0 * u, 8.0 * u),
                1.5 * u,
                1.4 * u,
                c,
            )?;
            pen.fill(Area::new(a.x + 5.5 * u, a.y + 5.5 * u, 3.0 * u, 3.0 * u), c)?;
            for i in 0..3 {
                #[allow(clippy::cast_precision_loss)]
                let y = a.y + (4.5 + 2.2 * i as f32) * u;
                pen.fill(Area::new(a.x, y, 2.2 * u, 1.2 * u), c)?;
                pen.fill(Area::new(a.x + 11.8 * u, y, 2.2 * u, 1.2 * u), c)?;
            }
            Ok(())
        }
        // Four panes.
        Tab::Windows => {
            for (x, y) in [(1.0, 1.0), (7.5, 1.0), (1.0, 7.5), (7.5, 7.5)] {
                let (x, y) = at(x, y);
                pen.round(Area::new(x, y, 5.5 * u, 5.5 * u), 0.8 * u, c)?;
            }
            Ok(())
        }
    }
}
