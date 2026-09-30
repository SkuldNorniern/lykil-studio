//! The lighting page for a keyboard that speaks only VIA. It is the Lykil
//! lighting page with VIA's values in it: the keyboard in the chosen
//! colour, the effects as cards, the colour square and the sliders. A
//! definition with more than one section (backlight and underglow) gets a
//! switch between them.

use aurea::AureaResult;
use lykil::lighting::{Effect, Hsv, Settings};
use lykil_qmk::import::ViaControlKind;

use crate::app::{Hit, Shared};
use crate::components::surface::panel;
use crate::components::text::label;
use crate::components::track::Track;
use crate::devices::via::{ViaGroup, ViaSetting, groups};
use crate::draw::{Area, Hits, Pen, color};
use crate::effects::{about as effect_about, points};
use crate::format::percent;
use crate::keyboard::{self, Keys, Presses};
use crate::widgets::colour::picker;
use crate::widgets::effects::{EffectCard, Strips, effect_grid};
use crate::widgets::layout::lighting_panels;
use crate::widgets::pills::pill_flow;
use crate::widgets::segmented::{labelled, segmented};
use crate::widgets::slider::slider;
use crate::widgets::status::status_line;

pub fn tab(pen: &mut Pen<'_>, body: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<String> {
    let lang = pen.lang;
    let kb = &shared.keyboard;
    let settings = &kb.via_settings;
    let all = groups(settings);
    let Some(group) = all.get(shared.ui.via_group.min(all.len().saturating_sub(1))) else {
        pen.centred(
            lang.tr("This keyboard's VIA definition has no lighting menu"),
            body,
            &pen.font(15.0),
            color::DIM,
        )?;
        return Ok(String::new());
    };
    let get = |i: Option<usize>| i.and_then(|i| settings.get(i));
    let look = look(group, settings);

    let points = kb
        .description
        .as_ref()
        .map(|d| points(d, false))
        .unwrap_or_default();
    let presses = Presses {
        at: &[],
        recent: Vec::new(),
        heat: Vec::new(),
    };
    let kb_area = Area::new(body.x, body.y + pen.s(4.0), body.w, body.h * 0.44);
    let used = keyboard::keyboard(
        pen,
        kb_area,
        kb,
        &Keys::Lighting {
            settings: look,
            time: pen.anim.time(),
            colors: &[],
            points: &points,
            presses: &presses,
            brush: None,
        },
        true,
        hits,
    )?;
    let line = Area::new(body.x, used.bottom() + pen.s(10.0), body.w, pen.s(18.0));
    status_line(
        pen,
        line,
        lang.tr("The keyboard runs this effect. The picture shows its colour, VIA effects are not played here."),
        color::GOOD,
    )?;

    let top = line.bottom() + pen.s(10.0);
    let rest = Area::new(body.x, top, body.w, body.bottom() - top);
    let (effects, colours, side) = lighting_panels(pen, rest, pen.s(16.0));
    for a in [effects, colours, side] {
        panel(pen, a)?;
    }
    effect_cards(pen, effects.inset(pen.s(16.0)), (group, look), shared, hits)?;
    picker_card(
        pen,
        colours.inset(pen.s(16.0)),
        look.color,
        (get(group.color), group.color),
        hits,
    )?;
    side_card(pen, side.inset(pen.s(16.0)), (&all, group), shared, hits)?;
    Ok(lang
        .tr("VIA settings from the keyboard's definition. Saved on the keyboard when you let go.")
        .into())
}

/// What the preview shows: the section's colour at its brightness and
/// speed, dark when its effect is off.
fn look(group: &ViaGroup, settings: &[ViaSetting]) -> Settings {
    let get = |i: Option<usize>| i.and_then(|i| settings.get(i));
    let (h, s) =
        get(group.color).map_or((0, 0), |c| (c.byte(), c.value.get(1).copied().unwrap_or(0)));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let v = get(group.brightness).map_or(255, |b| (b.fraction() * 255.0).round() as u8);
    let off = get(group.effect).is_some_and(|e| e.byte() == 0);
    Settings {
        effect: if off { Effect::Off } else { Effect::Solid },
        color: Hsv::new(h, s, v),
        speed: get(group.speed).map_or(Settings::DEFAULT.speed, ViaSetting::byte),
        ..Settings::DEFAULT
    }
}

