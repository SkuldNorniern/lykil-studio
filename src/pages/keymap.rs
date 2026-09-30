//! The keymap page: the keyboard, the picked key's card and the palette.

use aurea::AureaResult;
use lykil::binding::Binding;
use lykil_protocol::describe::Description;

use crate::app::{Hit, Shared};
use crate::components::button::{Tone, cap_face, keycap, pill, pill_width};
use crate::components::dot::mark;
use crate::components::surface::{dim, list_row};
use crate::components::text::label;
use crate::draw::Hits;
use crate::draw::{Area, Pen, color};
use crate::edit::{self, Hold};
use crate::keyboard::{self, Keys};
use crate::legend;
use crate::widgets::pills::{pill_flow, pills};

pub fn keymap_tab(
    pen: &mut Pen<'_>,
    body: Area,
    shared: &mut Shared,
    hits: &mut Hits,
) -> AureaResult<String> {
    let lang = pen.lang;
    let kb = &shared.keyboard;
    let ui = &shared.ui;
    let layers = kb.layer_names();
    layer_bar(pen, body, &layers, ui.layer, ui.confirm_reset, hits)?;

    let empty = Vec::new();
    let bindings = kb.keymap.get(usize::from(ui.layer)).unwrap_or(&empty);
    let kb_area = Area::new(body.x, body.y + pen.s(48.0), body.w, body.h * 0.46);
    let used = keyboard::keyboard(
        pen,
        kb_area,
        kb,
        &Keys::Keymap {
            bindings,
            layers: &layers,
            selected: ui.selected,
        },
        true,
        hits,
    )?;

    let panel = Area::new(
        body.x,
        used.bottom() + pen.s(20.0),
        body.w,
        body.bottom() - used.bottom() - pen.s(20.0),
    );
    if panel.h < pen.s(60.0) {
        return Ok(String::new());
    }
    crate::components::surface::panel(pen, panel)?;
    let desc = kb.description.as_ref();
    let hovered = hovered_key(ui, desc, bindings, &layers);
    let Some((index, info)) = ui.selected.and_then(|k| desc?.keys.get(k).map(|d| (k, d))) else {
        keymap_tips(pen, panel)?;
        return Ok(hovered.unwrap_or_else(|| {
            lang.tr("Click a key, then pick a binding. Changes are saved on the keyboard.")
                .into()
        }));
    };
    let current = bindings.get(index).copied().unwrap_or_default();

    let left = Area::new(
        panel.x + pen.s(20.0),
        panel.y + pen.s(18.0),
        pen.s(300.0),
        panel.h - pen.s(36.0),
    );
    let layer_name = layers
        .get(usize::from(ui.layer))
        .map_or("?", String::as_str);
    key_card(pen, left, (&info.id, layer_name), current, &layers, hits)?;

    let right = Area::new(
        left.right() + pen.s(24.0),
        panel.y + pen.s(18.0),
        panel.right() - left.right() - pen.s(44.0),
        panel.h - pen.s(36.0),
    );
    let holds = |b: Binding| kb.via.is_none_or(|v| v.holds(b));
    palette(
        pen,
        right,
        (&layers, &holds),
        current,
        (ui.group, !kb.macros.is_empty()),
        hits,
    )?;
    Ok(hovered.unwrap_or_else(|| {
        lang.fill(
            "Editing {} on {}. A pick moves on to the next key. Esc to stop.",
            &[&info.id, layer_name],
        )
    }))
}

pub fn key_card(
    pen: &mut Pen<'_>,
    left: Area,
    (id, layer_name): (&str, &str),
    current: Binding,
    layers: &[String],
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let (main, _) = legend::keycap(current, layers);
    let cap = Area::new(left.x, left.y, pen.s(58.0), pen.s(58.0));
    cap_face(pen, cap, &main)?;
    let tx = cap.right() + pen.s(14.0);
    pen.text(
        &lang.fill("{} on {}", &[id, layer_name]),
        tx,
        cap.y + pen.s(4.0),
        &pen.bold(14.0),
        color::TEXT,
    )?;
    let full = legend::full(current, layers);
    let now = Area::new(tx, cap.y + pen.s(28.0), left.right() - tx, pen.s(18.0));
    pen.fitted_left(&full, now, 12.0, 8.0, color::ACCENT)?;
    let parts = Area::new(
        left.x,
        cap.bottom() + pen.s(16.0),
        left.w,
        left.bottom() - cap.bottom() - pen.s(16.0),
    );
    key_parts(pen, parts, current, layers, hits)?;
    Ok(())
}

