//! The whole window, drawn on one canvas.

use aurea::AureaResult;
use aurea::render::{Color, DrawingContext};
use lykil::binding::Binding;
use lykil_protocol::describe::Description;
use lykil_protocol::lcp;

use crate::anim::{self, Key, rate};
use crate::app::{Hit, Shared, Tab};
use crate::device::{Connection, Keyboard};
use crate::draw::Hits;
use crate::draw::{Area, Pen, color};
use crate::edit::{self, Hold};
use crate::keyboard::{self, Keys};
use crate::legend;
use crate::lights;
use crate::widgets::{chip, label, pills, text_field};
use crate::{icons, windows};

const HEADER: f32 = 60.0;
const FOOTER: f32 = 30.0;
const MARGIN: f32 = 24.0;
pub fn draw(ctx: &mut dyn DrawingContext, shared: &mut Shared) -> AureaResult<()> {
    #[allow(clippy::cast_precision_loss)]
    let (w, h) = (ctx.width() as f32, ctx.height() as f32);
    let scale = if shared.ui.scale > 0.0 {
        shared.ui.scale
    } else {
        1.0
    };
    shared.settle();
    shared.follow_presses();
    let mut anim = std::mem::take(&mut shared.ui.anim);
    anim.frame();
    let mut pen = Pen {
        ctx,
        scale,
        mouse: shared.ui.mouse,
        lang: shared.ui.lang,
        anim: &mut anim,
    };
    let mut hits = Hits::new();
    pen.fill(Area::new(0.0, 0.0, w, h), color::BACKGROUND)?;

    header(&mut pen, w, shared, &mut hits)?;
    let full = Area::new(
        pen.s(MARGIN),
        pen.s(HEADER + 8.0),
        w - pen.s(2.0 * MARGIN),
        h - pen.s(HEADER + 8.0 + FOOTER),
    );
    // A new page rises into place and fades in.
    let arrived = anim::smooth(pen.anim.to(Key::Page, 1.0, rate::PAGE));
    let body = Area::new(
        full.x,
        full.y + pen.s(14.0) * (1.0 - arrived),
        full.w,
        full.h,
    );
    let status = if shared.ui.tab == Tab::Windows {
        windows::tab(&mut pen, body, shared, &mut hits)?
    } else if shared.keyboard.connection == Connection::Connected {
        match shared.ui.tab {
            Tab::Keymap => keymap_tab(&mut pen, body, shared, &mut hits)?,
            Tab::Macros => macros_tab(&mut pen, body, shared, &mut hits)?,
            Tab::Lighting if shared.keyboard.via.is_some() => {
                crate::via_lights::tab(&mut pen, body, shared, &mut hits)?
            }
            Tab::Lighting => lights::tab(&mut pen, body, shared, &mut hits)?,
            Tab::Device => device_tab(&mut pen, body, shared, &mut hits)?,
            Tab::Windows => String::new(),
        }
    } else {
        waiting(&mut pen, body, &shared.keyboard, &mut hits)?;
        String::new()
    };
    if arrived < 1.0 {
        pen.veil(
            Area::new(0.0, full.y - pen.s(8.0), w, full.h + pen.s(8.0)),
            color::BACKGROUND,
            1.0 - arrived,
        )?;
    }
    // A tab under the mouse says what it is; narrow tabs show no name.
    let status = match shared.ui.hovered() {
        Some(Hit::Tab(tab)) => format!("{}   Ctrl+{}", pen.lang.tr(tab.name()), tab.index() + 1),
        _ => status,
    };
    footer(
        &mut pen,
        (w, h),
        &shared.keyboard,
        shared.ui.notice.as_deref(),
        &status,
    )?;
    shared.ui.hits = hits;
    shared.ui.anim = anim;
    Ok(())
}

fn header(pen: &mut Pen<'_>, w: f32, shared: &Shared, hits: &mut Hits) -> AureaResult<()> {
    pen.fill(Area::new(0.0, 0.0, w, pen.s(HEADER)), color::SURFACE)?;
    pen.fill(Area::new(0.0, pen.s(HEADER) - 1.0, w, 1.0), color::BORDER)?;
    let kb = &shared.keyboard;
    let left_end = title(pen, kb)?;
    let right_start = status(pen, w, kb, hits)?;
    tab_bar(pen, (left_end, right_start, w), shared.ui.tab, hits)
}

