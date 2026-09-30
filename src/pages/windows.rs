//! The Windows page: whether Windows may take this keyboard's LEDs, and
//! the other Dynamic Lighting devices on the desk.

use aurea::AureaResult;
use aurea::render::Color;

use crate::app::{Hit, Shared};
use crate::components::colour::hue_strip;
use crate::components::dot;
use crate::components::surface::panel;
use crate::components::text::{heading, hint, label};
use crate::components::track::Track;
use crate::devices::Connection;
use crate::draw::{Area, Hits, Pen, color};
use crate::effects;
use crate::format::percent;
use crate::lamps::Lamp;
use crate::widgets::desk::desk;
use crate::widgets::effects::{EffectCard, Strips, effect_grid};
use crate::widgets::pills::{pills, pills_to};
use crate::widgets::segmented::labelled;
use crate::widgets::slider::slider;
use crate::widgets::status::status_line;

pub fn tab(pen: &mut Pen<'_>, body: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<String> {
    let width = pen.s(760.0).min(body.w);
    let x0 = body.x + (body.w - width) / 2.0;
    let keyboard = Area::new(x0, body.y + pen.s(16.0), width, pen.s(170.0));
    this_keyboard(pen, keyboard, shared, hits)?;
    let others = Area::new(
        x0,
        keyboard.bottom() + pen.s(16.0),
        width,
        (body.bottom() - keyboard.bottom() - pen.s(24.0)).min(pen.s(470.0)),
    );
    other_devices(pen, others, shared, hits)?;
    Ok(pen
        .lang
        .tr("Studio lights other devices only while it is the window in front; Windows keeps background control for packaged apps.")
        .into())
}

fn this_keyboard(
    pen: &mut Pen<'_>,
    card: Area,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    panel(pen, card)?;
    let x = card.x + pen.s(24.0);
    let w = card.w - pen.s(48.0);
    let mut y = card.y + pen.s(20.0);
    heading(pen, lang.tr("Windows Dynamic Lighting"), x, y)?;
    y += pen.s(30.0);
    pen.fitted_left(
        lang.tr("Your Lykil keyboard is a Dynamic Lighting device: Windows can light it together with your other devices."),
        Area::new(x, y, w, pen.s(18.0)),
        12.0,
        8.0,
        color::DIM,
    )?;
    y += pen.s(34.0);
    label(pen, lang.tr("THIS KEYBOARD"), x, y)?;
    y += pen.s(18.0);
    let connected = shared.keyboard.connection == Connection::Connected;
    let Some(s) = shared.lighting().filter(|_| connected) else {
        let why = if connected {
            lang.tr("This keyboard has no lighting")
        } else {
            lang.tr("Connect the keyboard to change how it shares its LEDs.")
        };
        return hint(pen, Area::new(x, y, w, pen.s(30.0)), why);
    };
    let (state, tone, flip) = if s.os_lighting {
        (
            lang.tr("Windows may take the LEDs"),
            color::GOOD,
            (
                lang.tr("Keep the keyboard's effect"),
                Hit::OsLighting(false),
                false,
            ),
        )
    } else {
        (
            lang.tr("The keyboard keeps its own effect"),
            color::DIM,
            (
                lang.tr("Let Windows take the LEDs"),
                Hit::OsLighting(true),
                true,
            ),
        )
    };
    status_line(pen, Area::new(x, y, w * 0.5, pen.s(30.0)), state, tone)?;
    let flip = [(flip.0.to_string(), flip.1, flip.2)];
    pills_to(pen, card.right() - pen.s(24.0), y, &flip, hits)?;
    let room = Area::new(x, y + pen.s(36.0), w, pen.s(14.0));
    hint(pen, room, lang.tr(crate::pages::lighting::RECONNECTS))
}

fn other_devices(
    pen: &mut Pen<'_>,
    card: Area,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    panel(pen, card)?;
    let x = card.x + pen.s(24.0);
    let w = card.w - pen.s(48.0);
    let mut y = card.y + pen.s(20.0);
    label(pen, lang.tr("OTHER DEVICES"), x, y)?;
    let open = [(
        lang.tr("Open Dynamic Lighting settings").to_string(),
        Hit::LightingSettings,
        false,
    )];
    pills_to(pen, card.right() - pen.s(24.0), y - pen.s(6.0), &open, hits)?;
    y += pen.s(32.0);
    let sync = [
        (
            lang.tr("Leave them to Windows").to_string(),
            Hit::LampSync(false),
            !shared.lamp_sync,
        ),
        (
            lang.tr("Light them with the keyboard's effect").to_string(),
            Hit::LampSync(true),
            shared.lamp_sync,
        ),
    ];
    pills(pen, x, y, &sync, hits)?;
    y += pen.s(44.0);

    if shared.lamps.is_empty() {
        return hint(
            pen,
            Area::new(x, y, w, pen.s(20.0)),
            lang.tr("No Dynamic Lighting devices found."),
        );
    }
    let rest = Area::new(x, y, w, card.bottom() - y - pen.s(20.0));
    let plane_w = (rest.w * 0.58).max(pen.s(200.0));
    let plane = Area::new(rest.x, rest.y, plane_w, rest.h);
    desk(pen, plane, shared, hits)?;
    let side = Area::new(
        plane.right() + pen.s(16.0),
        rest.y,
        rest.right() - plane.right() - pen.s(16.0),
        rest.h,
    );
    picked(pen, side, shared, hits)
}

