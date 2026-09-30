//! The macros page: the list, the text editor and its buttons.

use aurea::AureaResult;

use crate::app::{Hit, Shared};
use crate::draw::Hits;
use crate::draw::{Area, Pen, color};
use crate::widgets::{label, pills, text_field};

pub fn macros_tab(
    pen: &mut Pen<'_>,
    body: Area,
    shared: &mut Shared,
    hits: &mut Hits,
) -> AureaResult<String> {
    let lang = pen.lang;
    let kb = &shared.keyboard;
    let ui = &shared.ui;
    if kb.macros.is_empty() {
        pen.centred(
            lang.tr("This keyboard's firmware has no macros yet"),
            body,
            &pen.font(15.0),
            color::DIM,
        )?;
        return Ok(String::new());
    }
    let list = Area::new(
        body.x,
        body.y + pen.s(8.0),
        pen.s(300.0),
        body.h - pen.s(8.0),
    );
    pen.round(list, pen.s(12.0), color::SURFACE)?;
    macro_list(pen, list, &kb.macros, ui.macro_id, hits)?;

    let editor = Area::new(
        list.right() + pen.s(20.0),
        list.y,
        body.right() - list.right() - pen.s(20.0),
        pen.s(270.0),
    );
    pen.round(editor, pen.s(12.0), color::SURFACE)?;
    let x = editor.x + pen.s(20.0);
    let id = ui.macro_id;
    pen.text(
        &lang.fill("Macro {}", &[&format!("M{id}")]),
        x,
        editor.y + pen.s(18.0),
        &pen.bold(17.0),
        color::TEXT,
    )?;
    let saved = kb.macros.get(id).cloned().unwrap_or_default();
    let editing = ui.macro_text.is_some();
    let text = ui
        .macro_text
        .clone()
        .or_else(|| lykil_config::text::text(&saved))
        .unwrap_or_default();
    label(pen, lang.tr("TYPES"), x, editor.y + pen.s(54.0))?;
    let field = Area::new(
        x,
        editor.y + pen.s(72.0),
        editor.w - pen.s(40.0),
        pen.s(110.0),
    );
    pen.round(field, pen.s(8.0), color::BACKGROUND)?;
    if editing {
        pen.outline(field, pen.s(8.0), pen.s(1.5), color::ACCENT)?;
    }
    text_field(pen, field, &text, editing)?;
    let steps = lykil_config::text::steps(&text).map_or(0, |s| s.len());
    let full = steps > lykil::macros::MACRO_STEPS;
    let count = lang.fill(
        "{} / {} steps",
        &[&steps.to_string(), &lykil::macros::MACRO_STEPS.to_string()],
    );
    let cf = pen.font(11.0);
    let cw = pen.width(&count, &cf);
    pen.text(
        &count,
        field.right() - cw,
        field.bottom() + pen.s(8.0),
        &cf,
        if full { color::BAD } else { color::FAINT },
    )?;
    let row = Area::new(
        x,
        field.bottom() + pen.s(30.0),
        editor.right() - x - pen.s(20.0),
        pen.s(30.0),
    );
    macro_buttons(pen, row, (editing, !saved.is_empty(), steps), hits)?;
    macro_note(pen, editor, id)?;
    Ok(if editing {
        lang.tr("Typing into the macro. Save sends it to the keyboard; Esc throws it away.")
            .into()
    } else {
        lang.tr("Pick a macro and type. Letters, digits, symbols, space, Enter and Tab.")
            .into()
    })
}

pub fn macro_list(
    pen: &mut Pen<'_>,
    list: Area,
    macros: &[Vec<lykil::macros::Step>],
    chosen: usize,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let row_h = pen.s(40.0);
    for (i, steps) in macros.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let a = Area::new(
            list.x + pen.s(8.0),
            list.y + pen.s(8.0) + row_h * i as f32,
            list.w - pen.s(16.0),
            row_h - pen.s(4.0),
        );
        if a.bottom() > list.bottom() {
            break;
        }
        let active = i == chosen;
        let t = pen.hover(a, Hit::Macro(i));
        let bg = if active {
            color::RAISED
        } else {
            color::mix(color::SURFACE, color::RAISED, 0.5 * t)
        };
        pen.round(a, pen.s(8.0), bg)?;
        pen.text(
            &format!("M{i}"),
            a.x + pen.s(12.0),
            a.y + pen.s(10.0),
            &pen.bold(13.0),
            color::ACCENT,
        )?;
        let preview = match lykil_config::text::text(steps) {
            _ if steps.is_empty() => lang.tr("empty").to_string(),
            Some(t) => format!("\"{}\"", t.replace('\n', "\u{21b5}")),
            None if steps.len() == 1 => lang.tr("1 step").to_string(),
            None => lang.fill("{} steps", &[&steps.len().to_string()]),
        };
        let c = if steps.is_empty() {
            color::FAINT
        } else {
            color::DIM
        };
        let p = Area::new(a.x + pen.s(52.0), a.y, a.w - pen.s(60.0), a.h);
        pen.fitted_left(&preview, p, 12.0, 8.0, c)?;
        hits.push((a, Hit::Macro(i)));
    }

    Ok(())
}

/// Save and clear under the macro text, or why it cannot be saved.
pub fn macro_buttons(
    pen: &mut Pen<'_>,
    row: Area,
    (editing, saved, steps): (bool, bool, usize),
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let full = steps > lykil::macros::MACRO_STEPS;
    let mut buttons = Vec::new();
    if editing && !full {
        buttons.push((
            lang.tr("Save to keyboard").to_string(),
            Hit::SaveMacro,
            true,
        ));
    }
    if saved {
        buttons.push((lang.tr("Clear").to_string(), Hit::ClearMacro, false));
    }
    let end = pills(pen, row.x, row.y, &buttons, hits)?;
    if full {
        let over = (steps - lykil::macros::MACRO_STEPS).to_string();
        pen.fitted_left(
            &lang.fill(
                "Too long to save: {} steps over. Shorten the text.",
                &[&over],
            ),
            Area::new(end, row.y, row.right() - end, row.h),
            12.0,
            8.0,
            color::BAD,
        )?;
    }
    Ok(())
}

pub fn macro_note(pen: &mut Pen<'_>, editor: Area, id: usize) -> AureaResult<()> {
    let lang = pen.lang;
    let note = Area::new(
        editor.x + pen.s(20.0),
        editor.bottom() - pen.s(34.0),
        editor.w - pen.s(40.0),
        pen.s(18.0),
    );
    pen.fitted_left(
        &lang.fill(
            "Bind it on the keymap page: Macros group, {}. Typed as a US layout.",
            &[&format!("M{id}")],
        ),
        note,
        12.0,
        8.0,
        color::DIM,
    )?;
    Ok(())
}