fn title(pen: &mut Pen<'_>, kb: &Keyboard) -> AureaResult<f32> {
    let x = pen.s(MARGIN);
    let small = pen.bold(11.0);
    pen.text("LYKIL STUDIO", x, pen.s(12.0), &small, color::ACCENT)?;
    let name = if kb.connection == Connection::Connected {
        kb.name.as_str()
    } else {
        pen.lang.tr("No keyboard")
    };
    let big = pen.bold(17.0);
    pen.text(name, x, pen.s(28.0), &big, color::TEXT)?;
    let wide = pen.width(name, &big).max(pen.width("LYKIL STUDIO", &small));
    Ok(x + wide)
}

fn status(pen: &mut Pen<'_>, w: f32, kb: &Keyboard, hits: &mut Hits) -> AureaResult<f32> {
    let lang = pen.lang;
    let (text, dot) = match &kb.connection {
        Connection::Connected if kb.via.is_some() => (lang.tr("Connected over VIA"), color::GOOD),
        Connection::Connected => (lang.tr("Connected"), color::GOOD),
        Connection::NeedsDefinition => (lang.tr("VIA definition needed"), color::PRESSED),
        Connection::Searching => (lang.tr("Looking for a keyboard"), color::DIM),
        Connection::Lost(_) => (lang.tr("Connection lost"), color::BAD),
    };
    let font = pen.font(12.0);
    let tw = pen.width(text, &font);
    let right = w - pen.s(MARGIN);
    pen.circle(
        right - tw - pen.s(12.0),
        pen.s(HEADER / 2.0),
        pen.s(4.0),
        dot,
    )?;
    pen.text(
        text,
        right - tw,
        pen.s(HEADER / 2.0 - 7.0),
        &font,
        color::DIM,
    )?;

    let other = lang.other().label();
    // In its own language's font: the current one may not have its letters.
    let font = aurea::render::Font::new(lang.other().font_family(), pen.s(12.0));
    let lw = pen.width(other, &font) + pen.s(20.0);
    let switch = Area::new(
        right - tw - pen.s(28.0) - lw,
        pen.s(HEADER / 2.0 - 12.0),
        lw,
        pen.s(24.0),
    );
    let t = pen.hover(switch, Hit::Lang);
    pen.round(
        switch,
        pen.s(12.0),
        color::mix(color::RAISED, color::HOVER, t),
    )?;
    pen.centred(other, switch, &font, color::DIM)?;
    hits.push((switch, Hit::Lang));
    Ok(switch.x)
}

fn tab_bar(
    pen: &mut Pen<'_>,
    (left, right, width): (f32, f32, f32),
    current: Tab,
    hits: &mut Hits,
) -> AureaResult<()> {
    #[allow(clippy::cast_precision_loss)]
    let count = Tab::ALL.len() as f32;
    let gap = pen.s(20.0);
    let room = (right - left - 2.0 * gap).max(0.0);
    let tab_w = ((room - pen.s(8.0)) / count).clamp(pen.s(40.0), pen.s(118.0));
    let tab_h = pen.s(34.0);
    let total = tab_w * count + pen.s(8.0);
    let centred = (width - total) / 2.0;
    let x = centred
        .max(left + gap)
        .min((right - gap - total).max(left + gap));
    let bar = Area::new(x, pen.s(13.0), total, tab_h + pen.s(8.0));
    pen.round(bar, pen.s(10.0), color::BACKGROUND)?;
    let pad = pen.s(4.0);
    let tab_at = |i: f32| Area::new(bar.x + pad + tab_w * i, bar.y + pad, tab_w, tab_h);
    #[allow(clippy::cast_precision_loss)]
    let at = pen
        .anim
        .to(Key::TabBar, current.index() as f32, rate::SLIDE);
    for (i, tab) in Tab::ALL.into_iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let a = tab_at(i as f32);
        let t = pen.hover(a, Hit::Tab(tab));
        if t > 0.01 {
            pen.round(
                a,
                pen.s(8.0),
                color::mix(color::BACKGROUND, color::RAISED, t),
            )?;
        }
    }
    pen.round(tab_at(at), pen.s(8.0), color::ACCENT)?;
    for (i, tab) in Tab::ALL.into_iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let a = tab_at(i as f32);
        #[allow(clippy::cast_precision_loss)]
        let near = 1.0 - (at - i as f32).abs().min(1.0);
        let fg = color::mix(color::DIM, color::ACCENT_TEXT, near);
        tab_label(pen, a, tab, fg)?;
        hits.push((a, Hit::Tab(tab)));
    }
    Ok(())
}