/// QMK effects by a word in their VIA name, first match wins: a line about
/// them and the Lykil effect closest to them, which their card plays.
const FAMILIES: [(&str, &str, Effect); 15] = [
    ("off", "", Effect::Off),
    ("reactive", "Pressed keys light up", Effect::Reactive),
    ("splash", "Waves from pressed keys", Effect::Ripple),
    ("heatmap", "", Effect::Heatmap),
    ("rain", "", Effect::Rain),
    ("twinkle", "", Effect::Starlight),
    ("pixel", "Keys change at random", Effect::Starlight),
    ("breathing", "", Effect::Breathing),
    ("mood", "", Effect::Cycle),
    ("cycle all", "", Effect::Cycle),
    ("test", "Steps through red, green and blue", Effect::Cycle),
    ("alphas", "Letters and mods in two colours", Effect::Solid),
    ("solid", "", Effect::Solid),
    ("christmas", "Red and green", Effect::Wave),
    ("", "Colours moving across", Effect::Wave),
];

/// `name`'s line and closest Lykil effect, from [`FAMILIES`].
fn family(name: &str) -> (&'static str, Effect) {
    let name = name.to_lowercase();
    FAMILIES
        .iter()
        .find(|(word, ..)| name.contains(word))
        .map_or(("", Effect::Wave), |&(_, about, e)| {
            (
                if about.is_empty() {
                    effect_about(e)
                } else {
                    about
                },
                e,
            )
        })
}

fn effect_cards(
    pen: &mut Pen<'_>,
    area: Area,
    (group, look): (&ViaGroup, Settings),
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let settings = &shared.keyboard.via_settings;
    label(pen, lang.tr("EFFECT"), area.x, area.y)?;
    let grid = Area::new(area.x, area.y + pen.s(20.0), area.w, area.h - pen.s(20.0));
    let Some((index, setting)) = group.effect.and_then(|i| Some((i, settings.get(i)?))) else {
        pen.fitted_left(
            lang.tr("This section has no effects to pick"),
            Area::new(grid.x, grid.y, grid.w, pen.s(16.0)),
            11.0,
            8.0,
            color::FAINT,
        )?;
        return Ok(());
    };
    let ViaControlKind::Dropdown(options) = &setting.control.kind else {
        return Ok(());
    };
    let looks: Vec<(&str, Effect)> = options.iter().map(|(name, _)| family(name)).collect();
    // A dozen QMK names start "Solid Reactive"; the rest of it is what
    // tells them apart on a narrow card.
    let names: Vec<String> = options
        .iter()
        .map(|(name, _)| match name.strip_prefix("Solid Reactive") {
            Some(rest) => format!("Reactive{rest}"),
            None => name.clone(),
        })
        .collect();
    let cards: Vec<EffectCard<'_>> = options
        .iter()
        .zip(&looks)
        .zip(&names)
        .map(|(((_, value), (about, e)), name)| EffectCard {
            name,
            about: lang.tr(about),
            hit: Hit::ViaOption(index, *value),
            active: *value == setting.byte(),
            plays: (look, *e),
        })
        .collect();
    let strips = Strips {
        time: pen.anim.time(),
        key_colors: &[],
    };
    effect_grid(pen, grid, &cards, (shared.ui.effect_page, strips), hits)
}

fn picker_card(
    pen: &mut Pen<'_>,
    area: Area,
    c: Hsv,
    (colour, index): (Option<&ViaSetting>, Option<usize>),
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    label(pen, lang.tr("COLOUR"), area.x, area.y)?;
    let (Some(_), Some(index)) = (colour, index) else {
        pen.fitted_left(
            lang.tr("This section has no colour setting"),
            Area::new(area.x, area.y + pen.s(20.0), area.w, pen.s(16.0)),
            11.0,
            8.0,
            color::FAINT,
        )?;
        return Ok(());
    };
    let top = area.y + pen.s(20.0);
    let side = (area.bottom() - top - pen.s(30.0))
        .min(pen.s(170.0))
        .max(pen.s(60.0));
    let square = Area::new(area.x, top, side, side);
    picker(
        pen,
        square,
        c,
        (Hit::ViaSquare(index), Hit::ViaHue(index)),
        hits,
    )
}

