//! A slider's bar and its knob.

use aurea::AureaResult;
use lykil::lighting::Rgb;

use crate::colour::rgb;
use crate::draw::{Area, Pen, color};

/// What a track shows.
#[derive(Clone, Copy)]
pub enum Track<'a> {
    /// Filled up to the knob.
    Plain,
    /// In steps, each the colour that position gives.
    Shades(&'a dyn Fn(u8) -> Rgb),
}

/// The bar, filled to `t` (`0..=1`) or in shades.
pub fn track(pen: &mut Pen<'_>, bar: Area, t: f32, track: Track<'_>) -> AureaResult<()> {
    const STEPS: u8 = 24;
    pen.round(bar, bar.h / 2.0, color::RAISED)?;
    match track {
        Track::Plain => {
            let filled = Area::new(bar.x, bar.y, bar.w * t, bar.h);
            pen.round(filled, bar.h / 2.0, color::ACCENT)
        }
        Track::Shades(shade) => {
            let inner = bar.inset(pen.s(2.0));
            let seg_w = inner.w / f32::from(STEPS);
            for i in 0..STEPS {
                #[allow(clippy::cast_possible_truncation)]
                let v = (u32::from(i) * 255 / u32::from(STEPS - 1)) as u8;
                let seg = Area::new(
                    inner.x + seg_w * f32::from(i),
                    inner.y,
                    seg_w + 1.0,
                    inner.h,
                );
                pen.fill(seg, rgb(shade(v)))?;
            }
            Ok(())
        }
    }
}

/// The knob at `t` along `bar`, bigger by `hover` (`0..=1`).
pub fn knob(pen: &mut Pen<'_>, bar: Area, t: f32, hover: f32) -> AureaResult<()> {
    let x = bar.x + bar.w * t;
    let y = bar.y + bar.h / 2.0;
    let r = pen.s(8.0) + pen.s(2.0) * hover;
    pen.circle(x, y, r, color::TEXT)?;
    pen.circle(x, y, r - pen.s(3.0), color::BACKGROUND)
}
