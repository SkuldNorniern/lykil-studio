//! The lighting page. The preview runs the firmware's own `shade`.

use aurea::AureaResult;
use lykil::lighting::{Effect, Hsv, Moment, Palette, Point, Rgb, Settings, press_life, shade};
use lykil::time::{Duration, Tick};

use crate::app::{Field, Hit, SWATCHES, Shared, Slider};
use crate::draw::Hits;
use crate::draw::{Area, Pen, color};
use crate::keyboard::{self, Keys, Presses};
use crate::widgets::colour::{rgb, square_and_bar};
use crate::widgets::effects::{EffectCard, effect_grid};
use crate::widgets::{self, Track, segmented};

pub fn tab(pen: &mut Pen<'_>, body: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<String> {
    let lang = pen.lang;
    let Some(settings) = shared.lighting() else {
        pen.centred(
            lang.tr("This keyboard has no lighting"),
            body,
            &pen.font(15.0),
            color::DIM,
        )?;
        return Ok(String::new());
    };
    let kb = &shared.keyboard;
    let time = pen.anim.time();
    let brushing = settings.effect == Effect::PerKey;
    let points = kb
        .description
        .as_ref()
        .map(|d| points(d, true))
        .unwrap_or_default();
    let presses = Presses {
        at: &shared.ui.presses.at,
        recent: shared.ui.presses.recent.iter().copied().collect(),
        heat: (0..points.len())
            .map(|k| shared.ui.presses.heat_at(k, time, settings.speed))
            .collect(),
    };
    let kb_area = Area::new(body.x, body.y + pen.s(4.0), body.w, body.h * 0.44);
    let used = keyboard::keyboard(
        pen,
        kb_area,
        kb,
        &Keys::Lighting {
            settings,
            time,
            colors: &kb.key_colors,
            points: &points,
            presses: &presses,
            brush: brushing.then(|| shared.picked().to_rgb()),
        },
        true,
        hits,
    )?;

    if kb.lighting.is_some_and(|i| i.host) {
        host_banner(pen, used, settings.os_lighting, hits)?;
    }
    let line = Area::new(body.x, used.bottom() + pen.s(10.0), body.w, pen.s(18.0));
    status_line(pen, line, shared, settings)?;
    let top = line.bottom() + pen.s(10.0);
    let gap = pen.s(16.0);
    let rest = Area::new(body.x, top, body.w, body.bottom() - top);
    let (effects, picker, side) = panels(pen, rest, gap);
    for a in [effects, picker, side] {
        pen.round(a, pen.s(12.0), color::SURFACE)?;
    }
    effect_cards(pen, effects.inset(pen.s(16.0)), settings, time, kb, hits)?;
    picker_card(pen, picker.inset(pen.s(16.0)), shared, hits)?;
    side_card(pen, side.inset(pen.s(16.0)), shared, settings, hits)?;

    let info = kb.lighting;
    Ok(match info {
        Some(i) if !i.drivers_ok => lang
            .tr("The LED driver chips do not answer; the keyboard keeps trying.")
            .into(),
        Some(i) if i.host => lang
            .tr("An app or Windows is setting the colours right now.")
            .into(),
        _ if brushing => lang
            .tr("Click or drag to paint, right click takes a key's colour. Ctrl+C and Ctrl+V copy and paste colours.")
            .into(),
        _ if settings.effect.reacts() => lang
            .tr("Type on the keyboard, or click keys here, to see it.")
            .into(),
        Some(i) => lang.fill(
            "{} LEDs. Settings are saved on the keyboard.",
            &[&i.leds.to_string()],
        ),
        None => String::new(),
    })
}

fn host_banner(pen: &mut Pen<'_>, preview: Area, os: bool, hits: &mut Hits) -> AureaResult<()> {
    let lang = pen.lang;
    let text = if os {
        lang.tr("Windows Dynamic Lighting has the LEDs, so the effect below does not run.")
    } else {
        lang.tr("An app has the LEDs; the effect comes back when it lets go.")
    };
    let font = pen.bold(13.0);
    let button = lang.tr("Use keyboard effects");
    let bw = if os {
        pen.width(button, &pen.bold(12.0)) + pen.s(28.0)
    } else {
        0.0
    };
    let w = (pen.width(text, &font) + bw + pen.s(48.0)).min(preview.w);
    let bar = Area::new(
        preview.x + (preview.w - w) / 2.0,
        preview.y + preview.h / 2.0 - pen.s(26.0),
        w,
        pen.s(52.0),
    );
    pen.veil(preview, color::BACKGROUND, 0.55)?;
    pen.round(bar, pen.s(12.0), color::RAISED)?;
    pen.outline(bar, pen.s(12.0), pen.s(1.0), color::BORDER)?;
    let text_area = Area::new(bar.x + pen.s(20.0), bar.y, bar.w - bw - pen.s(36.0), bar.h);
    pen.fitted_left(text, text_area, 13.0, 9.0, color::TEXT)?;
    if os {
        let items = [(button.to_string(), Hit::OsLighting(false), true)];
        widgets::pills(
            pen,
            bar.right() - bw - pen.s(8.0),
            bar.y + pen.s(11.0),
            &items,
            hits,
        )?;
    }
    Ok(())
}

