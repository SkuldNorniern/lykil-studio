//! The lighting page: the keyboard running the effect, then the effect
//! cards, the colour picker and the sliders.

use aurea::AureaResult;
use lykil::lighting::{Effect, Hsv, Palette, Rgb, Settings, press_life};

use crate::app::{Field, Hit, SWATCHES, Shared, Slider};
use crate::components::colour::sample;
use crate::components::dot::led;
use crate::components::field::input;
use crate::components::surface::{choice, panel};
use crate::components::text::label;
use crate::components::track::Track;
use crate::draw::{Area, Hits, Pen, color};
use crate::effects::{self, has_background, has_size, points};
use crate::format::percent;
use crate::keyboard::{self, Keys, Presses};
use crate::widgets::colour::{picker, swatches};
use crate::widgets::effects::{EffectCard, Strips, effect_grid};
use crate::widgets::layout::lighting_panels;
use crate::widgets::pills::pills;
use crate::widgets::segmented::{labelled, segmented};
use crate::widgets::slider::slider;
use crate::widgets::status::{overlay, status_line};

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
        let (text, button) = if settings.os_lighting {
            (
                lang.tr("Windows Dynamic Lighting has the LEDs, so the effect below does not run."),
                Some((lang.tr("Use keyboard effects"), Hit::OsLighting(false))),
            )
        } else {
            (
                lang.tr("An app has the LEDs; the effect comes back when it lets go."),
                None,
            )
        };
        overlay(pen, used, text, button, hits)?;
    }
    let line = Area::new(body.x, used.bottom() + pen.s(10.0), body.w, pen.s(18.0));
    let (text, tone) = who_runs_it(pen, shared, settings);
    status_line(pen, line, &text, tone)?;
    let top = line.bottom() + pen.s(10.0);
    let rest = Area::new(body.x, top, body.w, body.bottom() - top);
    let (effects, colours, side) = lighting_panels(pen, rest, pen.s(16.0));
    for a in [effects, colours, side] {
        panel(pen, a)?;
    }
    effect_cards(pen, effects.inset(pen.s(16.0)), settings, shared, hits)?;
    picker_card(pen, colours.inset(pen.s(16.0)), shared, hits)?;
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

/// Who drives the LEDs right now, and whether Studio lights other
/// devices with this effect too.
fn who_runs_it(
    pen: &Pen<'_>,
    shared: &Shared,
    settings: Settings,
) -> (String, aurea::render::Color) {
    let lang = pen.lang;
    let (text, tone) = match shared.keyboard.lighting {
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
    (text, tone)
}

fn effect_cards(
    pen: &mut Pen<'_>,
    area: Area,
    settings: Settings,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    label(pen, lang.tr("EFFECT"), area.x, area.y)?;
    let grid = Area::new(area.x, area.y + pen.s(20.0), area.w, area.h - pen.s(20.0));
    let cards: Vec<EffectCard<'_>> = Effect::ALL
        .into_iter()
        .map(|e| EffectCard {
            name: lang.tr(effects::name(e)),
            about: lang.tr(effects::about(e)),
            hit: Hit::Effect(e),
            active: e == settings.effect,
            plays: (settings, e),
        })
        .collect();
    let strips = Strips {
        time: pen.anim.time(),
        key_colors: &shared.keyboard.key_colors,
    };
    effect_grid(pen, grid, &cards, (0, strips), hits)
}

fn picker_card(pen: &mut Pen<'_>, area: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<()> {
    let lang = pen.lang;
    let brushing = shared.brushing();
    let picked = shared.picked();
    label(
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
    picker(pen, square, picked, (Hit::Square, Hit::HueBar), hits)?;
    let right = Area::new(
        square.right() + pen.s(16.0),
        top,
        area.right() - square.right() - pen.s(16.0),
        area.bottom() - top,
    );
    codes(pen, right, picked.to_rgb(), shared, hits)
}

/// Colour 1 and colour 2 as two small cards, the one being set picked.
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
        let fg = choice(pen, a, (hit, 900 + i), on)?;
        let d = a.h - pen.s(12.0);
        let dot = Area::new(a.x + pen.s(8.0), a.y + pen.s(6.0), d, d);
        let lit = Hsv {
            v: c.v.max(160),
            ..c
        };
        led(pen, dot, lit.to_rgb())?;
        let text = Area::new(
            dot.right() + pen.s(8.0),
            a.y,
            a.right() - dot.right() - pen.s(12.0),
            a.h,
        );
        pen.fitted_left(lang.tr(name), text, 11.0, 8.0, fg)?;
        hits.push((a, hit));
    }
    Ok(())
}

/// The colour as a swatch and as codes to type, then colours to click.
fn codes(
    pen: &mut Pen<'_>,
    area: Area,
    now: Rgb,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let (x, w) = (area.x, area.w);
    let swatch = Area::new(x, area.y, w, pen.s(34.0));
    sample(pen, swatch, now)?;
    let mut y = swatch.bottom() + pen.s(10.0);
    let field = |f: Field| {
        let typed = shared.ui.editing.as_ref().filter(|(e, _)| *e == f);
        (
            typed.map_or_else(|| f.text(now), |(_, t)| t.clone()),
            typed.is_some(),
            Hit::Field(f),
        )
    };
    let (text, editing, hit) = field(Field::Hex);
    input(
        pen,
        Area::new(x, y, w, pen.s(28.0)),
        &text,
        editing,
        hit,
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
        let (text, editing, hit) = field(f);
        input(pen, a, &text, editing, hit, hits)?;
    }
    y += pen.s(52.0);
    let rest = Area::new(x, y, w, area.bottom() - y);
    swatches(pen, rest, (&SWATCHES, 20.0), now, hits)?;
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
        setting_slider(
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
        pills(pen, area.x, y, &items, hits)?;
        y += pen.s(40.0);
        if !shared.ui.recent.is_empty() {
            label(pen, lang.tr("RECENT"), area.x, y)?;
            let row = Area::new(area.x, y + pen.s(18.0), area.w, pen.s(18.0));
            let now = shared.picked().to_rgb();
            swatches(pen, row, (&shared.ui.recent, 18.0), now, hits)?;
            y += pen.s(48.0);
        }
    }
    let bottom = area.bottom() - pen.s(104.0);
    if bottom > y {
        labelled(
            pen,
            (area.x, bottom, area.w, pen.s(28.0)),
            lang.tr("LIGHT THE KEYS OF A HELD LAYER"),
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
        labelled(
            pen,
            (area.x, bottom, area.w, pen.s(32.0)),
            lang.tr("WHO CONTROLS THE LIGHTS"),
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

/// One of the effect's settings as a slider, its track in the shades it
/// gives.
fn setting_slider(
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
            percent(f32::from(value) / 255.0)
        };
    let (h, s) = (settings.color.h, settings.color.s);
    let shade = |v: u8| match which {
        Slider::Brightness => Hsv::new(h, s, v).to_rgb(),
        Slider::Speed | Slider::Size => Hsv::new(0, 0, 60 + v / 3).to_rgb(),
        Slider::Background => Hsv::new(h, s, v / 2).to_rgb(),
    };
    slider(
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
