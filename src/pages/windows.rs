//! The Windows lighting page.

use aurea::AureaResult;

use crate::app::{Hit, Shared};
use crate::devices::Connection;
use crate::draw::Hits;
use crate::draw::{Area, Pen, color};
use crate::widgets;

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
    pen.round(card, pen.s(14.0), color::SURFACE)?;
    let x = card.x + pen.s(24.0);
    let w = card.w - pen.s(48.0);
    let mut y = card.y + pen.s(20.0);
    pen.text(
        lang.tr("Windows Dynamic Lighting"),
        x,
        y,
        &pen.bold(17.0),
        color::TEXT,
    )?;
    y += pen.s(30.0);
    pen.fitted_left(
        lang.tr("Your Lykil keyboard is a Dynamic Lighting device: Windows can light it together with your other devices."),
        Area::new(x, y, w, pen.s(18.0)),
        12.0,
        8.0,
        color::DIM,
    )?;
    y += pen.s(34.0);
    widgets::label(pen, lang.tr("THIS KEYBOARD"), x, y)?;
    y += pen.s(18.0);
    let connected = shared.keyboard.connection == Connection::Connected;
    let Some(s) = shared.lighting().filter(|_| connected) else {
        let why = if connected {
            lang.tr("This keyboard has no lighting")
        } else {
            lang.tr("Connect the keyboard to change how it shares its LEDs.")
        };
        return pen.fitted_left(
            why,
            Area::new(x, y, w, pen.s(30.0)),
            13.0,
            9.0,
            color::FAINT,
        );
    };
    let (state, dot) = if s.os_lighting {
        (lang.tr("Windows may take the LEDs"), color::GOOD)
    } else {
        (lang.tr("The keyboard keeps its own effect"), color::DIM)
    };
    pen.circle(x + pen.s(5.0), y + pen.s(15.0), pen.s(4.0), dot)?;
    pen.text(
        state,
        x + pen.s(18.0),
        y + pen.s(7.0),
        &pen.font(13.0),
        color::TEXT,
    )?;
    let flip = if s.os_lighting {
        (
            lang.tr("Keep the keyboard's effect").to_string(),
            Hit::OsLighting(false),
            false,
        )
    } else {
        (
            lang.tr("Let Windows take the LEDs").to_string(),
            Hit::OsLighting(true),
            true,
        )
    };
    let font = pen.bold(12.0);
    let bw = pen.width(&flip.0, &font) + pen.s(24.0);
    widgets::pills(pen, card.right() - pen.s(24.0) - bw, y, &[flip], hits)?;
    Ok(())
}