/// Who drives the LEDs right now, and whether Studio lights other
/// devices with this effect too.
fn status_line(
    pen: &mut Pen<'_>,
    line: Area,
    shared: &Shared,
    settings: Settings,
) -> AureaResult<()> {
    let lang = pen.lang;
    let info = shared.keyboard.lighting;
    let (text, dot) = match info {
        Some(i) if !i.drivers_ok => (
            lang.tr("The LED drivers do not answer, so the keyboard stays dark."),
            color::BAD,
        ),
        Some(i) if i.host && settings.os_lighting => (
            lang.tr("Windows Dynamic Lighting has the LEDs: this effect is paused."),
            color::PRESSED,
        ),
        Some(i) if i.host => (
            lang.tr("An app has the LEDs: this effect is paused."),
            color::PRESSED,
        ),
        _ if settings.os_lighting => (
            lang.tr("The keyboard runs this effect. Windows may take the LEDs at any time."),
            color::GOOD,
        ),
        _ => (
            lang.tr("The keyboard runs this effect. Windows does not see it as a lighting device."),
            color::GOOD,
        ),
    };
    let mut text = text.to_string();
    if shared.lamp_sync {
        let followed = shared.lamps.iter().filter(|l| l.place.follow);
        let (lit, waiting) = followed.fold((0, 0), |(lit, waiting), l| {
            if l.open && l.available {
                (lit + 1, waiting)
            } else {
                (lit, waiting + 1)
            }
        });
        if lit > 0 {
            text.push_str("  ");
            text.push_str(&lang.fill(
                "Studio lights {} other devices with it.",
                &[&lit.to_string()],
            ));
        } else if waiting > 0 {
            text.push_str("  ");
            text.push_str(lang.tr("Other devices wait for Studio to be in front."));
        }
    }
    let x = line.x + pen.s(4.0);
    pen.circle(x + pen.s(4.0), line.y + line.h / 2.0, pen.s(4.0), dot)?;
    pen.fitted_left(
        &text,
        Area::new(x + pen.s(16.0), line.y, line.w - pen.s(20.0), line.h),
        12.0,
        8.0,
        color::DIM,
    )
}

/// Where the effects, the picker and the sliders go. Wide: three
/// columns. Narrow: effects on top, picker and sliders below. Cards stop
/// at what they hold instead of stretching down a tall window.
pub fn panels(pen: &Pen<'_>, rest: Area, gap: f32) -> (Area, Area, Area) {
    let picker_w = pen.s(360.0);
    let side_min = pen.s(260.0);
    let effects_min = pen.s(380.0);
    if rest.w >= effects_min + picker_w + side_min + 2.0 * gap {
        let h = rest.h.min(pen.s(380.0));
        let effects_w = (rest.w * 0.42).max(effects_min);
        let effects = Area::new(rest.x, rest.y, effects_w, h);
        let picker = Area::new(effects.right() + gap, rest.y, picker_w, h);
        let side = Area::new(
            picker.right() + gap,
            rest.y,
            rest.right() - picker.right() - gap,
            h,
        );
        return (effects, picker, side);
    }
    let effects_h = pen.s(300.0).min(rest.h * 0.5);
    let effects = Area::new(rest.x, rest.y, rest.w, effects_h);
    let below = rest.y + effects_h + gap;
    let h = (rest.bottom() - below).min(pen.s(380.0));
    let picker_w = picker_w.min((rest.w - gap) * 0.55);
    let picker = Area::new(rest.x, below, picker_w, h);
    let side = Area::new(
        picker.right() + gap,
        below,
        rest.right() - picker.right() - gap,
        h,
    );
    (effects, picker, side)
}