/// The colour each step of a slider's track shows.
type Shade<'a> = &'a dyn Fn(u8) -> lykil::lighting::Rgb;

/// The section switch, the brightness and speed sliders, and whatever
/// else the section has.
fn side_card(
    pen: &mut Pen<'_>,
    area: Area,
    (all, group): (&[ViaGroup], &ViaGroup),
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let settings = &shared.keyboard.via_settings;
    let mut y = area.y;
    if all.len() > 1 {
        let names: Vec<(&str, Hit)> = all
            .iter()
            .enumerate()
            .map(|(i, g)| (g.name.as_str(), Hit::ViaGroup(i)))
            .collect();
        let chosen = all.iter().position(|g| g == group).unwrap_or(0);
        segmented(
            pen,
            Area::new(area.x, y, area.w, pen.s(30.0)),
            (&names, 5000),
            chosen,
            hits,
        )?;
        y += pen.s(44.0);
    }
    let hue = group
        .color
        .and_then(|i| settings.get(i))
        .map_or((0, 0), |c| (c.byte(), c.value.get(1).copied().unwrap_or(0)));
    let bright = |v: u8| Hsv::new(hue.0, hue.1, v).to_rgb();
    let grey = |v: u8| Hsv::new(0, 0, 60 + v / 3).to_rgb();
    let named: [(Option<usize>, &str, Shade<'_>); 2] = [
        (group.brightness, "BRIGHTNESS", &bright),
        (group.speed, "SPEED", &grey),
    ];
    for (index, name, shade) in named {
        let Some((i, s)) = index.and_then(|i| Some((i, settings.get(i)?))) else {
            continue;
        };
        let t = s.fraction();
        let row = Area::new(area.x, y, area.w, pen.s(36.0));
        slider(
            pen,
            row,
            (lang.tr(name), &percent(t)),
            (t, Hit::ViaRange(i), Track::Shades(shade)),
            shared,
            hits,
        )?;
        y += pen.s(44.0);
    }
    for &i in &group.other {
        let Some(s) = settings.get(i) else {
            continue;
        };
        let row = Area::new(area.x, y, area.w, area.bottom() - y);
        y += other(pen, row, (i, s), shared, hits)? + pen.s(12.0);
    }
    Ok(())
}

/// A setting that is not brightness, effect, speed or colour, drawn by its
/// kind. Returns the height it took.
fn other(
    pen: &mut Pen<'_>,
    area: Area,
    (index, s): (usize, &ViaSetting),
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<f32> {
    let lang = pen.lang;
    let name = s.control.label.to_uppercase();
    match &s.control.kind {
        ViaControlKind::Range { .. } => {
            let t = s.fraction();
            let row = Area::new(area.x, area.y, area.w, pen.s(36.0));
            slider(
                pen,
                row,
                (&name, &percent(t)),
                (t, Hit::ViaRange(index), Track::Plain),
                shared,
                hits,
            )?;
            Ok(pen.s(36.0))
        }
        ViaControlKind::Toggle => {
            let items = [
                (lang.tr("Off"), Hit::ViaToggle(index, false)),
                (lang.tr("On"), Hit::ViaToggle(index, true)),
            ];
            labelled(
                pen,
                (area.x, area.y, area.w.min(pen.s(220.0)), pen.s(30.0)),
                &name,
                (&items, 3000 + index),
                usize::from(s.byte() != 0),
                hits,
            )
        }
        ViaControlKind::Dropdown(options) => {
            label(pen, &name, area.x, area.y)?;
            let items: Vec<(String, Hit, bool)> = options
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        Hit::ViaOption(index, *value),
                        *value == s.byte(),
                    )
                })
                .collect();
            let flow = Area::new(area.x, area.y + pen.s(20.0), area.w, area.h);
            Ok(pen.s(20.0) + pill_flow(pen, flow, &items, hits)?)
        }
        // A second colour in one section is rare; the first one is in the
        // picker.
        ViaControlKind::Color => Ok(0.0),
    }
}
