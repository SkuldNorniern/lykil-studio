//! A row of LED cells playing an effect, as on the effect cards.

use aurea::AureaResult;
use lykil::lighting::{Effect, Hsv, Point, Rgb, Settings, press_life};

use crate::colour::rgb;
use crate::draw::{Area, Pen, color};
use crate::effects::preview;
use crate::keyboard::Presses;

const CELLS: usize = 10;

/// Effect `e` with `settings` across `strip`, a key pressed now and then
/// for the effects that react. Per-key shows `key_colors` spread out.
pub fn led_strip(
    pen: &mut Pen<'_>,
    strip: Area,
    (settings, e): (Settings, Effect),
    time: f32,
    key_colors: &[Rgb],
) -> AureaResult<()> {
    let s = Settings {
        effect: e,
        color: Hsv::new(
            settings.color.h,
            settings.color.s,
            settings.color.v.max(160),
        ),
        ..settings
    };
    let points: Vec<Option<Point>> = (0..CELLS)
        .map(|i| {
            #[allow(clippy::cast_possible_truncation)]
            let x = (i * 255 / (CELLS - 1)) as u8;
            Some(Point::new(x, 0))
        })
        .collect();
    #[allow(clippy::cast_precision_loss)]
    let every = press_life(s.speed).0 as f32 * 1.5 / 1000.0;
    let n = (time / every).floor();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let key = (n as usize * 3 + 2) % CELLS;
    let mut at = vec![None; CELLS];
    at[key] = Some(n * every);
    let left = 1.0 - (time - n * every) / every;
    #[allow(clippy::cast_precision_loss)]
    let heat = (0..CELLS)
        .map(|i| (1.0 - i.abs_diff(key) as f32 / 3.0).max(0.0) * left)
        .collect();
    let presses = Presses {
        at: &at,
        recent: vec![(key, n * every)],
        heat,
    };
    #[allow(clippy::cast_precision_loss)]
    let cell_w = strip.w / CELLS as f32;
    for (i, p) in points.iter().enumerate() {
        let c = if e == Effect::PerKey {
            key_colors
                .get(i * key_colors.len().max(1) / CELLS)
                .copied()
                .unwrap_or(Rgb::OFF)
        } else {
            preview(s, p.unwrap_or_default(), i, time, &presses, &points)
        };
        #[allow(clippy::cast_precision_loss)]
        let cell = Area::new(
            strip.x + cell_w * i as f32 + pen.s(1.0),
            strip.y,
            cell_w - pen.s(2.0),
            strip.h,
        );
        pen.round(cell, pen.s(3.0), color::mix(color::HOVER, rgb(c), 0.9))?;
    }
    Ok(())
}