/// Each key's place for the effects, as the firmware computes it: key
/// centres of keys with an LED, scaled so they span `0..=255` across,
/// `y` on the same scale.
/// Each key's place for the effects; with `leds_only`, only keys with an
/// LED get one.
pub fn points(desc: &lykil_protocol::describe::Description, leds_only: bool) -> Vec<Option<Point>> {
    let centre = |k: &lykil_protocol::describe::Key| {
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
    };
    shade(s, at, &moment).to_rgb()
}

pub const fn effect_name(e: Effect) -> &'static str {
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
    }
}

pub const fn effect_about(e: Effect) -> &'static str {
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
    }
}

const fn has_background(e: Effect) -> bool {
    matches!(
        e,
        Effect::Starlight | Effect::Rain | Effect::Reactive | Effect::Ripple | Effect::Heatmap
    )
}

const fn has_size(e: Effect) -> bool {
    matches!(e, Effect::Reactive | Effect::Ripple | Effect::Heatmap)
}

fn effect_cards(
    pen: &mut Pen<'_>,
    area: Area,
    settings: Settings,
    time: f32,
    kb: &crate::devices::Keyboard,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    widgets::label(pen, lang.tr("EFFECT"), area.x, area.y)?;
    let grid = Area::new(area.x, area.y + pen.s(20.0), area.w, area.h - pen.s(20.0));
    let cards: Vec<EffectCard<'_>> = Effect::ALL
        .into_iter()
        .map(|e| EffectCard {
            name: lang.tr(effect_name(e)),
            about: lang.tr(effect_about(e)),
            hit: Hit::Effect(e),
            active: e == settings.effect,
        })
        .collect();
    effect_grid(
        pen,
        grid,
        &cards,
        0,
        &mut |pen, bar, i| effect_strip(pen, bar, settings, Effect::ALL[i], time, kb),
        hits,
    )
}

/// A row of cells running effect `e` with `settings`, for a card.
pub fn effect_strip(
    pen: &mut Pen<'_>,
    strip: Area,
    settings: Settings,
    e: Effect,
    time: f32,
    kb: &crate::devices::Keyboard,
) -> AureaResult<()> {
    const CELLS: usize = 10;
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
            kb.key_colors
                .get(i * kb.key_colors.len().max(1) / CELLS)
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
        let face = color::mix(color::HOVER, rgb(c), 0.9);
        pen.round(cell, pen.s(3.0), face)?;
    }
    Ok(())
}

fn picker_card(pen: &mut Pen<'_>, area: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<()> {
    let lang = pen.lang;
    let brushing = shared.brushing();
    let picked = shared.picked();
    widgets::label(
        pen,
        lang.tr(if brushing { "BRUSH" } else { "COLOUR" }),
        area.x,
        area.y,
    )?;
    let mut top = area.y + pen.s(20.0);
    if let Some(s) = shared.lighting().filter(|_| !brushing) {
        segmented(
            pen,
            Area::new(area.x, top, area.w, pen.s(28.0)),
            (
                &[
                    (lang.tr("One colour"), Hit::Colours(Palette::One)),
                    (lang.tr("Two colours"), Hit::Colours(Palette::Two)),
                    (lang.tr("Rainbow"), Hit::Colours(Palette::Rainbow)),
                ],
                1,
            ),
            s.palette as usize,
            hits,
        )?;
        top += pen.s(36.0);
        if s.palette == Palette::Two {
            which_colour(
                pen,
                Area::new(area.x, top, area.w, pen.s(26.0)),
                s,
                shared,
                hits,
            )?;
            top += pen.s(34.0);
        }
    }
    let side = (area.bottom() - top - pen.s(30.0))
        .min(pen.s(170.0))
        .max(pen.s(60.0));
    let square = Area::new(area.x, top, side, side);
    square_and_bar(pen, square, picked, (Hit::Square, Hit::HueBar), hits)?;
    let right = Area::new(
        square.right() + pen.s(16.0),
        top,
        area.right() - square.right() - pen.s(16.0),
        area.bottom() - top,
    );
    codes(pen, right, picked.to_rgb(), shared, hits)
}