pub fn hovered_key(
    ui: &crate::app::Ui,
    desc: Option<&Description>,
    bindings: &[Binding],
    layers: &[String],
) -> Option<String> {
    let Some(Hit::Key(k)) = ui.hovered() else {
        return None;
    };
    let id = &desc?.keys.get(k)?.id;
    let b = bindings.get(k).copied().unwrap_or_default();
    Some(format!("{id}: {}", legend::full(b, layers)))
}

pub fn key_parts(
    pen: &mut Pen<'_>,
    area: Area,
    current: Binding,
    layers: &[String],
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let mut y = area.y;
    if let Some(now) = edit::hold(current) {
        label(pen, lang.tr("WHEN HELD"), area.x, y)?;
        y += pen.s(16.0);
        let mut items = vec![(
            lang.tr("tap only").to_string(),
            Hit::Hold(Hold::Nothing),
            now == Hold::Nothing,
        )];
        for (name, m) in edit::MODS {
            items.push((
                name.to_string(),
                Hit::Hold(Hold::Mods(m)),
                now == Hold::Mods(m),
            ));
        }
        for (i, name) in layers.iter().enumerate() {
            let Ok(l) = u8::try_from(i) else { continue };
            let hold = Hold::Layer(lykil::layer::LayerId(l));
            items.push((name.clone(), Hit::Hold(hold), now == hold));
        }
        y += pill_flow(
            pen,
            Area::new(area.x, y, area.w, area.bottom() - y),
            &items,
            hits,
        )? + pen.s(10.0);
    }
    if let Some(mods) = edit::with(current) {
        label(pen, lang.tr("SEND WITH"), area.x, y)?;
        y += pen.s(16.0);
        let items: Vec<_> = edit::MODS
            .iter()
            .map(|(name, m)| ((*name).to_string(), Hit::With(*m), mods.0 & m.0 != 0))
            .collect();
        pill_flow(
            pen,
            Area::new(area.x, y, area.w, area.bottom() - y),
            &items,
            hits,
        )?;
    }
    Ok(())
}

/// What the keymap page does, while no key is picked.
pub fn keymap_tips(pen: &mut Pen<'_>, panel: Area) -> AureaResult<()> {
    let lang = pen.lang;
    let title = Area::new(
        panel.x,
        panel.y + panel.h / 2.0 - pen.s(70.0),
        panel.w,
        pen.s(24.0),
    );
    pen.centred(
        lang.tr("Click a key to change what it does"),
        title,
        &pen.bold(15.0),
        color::TEXT,
    )?;
    let tips = [
        "Arrow keys move to the next key, Delete clears it, Esc lets go.",
        "Pick a layer above; see-through keys use the layer below.",
        "When held makes a key do two things: tap for one, hold for another.",
        "Send with adds modifiers, so one key can type Shift+1 or Ctrl+C.",
    ];
    let font = pen.font(12.0);
    for (i, tip) in tips.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let a = Area::new(
            panel.x,
            title.bottom() + pen.s(14.0) + pen.s(22.0) * i as f32,
            panel.w,
            pen.s(18.0),
        );
        pen.centred(lang.tr(tip), a, &font, color::DIM)?;
    }
    Ok(())
}

