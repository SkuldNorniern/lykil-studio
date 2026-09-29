//! The Windows lighting page.

use aurea::AureaResult;

use crate::app::{Hit, Shared};
use crate::device::Connection;
use crate::draw::{Area, Pen, color};
use crate::view::{self, Hits};

pub fn tab(pen: &mut Pen<'_>, body: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<String> {
    let width = pen.s(760.0).min(body.w);
    let x0 = body.x + (body.w - width) / 2.0;
    let keyboard = Area::new(x0, body.y + pen.s(16.0), width, pen.s(170.0));
    this_keyboard(pen, keyboard, shared, hits)?;
    let others = Area::new(
        x0,
        keyboard.bottom() + pen.s(16.0),
        width,
        body.bottom() - keyboard.bottom() - pen.s(24.0),
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
    view::label(pen, lang.tr("THIS KEYBOARD"), x, y)?;
    y += pen.s(18.0);
    let connected = shared.keyboard.connection == Connection::Connected;
    match shared.lighting().filter(|_| connected) {
        Some(s) => {
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
            view::pills(pen, card.right() - pen.s(24.0) - bw, y, &[flip], hits)?;
        }
        None => {
            pen.fitted_left(
                lang.tr("Connect the keyboard to change how it shares its LEDs."),
                Area::new(x, y, w, pen.s(30.0)),
                13.0,
                9.0,
                color::FAINT,
            )?;
        }
    }
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
    view::label(pen, lang.tr("OTHER DEVICES"), x, y)?;
    let open = [(
        lang.tr("Open Dynamic Lighting settings").to_string(),
        Hit::LightingSettings,
        false,
    )];
    let font = pen.bold(12.0);
    let ow = pen.width(&open[0].0, &font) + pen.s(24.0);
    view::pills(
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
    view::pills(pen, x, y, &sync, hits)?;
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
    let row_h = pen.s(44.0);
    for (i, lamp) in shared.lamps.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let row = Area::new(
            x - pen.s(8.0),
            y + row_h * i as f32,
            w + pen.s(16.0),
            row_h - pen.s(4.0),
        );
        if row.bottom() > card.bottom() - pen.s(8.0) {
            break;
        }
        pen.round(row, pen.s(8.0), color::RAISED)?;
        let dot = if lamp.available {
            color::GOOD
        } else {
            color::FAINT
        };
        pen.circle(row.x + pen.s(16.0), row.y + row.h / 2.0, pen.s(4.0), dot)?;
        let name = Area::new(
            row.x + pen.s(30.0),
            row.y + pen.s(4.0),
            row.w * 0.5,
            pen.s(18.0),
        );
        pen.fitted_left(&lamp.name, name, 13.0, 9.0, color::TEXT)?;
        let about = if lamp.open {
            lang.fill(
                "{}, {} lamps",
                &[lang.tr(lamp.kind), &lamp.lamps.to_string()],
            )
        } else {
            lang.tr("Windows has not handed it over yet").to_string()
        };
        let sub = Area::new(name.x, row.y + pen.s(22.0), row.w * 0.5, pen.s(14.0));
        pen.fitted_left(&about, sub, 10.0, 8.0, color::DIM)?;
        let (text, on) = if lamp.follow {
            (lang.tr("Follows the keyboard"), true)
        } else {
            (lang.tr("Follow the keyboard"), false)
        };
        let fw = pen.width(text, &font) + pen.s(24.0);
        let pill = [(text.to_string(), Hit::LampFollow(i), on)];
        view::pills(
            pen,
            row.right() - fw - pen.s(8.0),
            row.y + pen.s(5.0),
            &pill,
            hits,
        )?;
    }
    Ok(())
}
