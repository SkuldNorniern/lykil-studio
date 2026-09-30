//! A keyed device drawn from its description: caps with legends, lit
//! faces for the lighting preview, pressed keys for the key test.

use aurea::AureaResult;
use aurea::render::Color;
use lykil::binding::Binding;
use lykil::lighting::{Effect, Point, Rgb, Settings};
use lykil_protocol::describe::Description;

use crate::anim::{Key, rate};
use crate::app::Hit;
use crate::device::Keyboard;
use crate::draw::{Area, Hits, Pen, color};
use crate::legend;

pub enum Keys<'a> {
    Keymap {
        bindings: &'a [Binding],
        layers: &'a [String],
        selected: Option<usize>,
    },
    /// The lighting preview at `time` seconds; `colors` are the per-key
    /// colours by LED, `points` each key's place for the effects, `brush`
    /// the per-key brush shown on the key under the mouse.
    Lighting {
        settings: Settings,
        time: f32,
        colors: &'a [Rgb],
        points: &'a [Option<Point>],
        presses: &'a Presses<'a>,
        brush: Option<Rgb>,
    },
    /// The key test: what each key does on the first layer, or its id.
    Device {
        bindings: &'a [Binding],
        layers: &'a [String],
    },
}

pub fn keyboard(
    pen: &mut Pen<'_>,
    area: Area,
    kb: &Keyboard,
    keys: &Keys<'_>,
    hover_keys: bool,
    hits: &mut Hits,
) -> AureaResult<Area> {
    let Some(desc) = &kb.description else {
        return Ok(Area::default());
    };
    let Some((x0, y0, x1, y1)) = bounds(desc) else {
        return Ok(Area::default());
    };
    #[allow(clippy::cast_possible_truncation)]
    let (units_w, units_h) = ((x1 - x0) as f32, (y1 - y0) as f32);
    let unit = (area.w / units_w)
        .min(area.h / units_h)
        .min(pen.s(MAX_UNIT));
    let used = Area::new(
        area.x + (area.w - unit * units_w) / 2.0,
        area.y,
        unit * units_w,
        unit * units_h,
    );
    let gap = (unit * 0.08).max(2.0);
    for (i, key) in desc.keys.iter().enumerate() {
        let Some([x, y, kw, kh]) = key.geometry else {
            continue;
        };
        #[allow(clippy::cast_possible_truncation)]
        let cap = Area::new(
            used.x + (x - x0) as f32 * unit + gap / 2.0,
            used.y + (y - y0) as f32 * unit + gap / 2.0,
            kw as f32 * unit - gap,
            kh as f32 * unit - gap,
        );
        #[allow(clippy::cast_possible_truncation)]
        let info = Cap {
            index: i,
            id: &key.id,
            led: key.led,
            down: key.cell.is_some_and(|c| kb.closed(c)),
            hovered: if hover_keys {
                pen.hover(cap, Hit::Key(i))
            } else {
                0.0
            },
        };
        keycap(pen, cap, &info, keys)?;
        if hover_keys {
            hits.push((cap, Hit::Key(i)));
        }
    }
    Ok(used)
}

pub struct Cap<'a> {
    index: usize,
    id: &'a str,
    led: Option<u16>,
    down: bool,
    hovered: f32,
}