/// Who lights device `d` right now, and how that reads.
fn who_lights(d: &Lamp, sync: bool) -> (&'static str, Color) {
    if !sync {
        ("Windows controls it", color::DIM)
    } else if !d.place.follow {
        ("Not following: Windows controls it", color::DIM)
    } else if !d.open {
        ("Waiting for Windows to hand it over", color::PRESSED)
    } else if !d.available {
        (
            "Waiting: Studio has to be the window in front",
            color::PRESSED,
        )
    } else {
        ("Studio lights it", color::GOOD)
    }
}

/// The picked device's brightness and effect, then every device with its
/// follow switch.
fn picked(pen: &mut Pen<'_>, area: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<()> {
    let lang = pen.lang;
    let mut y = area.y;
    if let Some(d) = shared
        .lamps
        .iter()
        .find(|d| shared.ui.desk_selected.as_deref() == Some(d.id.as_str()))
    {
        pen.fitted_left(
            &d.name,
            Area::new(area.x, y, area.w, pen.s(18.0)),
            13.0,
            9.0,
            color::TEXT,
        )?;
        y += pen.s(24.0);
        let level = f32::from(d.place.level) / 255.0;
        slider(
            pen,
            Area::new(area.x, y, area.w, pen.s(34.0)),
            (lang.tr("BRIGHTNESS"), &percent(level)),
            (level, Hit::DeviceLevel, Track::Plain),
            shared,
            hits,
        )?;
        y += pen.s(46.0);
        y += own_effect(
            pen,
            Area::new(area.x, y, area.w, area.bottom() - y),
            d,
            shared,
            hits,
        )?;
        if !shared.lamp_sync {
            let line = Area::new(area.x, y - pen.s(12.0), area.w, pen.s(14.0));
            hint(pen, line, lang.tr("Used while Studio lights it."))?;
            y += pen.s(10.0);
        }
    } else {
        let line = Area::new(area.x, y, area.w, pen.s(18.0));
        hint(
            pen,
            line,
            lang.tr("Click a device on the desk to set it up"),
        )?;
        y += pen.s(28.0);
    }
    let row_h = pen.s(42.0);
    for (i, d) in shared.lamps.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let row = Area::new(area.x, y + row_h * i as f32, area.w, row_h - pen.s(4.0));
        if row.bottom() > area.bottom() {
            break;
        }
        device_row(pen, row, (i, d), shared.lamp_sync, hits)?;
    }
    Ok(())
}

/// A device's name, who lights it, and its follow switch.
fn device_row(
    pen: &mut Pen<'_>,
    row: Area,
    (i, d): (usize, &Lamp),
    sync: bool,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let (state, tone) = who_lights(d, sync);
    dot::status(pen, row.x, row.y + row.h / 2.0, tone)?;
    let (text, on) = if d.place.follow {
        (lang.tr("Following"), true)
    } else {
        (lang.tr("Follow"), false)
    };
    let pill = [(text.to_string(), Hit::LampFollow(i), on)];
    let start = pills_to(pen, row.right(), row.y + pen.s(2.0), &pill, hits)?;
    let x = row.x + pen.s(16.0);
    let text_w = start - pen.s(10.0) - x;
    let name = Area::new(x, row.y + pen.s(2.0), text_w, pen.s(16.0));
    pen.fitted_left(&d.name, name, 12.0, 9.0, color::TEXT)?;
    let sub = Area::new(name.x, row.y + pen.s(19.0), text_w, pen.s(13.0));
    pen.fitted_left(lang.tr(state), sub, 10.0, 8.0, tone)
}

/// The picked device's effect: the keyboard's, or its own with effect,
/// colour and speed. Returns the height it took.
fn own_effect(
    pen: &mut Pen<'_>,
    area: Area,
    d: &Lamp,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<f32> {
    let lang = pen.lang;
    let items = [
        (lang.tr("The keyboard's"), Hit::DeviceOwn(false)),
        (lang.tr("Its own"), Hit::DeviceOwn(true)),
    ];
    let used = labelled(
        pen,
        (area.x, area.y, area.w, pen.s(30.0)),
        lang.tr("EFFECT"),
        (&items, 4000),
        usize::from(d.place.own.is_some()),
        hits,
    )?;
    let mut y = area.y + used + pen.s(12.0);
    let Some(own) = d.place.own else {
        return Ok(y - area.y + pen.s(6.0));
    };
    // The same cards as the lighting page, running the device's own look.
    let cards: Vec<EffectCard<'_>> = crate::lamps::OWN_EFFECTS
        .iter()
        .map(|e| EffectCard {
            name: lang.tr(effects::name(*e)),
            about: "",
            hit: Hit::DeviceEffect(*e),
            active: *e == own.effect,
            runs: true,
            plays: (own, *e),
        })
        .collect();
    let grid = Area::new(area.x, y, area.w, pen.s(130.0));
    let strips = Strips {
        time: pen.anim.time(),
        key_colors: &shared.keyboard.key_colors,
    };
    effect_grid(pen, grid, &cards, (0, strips), hits)?;
    y = grid.bottom() + pen.s(14.0);
    label(pen, lang.tr("COLOUR"), area.x, y)?;
    let hues = Area::new(area.x, y + pen.s(20.0), area.w, pen.s(12.0));
    hue_strip(pen, hues, own.color.h, Hit::DeviceHue, hits)?;
    y = hues.bottom() + pen.s(14.0);
    if own.effect != lykil::lighting::Effect::Solid {
        let speed = f32::from(own.speed) / 255.0;
        slider(
            pen,
            Area::new(area.x, y, area.w, pen.s(34.0)),
            (lang.tr("SPEED"), &percent(speed)),
            (speed, Hit::DeviceSpeed, Track::Plain),
            shared,
            hits,
        )?;
        y += pen.s(46.0);
    }
    Ok(y - area.y)
}