fn tab_label(pen: &mut Pen<'_>, a: Area, tab: Tab, fg: Color) -> AureaResult<()> {
    let text = pen.lang.tr(tab.name());
    let icon = pen.s(14.0);
    let cy = a.y + a.h / 2.0;
    if a.w < pen.s(84.0) {
        let x = a.x + (a.w - icon) / 2.0;
        return icons::tab(pen, tab, Area::new(x, cy - icon / 2.0, icon, icon), fg);
    }
    let gap = pen.s(7.0);
    let room = a.w - icon - gap - pen.s(16.0);
    let mut font = pen.bold(13.0);
    while pen.width(text, &font) > room && font.size > pen.s(9.0) {
        font.size -= pen.s(0.5);
    }
    let tw = pen.width(text, &font);
    let x = a.x + (a.w - icon - gap - tw) / 2.0;
    icons::tab(pen, tab, Area::new(x, cy - icon / 2.0, icon, icon), fg)?;
    pen.text(
        text,
        x + icon + gap,
        a.y + (a.h - font.size) / 2.0,
        &font,
        fg,
    )
}

fn footer(
    pen: &mut Pen<'_>,
    (w, h): (f32, f32),
    kb: &Keyboard,
    notice: Option<&str>,
    status: &str,
) -> AureaResult<()> {
    let lang = pen.lang;
    let bar = Area::new(0.0, h - pen.s(FOOTER), w, pen.s(FOOTER));
    pen.fill(bar, color::SURFACE)?;
    pen.fill(Area::new(0.0, bar.y, w, 1.0), color::BORDER)?;
    let (text, c) = match (notice, &kb.error) {
        (Some(n), _) => (n.to_string(), color::BAD),
        (None, Some(e)) => (
            lang.fill("the keyboard refused the change: {}", &[e]),
            color::BAD,
        ),
        (None, None) => (status.to_string(), color::DIM),
    };
    pen.text(&text, pen.s(MARGIN), bar.y + pen.s(8.0), &pen.font(12.0), c)?;
    if let Some(d) = &kb.diagnostics {
        let health = if d.faults == 0 {
            lang.fill("up {}   healthy", &[&uptime(d.uptime_ms)])
        } else {
            lang.fill(
                "up {}   {} faults",
                &[&uptime(d.uptime_ms), &d.faults.to_string()],
            )
        };
        let font = pen.font(12.0);
        let tw = pen.width(&health, &font);
        let c = if d.faults == 0 {
            color::FAINT
        } else {
            color::BAD
        };
        pen.text(
            &health,
            w - pen.s(MARGIN) - tw,
            bar.y + pen.s(8.0),
            &font,
            c,
        )?;
    }
    Ok(())
}

fn uptime(ms: u32) -> String {
    let s = ms / 1000;
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, s / 60 % 60)
    } else {
        format!("{}m {:02}s", s / 60, s % 60)
    }
}