pub fn layer_bar(
    pen: &mut Pen<'_>,
    body: Area,
    layers: &[String],
    current: u8,
    confirm_reset: bool,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    label(pen, lang.tr("LAYER"), body.x, body.y + pen.s(8.0))?;
    let items: Vec<_> = layers
        .iter()
        .enumerate()
        .filter_map(|(i, n)| {
            let l = u8::try_from(i).ok()?;
            Some((n.clone(), Hit::Layer(l), l == current))
        })
        .collect();
    pills(pen, body.x + pen.s(56.0), body.y, &items, hits)?;

    let reset = if confirm_reset {
        lang.tr("Click again to reset every layer")
    } else {
        lang.tr("Reset keymap")
    };
    let font = pen.bold(12.0);
    let rw = pill_width(pen, reset, &font, 12.0);
    let ra = Area::new(body.right() - rw, body.y, rw, pen.s(30.0));
    let tone = if confirm_reset {
        Tone::Danger
    } else {
        Tone::Quiet
    };
    pill(pen, ra, (reset, &font), Hit::ResetKeymap, tone)?;
    hits.push((ra, Hit::ResetKeymap));
    Ok(())
}

/// `holds` says which bindings the keyboard can take; the rest are Lykil
/// only and show dimmed.
pub fn palette(
    pen: &mut Pen<'_>,
    area: Area,
    (layers, holds): (&[String], &dyn Fn(Binding) -> bool),
    current: Binding,
    (chosen, macros): (usize, bool),
    hits: &mut Hits,
) -> AureaResult<()> {
    let groups = legend::palette(layers, macros);
    let list_w = pen.s(150.0);
    let list = Area::new(area.x, area.y, list_w, area.h);
    group_list(pen, list, (&groups, chosen), current, holds, hits)?;

    let Some(group) = groups.get(chosen) else {
        return Ok(());
    };
    let grid = Area::new(
        area.x + list_w + pen.s(20.0),
        area.y,
        area.w - list_w - pen.s(20.0),
        area.h,
    );
    let cap = pen.s(46.0);
    let gap = pen.s(6.0);
    let mut x = grid.x;
    let mut y = grid.y;
    for b in &group.items {
        let (main, sub) = legend::keycap(*b, layers);
        let main = if main.is_empty() {
            "none".to_string()
        } else {
            main
        };
        let w = if pen.width(&main, &pen.font(12.0)) > cap - pen.s(10.0) {
            cap * 2.0 + gap
        } else {
            cap
        };
        if x + w > grid.right() {
            x = grid.x;
            y += cap + gap;
        }
        if y + cap > grid.bottom() {
            break;
        }
        let a = Area::new(x, y, w, cap);
        keycap(
            pen,
            (a, Hit::Palette(*b)),
            &main,
            sub.as_deref(),
            *b == current,
        )?;
        if holds(*b) {
            hits.push((a, Hit::Palette(*b)));
        } else {
            dim(pen, a)?;
        }
        x += w + gap;
    }
    Ok(())
}

/// The palette's groups down the left, the chosen one marked.
pub fn group_list(
    pen: &mut Pen<'_>,
    area: Area,
    (groups, chosen): (&[legend::Group], usize),
    current: Binding,
    holds: &dyn Fn(Binding) -> bool,
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    #[allow(clippy::cast_precision_loss)]
    let row_h = pen.s(24.0).min(area.h / groups.len().max(1) as f32);
    for (i, group) in groups.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let a = Area::new(
            area.x,
            area.y + row_h * i as f32,
            area.w,
            row_h - pen.s(2.0),
        );
        if a.bottom() > area.bottom() {
            break;
        }
        let has_current = group.items.contains(&current);
        let fg = list_row(pen, a, Hit::Group(i), i == chosen)?;
        let name = if group.items.iter().any(|b| holds(*b)) {
            lang.tr(group.name).to_string()
        } else {
            lang.fill("{} (Lykil only)", &[lang.tr(group.name)])
        };
        pen.fitted_left(
            &name,
            Area::new(a.x + pen.s(12.0), a.y, a.w - pen.s(24.0), a.h),
            12.0,
            8.0,
            fg,
        )?;
        if has_current {
            mark(pen, a.right() - pen.s(10.0), a.y + a.h / 2.0)?;
        }
        hits.push((a, Hit::Group(i)));
    }

    Ok(())
}