fn other_devices(
    pen: &mut Pen<'_>,
    card: Area,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    pen.round(card, pen.s(14.0), color::SURFACE)?;
    let x = card.x + pen.s(24.0);
    let w = card.w - pen.s(48.0);
    let mut y = card.y + pen.s(20.0);
    widgets::label(pen, lang.tr("OTHER DEVICES"), x, y)?;
    let open = [(
        lang.tr("Open Dynamic Lighting settings").to_string(),
        Hit::LightingSettings,
        false,
    )];
    let font = pen.bold(12.0);
    let ow = pen.width(&open[0].0, &font) + pen.s(24.0);
    widgets::pills(
        pen,
        card.right() - pen.s(24.0) - ow,
        y - pen.s(6.0),
        &open,
        hits,
    )?;
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
    widgets::pills(pen, x, y, &sync, hits)?;
    y += pen.s(44.0);

    if shared.lamps.is_empty() {
        pen.fitted_left(
            lang.tr("No Dynamic Lighting devices found."),
            Area::new(x, y, w, pen.s(20.0)),
            13.0,
            9.0,
            color::FAINT,
        )?;
        return Ok(());
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
    picked(pen, side, shared, hits)?;
    Ok(())
}

/// The desk: every device where it sits, its lamps in their live colours.
/// Drag a device to move it.
fn desk(pen: &mut Pen<'_>, area: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<()> {
    let lang = pen.lang;
    pen.round(area, pen.s(10.0), color::BACKGROUND)?;
    let (mut left, mut top, mut right, mut bottom) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for d in &shared.lamps {
        left = left.min(d.place.x);
        top = top.min(d.place.y);
        right = right.max(d.place.x + d.size.0);
        bottom = bottom.max(d.place.y + d.size.1);
    }
    let pad = pen.s(24.0);
    let scale = ((area.w - 2.0 * pad) / (right - left).max(0.05))
        .min((area.h - 2.0 * pad - pen.s(14.0)) / (bottom - top).max(0.05))
        .max(1.0);
    let ox = area.x + (area.w - (right - left) * scale) / 2.0;
    let oy = area.y + (area.h - (bottom - top) * scale) / 2.0;
    let settings = shared.lighting().filter(|_| shared.lamp_sync);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let now = lykil::time::Tick((pen.anim.time() * 1000.0) as u32);
    let colours = settings.map(|s| crate::lamps::colours(&shared.lamps, s, now));
    // The picked device last, so it is on top where devices overlap.
    let picked_id = shared.ui.desk_selected.as_deref();
    let mut order: Vec<usize> = (0..shared.lamps.len()).collect();
    order.sort_by_key(|&i| Some(shared.lamps[i].id.as_str()) == picked_id);
    for (i, d) in order.into_iter().map(|i| (i, &shared.lamps[i])) {
        let rect = Area::new(
            ox + (d.place.x - left) * scale,
            oy + (d.place.y - top) * scale,
            d.size.0 * scale,
            d.size.1 * scale,
        );
        let picked = shared.ui.desk_selected.as_deref() == Some(d.id.as_str());
        let hover = pen.hover(rect, Hit::DeskDevice(i));
        let face = if d.place.follow {
            color::RAISED
        } else {
            color::mix(color::BACKGROUND, color::RAISED, 0.5)
        };
        pen.round(rect, pen.s(6.0), color::mix(face, color::HOVER, hover))?;
        let crowded = shared.lamps.iter().any(|o| {
            o.id != d.id && crate::lamps::desk::overlaps((d.place, d.size), (o.place, o.size))
        });
        if crowded {
            pen.outline(rect, pen.s(6.0), pen.s(2.0), color::BAD)?;
        } else if picked {
            pen.outline(rect, pen.s(6.0), pen.s(2.0), color::ACCENT)?;
        }
        let dot = (pen.s(2.5) + scale * 0.004).min(pen.s(5.0));
        for (k, (lx, ly)) in d.lamps.iter().enumerate() {
            let c = colours
                .as_ref()
                .filter(|_| d.place.follow)
                .and_then(|c| c.get(i)?.get(k).copied())
                .map_or(color::FAINT, crate::widgets::colour::rgb);
            pen.circle(rect.x + lx * scale, rect.y + ly * scale, dot, c)?;
        }
        // Inside the device, so names never run into a neighbour.
        let label = Area::new(
            rect.x + pen.s(6.0),
            rect.y + pen.s(4.0),
            (rect.w - pen.s(12.0)).max(pen.s(10.0)),
            pen.s(12.0).min(rect.h),
        );
        pen.fitted_left(&d.name, label, 9.0, 6.0, color::DIM)?;
        hits.push((rect, Hit::DeskDevice(i)));
    }
    let hint = Area::new(
        area.x + pen.s(10.0),
        area.y + pen.s(6.0),
        area.w,
        pen.s(12.0),
    );
    pen.fitted_left(
        lang.tr("Drag the devices to where they sit"),
        hint,
        9.0,
        7.0,
        color::FAINT,
    )?;
    if !shared.lamp_sync {
        windows_banner(pen, area)?;
    }
    Ok(())
}

/// Across the bottom of the desk while Windows has the lights.
fn windows_banner(pen: &mut Pen<'_>, area: Area) -> AureaResult<()> {
    let lang = pen.lang;
    let bar = Area::new(
        area.x + pen.s(10.0),
        area.bottom() - pen.s(44.0),
        area.w - pen.s(20.0),
        pen.s(34.0),
    );
    pen.round(bar, pen.s(8.0), color::RAISED)?;
    pen.circle(
        bar.x + pen.s(16.0),
        bar.y + bar.h / 2.0,
        pen.s(4.0),
        color::DIM,
    )?;
    pen.fitted_left(
        lang.tr("Windows controls these lights now. Studio only shows where they sit."),
        Area::new(bar.x + pen.s(30.0), bar.y, bar.w - pen.s(40.0), bar.h),
        12.0,
        8.0,
        color::DIM,
    )
}

/// Who lights device `d` right now, and how that reads.
fn who_lights(d: &crate::lamps::Lamp, sync: bool) -> (&'static str, aurea::render::Color) {
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

/// The picked device's brightness, then every device with its follow
/// switch.
/// The picked device's effect: the keyboard's, or its own with effect,
/// colour and speed. Returns the height it took.
fn own_effect(
    pen: &mut Pen<'_>,
    area: Area,
    d: &crate::lamps::Lamp,
    shared: &Shared,
    hits: &mut Hits,
) -> AureaResult<f32> {
    let lang = pen.lang;
    widgets::label(pen, lang.tr("EFFECT"), area.x, area.y)?;
    let bar = Area::new(area.x, area.y + pen.s(18.0), area.w, pen.s(30.0));
    let items = [
        (lang.tr("The keyboard's"), Hit::DeviceOwn(false)),
        (lang.tr("Its own"), Hit::DeviceOwn(true)),
    ];
    widgets::segmented(
        pen,
        bar,
        (&items, 4000),
        usize::from(d.place.own.is_some()),
        hits,
    )?;
    let mut y = bar.bottom() + pen.s(12.0);
    let Some(own) = d.place.own else {
        return Ok(y - area.y + pen.s(6.0));
    };
    let chips: Vec<(String, Hit, bool)> = crate::lamps::OWN_EFFECTS
        .iter()
        .map(|e| {
            let name = lang.tr(crate::pages::lighting::effect_name(*e)).to_string();
            (name, Hit::DeviceEffect(*e), *e == own.effect)
        })
        .collect();
    y += widgets::chip_flow(pen, Area::new(area.x, y, area.w, area.h), &chips, hits)?;
    y += pen.s(14.0);
    widgets::label(pen, lang.tr("COLOUR"), area.x, y)?;
    let hues = Area::new(area.x, y + pen.s(20.0), area.w, pen.s(12.0));
    widgets::colour::hue_bar(pen, hues, own.color.h, Hit::DeviceHue, hits)?;
    y = hues.bottom() + pen.s(14.0);
    if own.effect != lykil::lighting::Effect::Solid {
        let row = Area::new(area.x, y, area.w, pen.s(34.0));
        let speed = f32::from(own.speed) / 255.0;
        widgets::slider(
            pen,
            row,
            lang.tr("SPEED"),
            speed,
            Hit::DeviceSpeed,
            shared,
            hits,
        )?;
        y += pen.s(46.0);
    }
    Ok(y - area.y)
}

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
        let row = Area::new(area.x, y, area.w, pen.s(34.0));
        let level = f32::from(d.place.level) / 255.0;
        widgets::slider(
            pen,
            row,
            lang.tr("BRIGHTNESS"),
            level,
            Hit::DeviceLevel,
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
            pen.fitted_left(
                lang.tr("Used while Studio lights it."),
                Area::new(area.x, y - pen.s(12.0), area.w, pen.s(14.0)),
                10.0,
                8.0,
                color::FAINT,
            )?;
            y += pen.s(10.0);
        }
    } else {
        pen.fitted_left(
            lang.tr("Click a device on the desk to set it up"),
            Area::new(area.x, y, area.w, pen.s(18.0)),
            11.0,
            8.0,
            color::FAINT,
        )?;
        y += pen.s(28.0);
    }
    let row_h = pen.s(42.0);
    let font = pen.bold(11.0);
    for (i, d) in shared.lamps.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let row = Area::new(area.x, y + row_h * i as f32, area.w, row_h - pen.s(4.0));
        if row.bottom() > area.bottom() {
            break;
        }
        let dot = who_lights(d, shared.lamp_sync).1;
        pen.circle(row.x + pen.s(5.0), row.y + row.h / 2.0, pen.s(3.5), dot)?;
        let (text, on) = if d.place.follow {
            (lang.tr("Following"), true)
        } else {
            (lang.tr("Follow"), false)
        };
        let fw = pen.width(text, &font) + pen.s(24.0);
        let text_w = row.w - fw - pen.s(26.0);
        let name = Area::new(row.x + pen.s(16.0), row.y + pen.s(2.0), text_w, pen.s(16.0));
        pen.fitted_left(&d.name, name, 12.0, 9.0, color::TEXT)?;
        let (state, tone) = who_lights(d, shared.lamp_sync);
        let sub = Area::new(name.x, row.y + pen.s(19.0), text_w, pen.s(13.0));
        pen.fitted_left(lang.tr(state), sub, 10.0, 8.0, tone)?;
        let pill = [(text.to_string(), Hit::LampFollow(i), on)];
        widgets::pills(pen, row.right() - fw, row.y + pen.s(2.0), &pill, hits)?;
    }
    Ok(())
}