fn waiting(pen: &mut Pen<'_>, body: Area, kb: &Keyboard, hits: &mut Hits) -> AureaResult<()> {
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

fn needs_definition(
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
        crate::via::folder().display().to_string(),
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

fn keymap_tab(
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
    pen.round(panel, pen.s(12.0), color::SURFACE)?;
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

fn key_card(
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
    pen.round(
        cap,
        pen.s(8.0),
        color::mix(color::ACCENT, color::BACKGROUND, 0.5),
    )?;
    pen.round(cap.inset(pen.s(3.0)), pen.s(7.0), color::ACCENT)?;
    pen.fitted(&main, cap.inset(pen.s(8.0)), 16.0, 8.0, color::ACCENT_TEXT)?;
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

fn hovered_key(
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

fn key_parts(
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
        y = small_pills(
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
        small_pills(
            pen,
            Area::new(area.x, y, area.w, area.bottom() - y),
            &items,
            hits,
        )?;
    }
    Ok(())
}

fn small_pills(
    pen: &mut Pen<'_>,
    area: Area,
    items: &[(String, Hit, bool)],
    hits: &mut Hits,
) -> AureaResult<f32> {
    let font = pen.font(11.0);
    let pill_h = pen.s(24.0);
    let (mut x, mut y) = (area.x, area.y);
    for (text, hit, active) in items {
        let w = pen.width(text, &font) + pen.s(18.0);
        if x + w > area.right() && x > area.x {
            x = area.x;
            y += pill_h + pen.s(5.0);
        }
        if y + pill_h > area.bottom() {
            break;
        }
        let a = Area::new(x, y, w, pill_h);
        let hover = pen.hover(a, *hit);
        let (bg, fg) = if *active {
            (color::ACCENT, color::ACCENT_TEXT)
        } else {
            (
                color::mix(color::RAISED, color::HOVER, hover),
                color::mix(color::DIM, color::TEXT, hover),
            )
        };
        pen.round(a, pill_h / 2.0, bg)?;
        pen.centred(text, a, &font, fg)?;
        hits.push((a, *hit));
        x += w + pen.s(5.0);
    }
    Ok(y + pill_h)
}

/// What the keymap page does, while no key is picked.
fn keymap_tips(pen: &mut Pen<'_>, panel: Area) -> AureaResult<()> {
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

fn layer_bar(
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
    let rw = pen.width(reset, &font) + pen.s(24.0);
    let ra = Area::new(body.right() - rw, body.y, rw, pen.s(30.0));
    let t = pen.hover(ra, Hit::ResetKeymap);
    let (bg, fg) = if confirm_reset {
        (color::BAD, color::ACCENT_TEXT)
    } else {
        (
            color::mix(color::RAISED, color::HOVER, t),
            color::mix(color::DIM, color::TEXT, t),
        )
    };
    pen.round(ra, pen.s(15.0), bg)?;
    pen.centred(reset, ra, &font, fg)?;
    hits.push((ra, Hit::ResetKeymap));
    Ok(())
}

/// `holds` says which bindings the keyboard can take; the rest are Lykil
/// only and show dimmed.
fn palette(
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
        chip(
            pen,
            (a, Hit::Palette(*b)),
            &main,
            sub.as_deref(),
            *b == current,
        )?;
        if holds(*b) {
            hits.push((a, Hit::Palette(*b)));
        } else {
            pen.veil(a, color::SURFACE, 0.65)?;
        }
        x += w + gap;
    }
    Ok(())
}

/// The palette's groups down the left, the chosen one marked.
fn group_list(
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
        let active = i == chosen;
        let has_current = group.items.contains(&current);
        let t = pen.hover(a, Hit::Group(i));
        let bg = if active {
            color::RAISED
        } else {
            color::mix(color::SURFACE, color::RAISED, 0.5 * t)
        };
        pen.round(a, pen.s(6.0), bg)?;
        if active {
            pen.round(
                Area::new(a.x, a.y + pen.s(5.0), pen.s(3.0), a.h - pen.s(10.0)),
                pen.s(1.5),
                color::ACCENT,
            )?;
        }
        let fg = if active { color::TEXT } else { color::DIM };
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
            pen.circle(
                a.right() - pen.s(10.0),
                a.y + a.h / 2.0,
                pen.s(3.0),
                color::ACCENT,
            )?;
        }
        hits.push((a, Hit::Group(i)));
    }

    Ok(())
}

fn macro_list(
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
fn macro_buttons(
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

fn macro_note(pen: &mut Pen<'_>, editor: Area, id: usize) -> AureaResult<()> {
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

fn macros_tab(
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

fn device_tab(
    pen: &mut Pen<'_>,
    body: Area,
    shared: &mut Shared,
    hits: &mut Hits,
) -> AureaResult<String> {
    let lang = pen.lang;
    let kb = &shared.keyboard;
    let kb_area = Area::new(body.x, body.y + pen.s(8.0), body.w, body.h * 0.46);
    let layers = kb.layer_names();
    let empty = Vec::new();
    let keys = Keys::Device {
        bindings: kb.keymap.first().unwrap_or(&empty),
        layers: &layers,
    };
    let used = keyboard::keyboard(pen, kb_area, kb, &keys, false, hits)?;
    let mut cards: Vec<(&str, String)> = Vec::new();
    if let Some(h) = kb.hello {
        cards.push((
            "LAYOUT",
            lang.fill(
                "{} keys, {} layers",
                &[&h.keys.to_string(), &h.layers.to_string()],
            ),
        ));
        cards.push(("MATRIX", format!("{} x {}", h.matrix_rows, h.matrix_cols)));
        cards.push(("PROTOCOL", format!("LCP {}", h.version)));
    }
    if let Some(v) = kb.via {
        cards.push(("PROTOCOL", format!("VIA {}", v.protocol)));
        let (vid, pid) = v.ids;
        cards.push(("USB ID", format!("{vid:04X}:{pid:04X}")));
    }
    match &kb.firmware {
        Some(fw) => {
            cards.push(("FIRMWARE", format!("{} {}", fw.name, fw.version)));
            cards.push(("LYKIL", fw.lykil.to_string()));
            cards.push(("CHIP ID", fw.id.clone()));
        }
        None => cards.push(("FIRMWARE", lang.tr("update it to see").to_string())),
    }
    if let Some(d) = &kb.diagnostics {
        cards.push(("UPTIME", uptime(d.uptime_ms)));
        cards.push(("SCANS", group(d.scans)));
        cards.push((
            "KEY CHANGES",
            lang.fill(
                "{} ({} raw)",
                &[&group(d.stable_transitions), &group(d.raw_transitions)],
            ),
        ));
        cards.push(("FAULTS", d.faults.to_string()));
        cards.push(("WATCHDOG RESETS", d.watchdog_resets.to_string()));
        cards.push((
            "LAST START",
            lang.tr(lcp::reset::name(d.reset_cause)).to_string(),
        ));
        cards.push((
            "STORAGE",
            lang.tr(match d.storage {
                1 => "ok",
                2 => "failed",
                _ => "none",
            })
            .to_string(),
        ));
    }
    let top = used.bottom() + pen.s(20.0);
    let cols = 5.0;
    let gap = pen.s(10.0);
    let cw = (body.w - gap * (cols - 1.0)) / cols;
    let ch = pen.s(62.0);
    for (i, (name, value)) in cards.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let (col, row) = ((i % 5) as f32, (i / 5) as f32);
        let a = Area::new(body.x + col * (cw + gap), top + row * (ch + gap), cw, ch);
        if a.bottom() > body.bottom() {
            break;
        }
        pen.round(a, pen.s(10.0), color::SURFACE)?;
        label(pen, lang.tr(name), a.x + pen.s(14.0), a.y + pen.s(12.0))?;
        let v = Area::new(
            a.x + pen.s(14.0),
            a.y + pen.s(30.0),
            a.w - pen.s(28.0),
            pen.s(20.0),
        );
        let c = match *name {
            "FAULTS" | "WATCHDOG RESETS" if value != "0" => color::BAD,
            _ => color::TEXT,
        };
        pen.fitted_left(value, v, 15.0, 9.0, c)?;
        let _ = &hits;
    }
    Ok(lang
        .tr("Keys light up while pressed: a quick way to check every switch.")
        .into())
}

fn group(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_times() {
        assert_eq!(group(1_234_567), "1,234,567");
        assert_eq!(group(12), "12");
        assert_eq!(uptime(61_000), "1m 01s");
        assert_eq!(uptime(3_725_000), "1h 02m");
    }
}