fn which_colour(
    pen: &mut Pen<'_>,
    area: Area,
    s: Settings,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let w = (area.w - pen.s(8.0)) / 2.0;
    let second = shared.editing_second();
    for (i, (name, c, hit, on)) in [
        ("Colour 1", s.color, Hit::Second(false), !second),
        ("Colour 2", s.second(), Hit::Second(true), second),
    ]
    .into_iter()
    .enumerate()
    {
        #[allow(clippy::cast_precision_loss)]
        let a = Area::new(area.x + (w + pen.s(8.0)) * i as f32, area.y, w, area.h);
        let t = pen.hover(a, hit);
        let bg = if on {
            color::mix(color::RAISED, color::ACCENT, 0.2)
        } else {
            color::mix(color::BACKGROUND, color::RAISED, t)
        };
        pen.round(a, pen.s(7.0), bg)?;
        if on {
            pen.outline(a, pen.s(7.0), pen.s(1.5), color::ACCENT)?;
        }
        let dot = Area::new(
            a.x + pen.s(8.0),
            a.y + pen.s(6.0),
            a.h - pen.s(12.0),
            a.h - pen.s(12.0),
        );
        pen.round(
            dot,
            dot.w / 2.0,
            rgb(Hsv {
                v: c.v.max(160),
                ..c
            }
            .to_rgb()),
        )?;
        let label = Area::new(
            dot.right() + pen.s(8.0),
            a.y,
            a.right() - dot.right() - pen.s(12.0),
            a.h,
        );
        pen.fitted_left(
            lang.tr(name),
            label,
            11.0,
            8.0,
            if on { color::TEXT } else { color::DIM },
        )?;
        hits.push((a, hit));
    }
    Ok(())
}

fn codes(
    pen: &mut Pen<'_>,
    area: Area,
    now: Rgb,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let (x, w) = (area.x, area.w);
    let swatch = Area::new(x, area.y, w, pen.s(34.0));
    pen.round(swatch, pen.s(8.0), rgb(now))?;
    pen.outline(swatch, pen.s(8.0), pen.s(1.0), color::BORDER)?;
    let mut y = swatch.bottom() + pen.s(10.0);
    field(
        pen,
        Area::new(x, y, w, pen.s(28.0)),
        Field::Hex,
        now,
        shared,
        hits,
    )?;
    y += pen.s(36.0);
    let fw = (w - pen.s(12.0)) / 3.0;
    for (i, (f, name)) in [(Field::Red, "R"), (Field::Green, "G"), (Field::Blue, "B")]
        .into_iter()
        .enumerate()
    {
        #[allow(clippy::cast_precision_loss)]
        let a = Area::new(
            x + (fw + pen.s(6.0)) * i as f32,
            y + pen.s(14.0),
            fw,
            pen.s(28.0),
        );
        pen.text(name, a.x + pen.s(2.0), y, &pen.bold(10.0), color::FAINT)?;
        field(pen, a, f, now, shared, hits)?;
    }
    y += pen.s(52.0);
    let dot = pen.s(20.0);
    let per_row = ((w + pen.s(6.0)) / (dot + pen.s(6.0))).floor().max(1.0);
    let mut drawn = 0.0;
    for s in SWATCHES {
        let col = drawn % per_row;
        let row = (drawn / per_row).floor();
        let a = Area::new(
            x + col * (dot + pen.s(6.0)),
            y + row * (dot + pen.s(6.0)),
            dot,
            dot,
        );
        if a.bottom() > area.bottom() {
            break;
        }
        swatch_dot(pen, a, s, now, hits)?;
        drawn += 1.0;
    }
    Ok(())
}

fn swatch_dot(pen: &mut Pen<'_>, a: Area, c: Rgb, now: Rgb, hits: &mut Hits) -> AureaResult<()> {
    let hit = Hit::Swatch(c);
    let t = pen.hover(a, hit);
    let grow = pen.s(2.0) * t;
    let r = a.w / 2.0 + grow;
    let (cx, cy) = (a.x + a.w / 2.0, a.y + a.h / 2.0);
    if c == now {
        pen.circle(cx, cy, r + pen.s(3.0), color::TEXT)?;
        pen.circle(cx, cy, r + pen.s(1.5), color::SURFACE)?;
    }
    pen.circle(cx, cy, r, rgb(c))?;
    hits.push((a, hit));
    Ok(())
}

fn field(
    pen: &mut Pen<'_>,
    a: Area,
    f: Field,
    now: Rgb,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let hit = Hit::Field(f);
    let editing = shared.ui.editing.as_ref().filter(|(e, _)| *e == f);
    let t = pen.hover(a, hit);
    pen.round(
        a,
        pen.s(6.0),
        color::mix(color::BACKGROUND, color::RAISED, t * 0.6),
    )?;
    let text = match editing {
        Some((_, typed)) => {
            let caret = (pen.anim.time() * 2.0).fract() < 0.5;
            format!("{typed}{}", if caret { "|" } else { " " })
        }
        None => f.text(now),
    };
    if editing.is_some() {
        pen.outline(a, pen.s(6.0), pen.s(1.5), color::ACCENT)?;
    }
    let inner = Area::new(a.x + pen.s(8.0), a.y, a.w - pen.s(12.0), a.h);
    pen.fitted_left(&text, inner, 13.0, 8.0, color::TEXT)?;
    hits.push((a, hit));
    Ok(())
}

