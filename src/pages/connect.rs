//! What shows before a keyboard is ready: plug one in, or add its VIA
//! definition.

use aurea::AureaResult;

use crate::app::Hit;
use crate::devices::{Connection, Keyboard};
use crate::draw::Hits;
use crate::draw::{Area, Pen, color};
use crate::widgets::pills;

pub fn waiting(pen: &mut Pen<'_>, body: Area, kb: &Keyboard, hits: &mut Hits) -> AureaResult<()> {
    let lang = pen.lang;
    if kb.connection == Connection::NeedsDefinition {
        return needs_definition(pen, body, kb, hits);
    }
    let seen = &kb.seen[..kb.seen.len().min(3)];
    let extra = if seen.is_empty() {
        0.0
    } else {
        pen.s(28.0 + 20.0 * f32::from(u8::try_from(seen.len()).unwrap_or(3)))
    };
    let card = Area::new(
        body.x + (body.w - pen.s(460.0)) / 2.0,
        body.y + body.h / 2.0 - pen.s(70.0),
        pen.s(460.0),
        pen.s(140.0) + extra,
    );
    pen.round(card, pen.s(14.0), color::SURFACE)?;
    let title = Area::new(card.x, card.y + pen.s(30.0), card.w, pen.s(24.0));
    pen.centred(
        lang.tr("Plug in a Lykil or VIA keyboard"),
        title,
        &pen.bold(18.0),
        color::TEXT,
    )?;
    let line = match &kb.connection {
        Connection::Lost(why) => lang.fill("The connection was lost: {}", &[why]),
        _ => lang
            .tr("Studio finds it on its own. Close VIA if it is open.")
            .into(),
    };
    let sub = Area::new(
        card.x + pen.s(16.0),
        card.y + pen.s(72.0),
        card.w - pen.s(32.0),
        pen.s(20.0),
    );
    pen.fitted(&line, sub, 13.0, 9.0, color::DIM)?;
    if seen.is_empty() {
        return Ok(());
    }
    let mut y = sub.bottom() + pen.s(20.0);
    let head = Area::new(sub.x, y, sub.w, pen.s(16.0));
    pen.fitted(
        lang.tr("Seen, but no answer:"),
        head,
        11.0,
        8.0,
        color::FAINT,
    )?;
    for line in seen {
        y += pen.s(20.0);
        pen.fitted(
            line,
            Area::new(sub.x, y, sub.w, pen.s(16.0)),
            11.0,
            8.0,
            color::DIM,
        )?;
    }
    Ok(())
}

pub fn needs_definition(
    pen: &mut Pen<'_>,
    body: Area,
    kb: &Keyboard,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let card = Area::new(
        body.x + (body.w - pen.s(560.0)) / 2.0,
        body.y + body.h / 2.0 - pen.s(110.0),
        pen.s(560.0),
        pen.s(220.0),
    );
    pen.round(card, pen.s(14.0), color::SURFACE)?;
    let x = card.x + pen.s(28.0);
    let w = card.w - pen.s(56.0);
    let (vid, pid) = kb.via.map_or((0, 0), |v| v.ids);
    pen.text(
        &lang.fill("{} speaks VIA", &[&kb.name]),
        x,
        card.y + pen.s(26.0),
        &pen.bold(18.0),
        color::TEXT,
    )?;
    let lines = [
        lang.fill(
            "Studio needs its VIA definition, the JSON VIA's Design tab loads (vendor {}, product {}).",
            &[&format!("0x{vid:04X}"), &format!("0x{pid:04X}")],
        ),
        lang.tr("Put the file in this folder; Studio picks it up by itself:")
            .to_string(),
        crate::devices::via::folder().display().to_string(),
    ];
    for (i, line) in lines.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let a = Area::new(x, card.y + pen.s(66.0 + 24.0 * i as f32), w, pen.s(18.0));
        let c = if i == 2 { color::ACCENT } else { color::DIM };
        pen.fitted_left(line, a, 12.0, 8.0, c)?;
    }
    let open = [(lang.tr("Open the folder").to_string(), Hit::ViaFolder, true)];
    pills(pen, x, card.bottom() - pen.s(52.0), &open, hits)?;
    Ok(())
}
