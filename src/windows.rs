//! The Windows lighting page.
//!
//! Whether the keyboard lets Windows Dynamic Lighting take its LEDs, a
//! way to the Windows settings, and where other lit devices stand. Works
//! without a keyboard too.

use aurea::AureaResult;

use crate::app::{Hit, Shared};
use crate::device::Connection;
use crate::draw::{Area, Pen, color};
use crate::view::{self, Hits};

pub fn tab(pen: &mut Pen<'_>, body: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<String> {
    let lang = pen.lang;
    let card = Area::new(
        body.x + (body.w - pen.s(720.0)).max(0.0) / 2.0,
        body.y + pen.s(24.0),
        pen.s(720.0).min(body.w),
        pen.s(320.0),
    );
    pen.round(card, pen.s(14.0), color::SURFACE)?;
    let x = card.x + pen.s(28.0);
    let w = card.w - pen.s(56.0);
    let mut y = card.y + pen.s(24.0);
    pen.text(
        lang.tr("Windows Dynamic Lighting"),
        x,
        y,
        &pen.bold(18.0),
        color::TEXT,
    )?;
    y += pen.s(34.0);
    pen.fitted_left(
        lang.tr("Your Lykil keyboard is a Dynamic Lighting device: Windows can light it together with your other devices."),
        Area::new(x, y, w, pen.s(18.0)),
        13.0,
        9.0,
        color::DIM,
    )?;
    y += pen.s(40.0);

    view::label(pen, lang.tr("THIS KEYBOARD"), x, y)?;
    y += pen.s(20.0);
    let settings = shared.lighting();
    match settings {
        Some(s) if shared.keyboard.connection == Connection::Connected => {
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
            view::pills(pen, card.right() - pen.s(28.0) - bw, y, &[flip], hits)?;
        }
        _ => {
            pen.fitted_left(
                lang.tr("Connect the keyboard to change how it shares its LEDs."),
                Area::new(x, y, w, pen.s(30.0)),
                13.0,
                9.0,
                color::FAINT,
            )?;
        }
    }
    y += pen.s(56.0);

    view::label(pen, lang.tr("OTHER DEVICES"), x, y)?;
    y += pen.s(20.0);
    pen.fitted_left(
        lang.tr("Mice, cases and other lit devices are set in Windows Settings for now. Driving them from Studio, in step with the keyboard, comes later."),
        Area::new(x, y, w, pen.s(18.0)),
        12.0,
        8.0,
        color::DIM,
    )?;
    y += pen.s(40.0);
    let open = [(
        lang.tr("Open Dynamic Lighting settings").to_string(),
        Hit::LightingSettings,
        true,
    )];
    view::pills(pen, x, y, &open, hits)?;
    Ok(lang.tr("Ctrl+1 to Ctrl+5 or Ctrl+Tab switch pages").into())
}