fn side_card(
    pen: &mut Pen<'_>,
    area: Area,
    shared: &Shared,
    settings: Settings,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let mut y = area.y;
    let e = settings.effect;
    let sliders = [
        (true, Slider::Brightness, "BRIGHTNESS", settings.color.v),
        (e.moves(), Slider::Speed, "SPEED", settings.speed),
        (
            has_background(e),
            Slider::Background,
            "BACKGROUND",
            settings.background,
        ),
        (has_size(e), Slider::Size, "SIZE", settings.size),
    ];
    for (shown, which, name, value) in sliders {
        if !shown {
            continue;
        }
        slider(
            pen,
            Area::new(area.x, y, area.w, pen.s(36.0)),
            (which, name, value),
            settings,
            shared,
            hits,
        )?;
        y += pen.s(44.0);
    }
    if settings.effect == Effect::PerKey {
        let items = [
            (lang.tr("Paint all").to_string(), Hit::PaintAll, false),
            (lang.tr("Clear all").to_string(), Hit::ClearAll, false),
        ];
        widgets::pills(pen, area.x, y, &items, hits)?;
        y += pen.s(40.0);
        if !shared.ui.recent.is_empty() {
            widgets::label(pen, lang.tr("RECENT"), area.x, y)?;
            y += pen.s(18.0);
            let dot = pen.s(18.0);
            let now = shared.picked().to_rgb();
            for (i, c) in shared.ui.recent.iter().enumerate() {
                #[allow(clippy::cast_precision_loss)]
                let a = Area::new(area.x + (dot + pen.s(6.0)) * i as f32, y, dot, dot);
                if a.right() > area.right() {
                    break;
                }
                swatch_dot(pen, a, *c, now, hits)?;
            }
            y += pen.s(30.0);
        }
    }
    let bottom = area.bottom() - pen.s(104.0);
    if bottom > y {
        widgets::label(
            pen,
            lang.tr("LIGHT THE KEYS OF A HELD LAYER"),
            area.x,
            bottom,
        )?;
        segmented(
            pen,
            Area::new(area.x, bottom + pen.s(16.0), area.w, pen.s(28.0)),
            (
                &[
                    (lang.tr("Off"), Hit::LayerKeys(false)),
                    (lang.tr("On"), Hit::LayerKeys(true)),
                ],
                2,
            ),
            usize::from(settings.layer_keys),
            hits,
        )?;
    }
    let bottom = area.bottom() - pen.s(50.0);
    if bottom > y {
        widgets::label(pen, lang.tr("WHO CONTROLS THE LIGHTS"), area.x, bottom)?;
        let seg = Area::new(area.x, bottom + pen.s(16.0), area.w, pen.s(32.0));
        segmented(
            pen,
            seg,
            (
                &[
                    (lang.tr("Keyboard effects"), Hit::OsLighting(false)),
                    (lang.tr("Windows Dynamic Lighting"), Hit::OsLighting(true)),
                ],
                0,
            ),
            usize::from(settings.os_lighting),
            hits,
        )?;
    }
    Ok(())
}

fn slider(
    pen: &mut Pen<'_>,
    area: Area,
    (which, name, value): (Slider, &'static str, u8),
    settings: Settings,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    // For the press effects, speed is how long a press shows.
    let text =
        if which == Slider::Speed && matches!(settings.effect, Effect::Reactive | Effect::Ripple) {
            format!("{:.1} s", f64::from(press_life(value).0) / 1000.0)
        } else {
            widgets::percent(f32::from(value) / 255.0)
        };
    let (h, s) = (settings.color.h, settings.color.s);
    let shade = |v: u8| match which {
        Slider::Brightness => Hsv::new(h, s, v).to_rgb(),
        Slider::Speed | Slider::Size => Hsv::new(0, 0, 60 + v / 3).to_rgb(),
        Slider::Background => Hsv::new(h, s, v / 2).to_rgb(),
    };
    widgets::slider(
        pen,
        area,
        (lang.tr(name), &text),
        (
            f32::from(value) / 255.0,
            Hit::Slider(which),
            Track::Shades(&shade),
        ),
        shared,
        hits,
    )
}