pub fn keycap(pen: &mut Pen<'_>, cap: Area, info: &Cap<'_>, keys: &Keys<'_>) -> AureaResult<()> {
    let Cap {
        index,
        id,
        led: _,
        down,
        hovered,
    } = *info;
    let radius = pen.s(6.0);
    let (face, selected) = match keys {
        Keys::Lighting { .. } => (lit_face(info, keys), false),
        Keys::Keymap { selected, .. } => (
            color::mix(color::RAISED, color::HOVER, hovered),
            *selected == Some(index),
        ),
        Keys::Device { .. } => (color::RAISED, false),
    };
    let glow = pen.anim.towards(
        Key::Down(index),
        if down { 1.0 } else { 0.0 },
        rate::PRESS,
        rate::RELEASE,
    );
    let face = if matches!(keys, Keys::Device { .. }) {
        color::mix(face, color::PRESSED, glow * 0.55)
    } else {
        face
    };
    let sink = pen.s(1.5) * glow;
    let cap = Area::new(cap.x, cap.y + sink, cap.w, cap.h - sink);
    pen.round(cap, radius, color::mix(face, color::BACKGROUND, 0.45))?;
    let top = Area::new(
        cap.x + pen.s(2.0),
        cap.y + pen.s(1.0),
        cap.w - pen.s(4.0),
        cap.h - pen.s(5.0) + sink,
    );
    pen.round(top, radius * 0.8, face)?;
    if glow > 0.01 {
        pen.outline(
            cap,
            radius,
            pen.s(2.0),
            color::mix(face, color::PRESSED, glow),
        )?;
    }
    let sel = pen.anim.to(
        Key::Selected(index),
        if selected { 1.0 } else { 0.0 },
        rate::HOVER,
    );
    if sel > 0.01 {
        pen.outline(
            cap,
            radius,
            pen.s(1.0) + pen.s(1.5) * sel,
            color::mix(face, color::ACCENT, sel),
        )?;
    }
    let label = top.inset(pen.s(3.0));
    match keys {
        Keys::Keymap {
            bindings, layers, ..
        } => {
            let binding = bindings.get(index).copied().unwrap_or_default();
            let (main, sub) = legend::keycap(binding, layers);
            let c = if binding == Binding::Transparent {
                color::FAINT
            } else {
                color::TEXT
            };
            match sub {
                Some(sub) => {
                    let upper = Area::new(label.x, label.y, label.w, label.h * 0.62);
                    let lower =
                        Area::new(label.x, label.y + label.h * 0.58, label.w, label.h * 0.38);
                    pen.fitted(&main, upper, 12.0, 7.0, c)?;
                    pen.fitted(&sub, lower, 9.0, 6.0, color::ACCENT)
                }
                None => pen.fitted(&main, label, 12.0, 7.0, c),
            }
        }
        Keys::Device { bindings, layers } => {
            let main = bindings
                .get(index)
                .map(|b| legend::keycap(*b, layers).0)
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| id.to_string());
            pen.fitted(&main, label, 11.0, 6.0, color::DIM)
        }
        Keys::Lighting { .. } => Ok(()),
    }
}

pub fn lit_face(info: &Cap<'_>, keys: &Keys<'_>) -> Color {
    let Keys::Lighting {
        settings,
        time,
        colors,
        points,
        presses,
        brush,
    } = keys
    else {
        return color::RAISED;
    };
    let lit = if settings.effect == Effect::PerKey {
        info.led
            .and_then(|l| colors.get(usize::from(l)).copied())
            .unwrap_or(Rgb::OFF)
            .scale(settings.color.v)
    } else {
        points
            .get(info.index)
            .copied()
            .flatten()
            .map_or(Rgb::OFF, |p| {
                crate::lights::preview(*settings, p, info.index, *time, presses, points)
            })
    };
    // A dark LED leaves the cap visible; light adds to it.
    let face = color::glow(color::SURFACE, crate::widgets::colour::rgb(lit));
    match brush {
        Some(b) if info.led.is_some() => {
            color::mix(face, crate::widgets::colour::rgb(*b), info.hovered * 0.6)
        }
        _ => color::mix(face, color::TEXT, info.hovered * 0.15),
    }
}

pub fn bounds(desc: &Description) -> Option<(f64, f64, f64, f64)> {
    let mut b: Option<(f64, f64, f64, f64)> = None;
    for [x, y, w, h] in desc.keys.iter().filter_map(|k| k.geometry) {
        let (l, t, r, bo) = b.unwrap_or((x, y, x + w, y + h));
        b = Some((l.min(x), t.min(y), r.max(x + w), bo.max(y + h)));
    }
    b
}

pub const MAX_UNIT: f32 = 58.0;

pub struct Presses<'a> {
    pub at: &'a [Option<f32>],
    pub recent: Vec<(usize, f32)>,
    pub heat: Vec<f32>,
}
