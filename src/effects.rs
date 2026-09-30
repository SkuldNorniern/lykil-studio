//! Lykil's effects on the host: their names, which settings they take, and
//! running them with the firmware's own `shade` for the previews.

use lykil::lighting::{Effect, Moment, Point, Rgb, Settings, shade};
use lykil::time::{Duration, Tick};
use lykil_protocol::describe::{Description, Key};

use crate::keyboard::Presses;

pub const fn name(e: Effect) -> &'static str {
    match e {
        Effect::Off => "Off",
        Effect::Solid => "Solid",
        Effect::Breathing => "Breathing",
        Effect::Cycle => "Cycle",
        Effect::Wave => "Wave",
        Effect::Reactive => "Reactive",
        Effect::Ripple => "Ripple",
        Effect::PerKey => "Per-key",
        Effect::Starlight => "Starlight",
        Effect::Rain => "Rain",
        Effect::Heatmap => "Heatmap",
        Effect::Predict => "Predict",
    }
}

pub const fn about(e: Effect) -> &'static str {
    match e {
        Effect::Off => "LEDs off",
        Effect::Solid => "One steady colour",
        Effect::Breathing => "Fades in and out",
        Effect::Cycle => "Round the colour wheel",
        Effect::Wave => "A rainbow moving across",
        Effect::Reactive => "Dim; pressed keys flash",
        Effect::Ripple => "Rings from pressed keys",
        Effect::PerKey => "Paint every key",
        Effect::Starlight => "Keys twinkle at random",
        Effect::Rain => "Drops fall down the board",
        Effect::Heatmap => "Keys warm up as you type",
        Effect::Predict => "Lights the likely next keys",
    }
}

pub const fn has_background(e: Effect) -> bool {
    matches!(
        e,
        Effect::Starlight
            | Effect::Rain
            | Effect::Reactive
            | Effect::Ripple
            | Effect::Heatmap
            | Effect::Predict
    )
}

pub const fn has_size(e: Effect) -> bool {
    matches!(e, Effect::Reactive | Effect::Ripple | Effect::Heatmap)
}

/// Each key's place for the effects, as the firmware computes it: key
/// centres scaled so they span `0..=255` across, `y` on the same scale.
/// With `leds_only`, only keys with an LED get one.
pub fn points(desc: &Description, leds_only: bool) -> Vec<Option<Point>> {
    let centre = |k: &Key| {
        if leds_only {
            k.led?;
        }
        k.geometry.map(|[x, y, w, h]| (x + w / 2.0, y + h / 2.0))
    };
    let centres: Vec<_> = desc.keys.iter().map(centre).collect();
    let known = centres.iter().flatten();
    let min_x = known.clone().map(|c| c.0).fold(f64::INFINITY, f64::min);
    let max_x = known.clone().map(|c| c.0).fold(f64::NEG_INFINITY, f64::max);
    let min_y = known.map(|c| c.1).fold(f64::INFINITY, f64::min);
    let span = max_x - min_x;
    let scale = if span > 0.0 { 255.0 / span } else { 0.0 };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let to_u8 = |v: f64| (v * scale).round().clamp(0.0, 255.0) as u8;
    centres
        .into_iter()
        .map(|c| c.map(|(x, y)| Point::new(to_u8(x - min_x), to_u8(y - min_y))))
        .collect()
}

/// The colour key `index` at `at` shows at `time` (seconds).
pub fn preview(
    s: Settings,
    at: Point,
    index: usize,
    time: f32,
    presses: &Presses<'_>,
    points: &[Option<Point>],
) -> Rgb {
    let ms = |t: f32| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ms = (t.max(0.0) * 1000.0) as u32;
        ms
    };
    let now = Tick(ms(time));
    let since_press = presses
        .at
        .get(index)
        .copied()
        .flatten()
        .map(|t| Duration(ms(time - t)));
    let recent: Vec<(Point, Duration)> = presses
        .recent
        .iter()
        .filter_map(|&(k, t)| Some((points.get(k).copied().flatten()?, Duration(ms(time - t)))))
        .collect();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let heat = (presses.heat.get(index).copied().unwrap_or(0.0) * 255.0) as u8;
    let moment = Moment {
        now,
        pressed: since_press,
        presses: &recent,
        heat,
        predicted: presses.predicted.get(index).copied().unwrap_or(0),
    };
    shade(s, at, &moment).to_rgb()
}
