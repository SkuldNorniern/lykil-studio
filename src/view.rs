//! The whole window, drawn on one canvas.
//!
//! Header with the tabs and connection, the keyboard as it describes
//! itself, and under it the panel of the current tab. Every control is
//! recorded in the hit list as it is drawn.

use aurea::AureaResult;
use aurea::render::{Color, DrawingContext};
use lykil::binding::Binding;
use lykil::lighting::{Effect, Point, Rgb, Settings};
use lykil_protocol::describe::Description;
use lykil_protocol::lcp;

use crate::anim::{self, Key, rate};
use crate::app::{Hit, Shared, Tab};
use crate::device::{Connection, Keyboard};
use crate::draw::{Area, Pen, color};
use crate::edit::{self, Hold};
use crate::legend;
use crate::lights::{self, Presses};
use crate::{icons, windows};

const HEADER: f32 = 60.0;
const FOOTER: f32 = 30.0;
const MARGIN: f32 = 24.0;
/// Largest key unit, in design pixels.
const MAX_UNIT: f32 = 58.0;

pub type Hits = Vec<(Area, Hit)>;

pub fn draw(ctx: &mut dyn DrawingContext, shared: &mut Shared) -> AureaResult<()> {
    #[allow(clippy::cast_precision_loss)]
    let (w, h) = (ctx.width() as f32, ctx.height() as f32);
    let scale = if shared.ui.scale > 0.0 {
        shared.ui.scale
    } else {
        1.0
    };
    shared.settle();
    shared.settle_leave();
    shared.follow_presses();
    // The pen holds the eased values while the pages read `shared`.
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
            Tab::Lighting => lights::tab(&mut pen, body, shared, &mut hits)?,
            Tab::Device => device_tab(&mut pen, body, shared, &mut hits)?,
            Tab::Windows => String::new(),
        }
    } else {
        waiting(&mut pen, body, &shared.keyboard)?;
        String::new()
    };
    if arrived < 1.0 {
        pen.veil(
            Area::new(0.0, full.y - pen.s(8.0), w, full.h + pen.s(8.0)),
            color::BACKGROUND,
            1.0 - arrived,
        )?;
    }
    footer(
        &mut pen,
        (w, h),
        &shared.keyboard,
        shared.ui.notice.as_deref(),
        &status,
    )?;
    shared.ui.frame = shared.ui.frame.wrapping_add(1);
    repaint_everything(&mut pen, w, h, shared.ui.frame)?;
    shared.ui.hits = hits;
    shared.ui.anim = anim;
    Ok(())
}

/// Aurea's CPU renderer (git `4a5a7f8`) repaints only tiles whose items
/// changed, but clips spanning draws to the rectangle around them, so a
/// clean tile inside that rectangle loses its items (keys vanish after a
/// click). One invisible pixel per tile that changes every frame makes
/// every tile repaint. Goes away with the renderer fix.
fn repaint_everything(pen: &mut Pen<'_>, w: f32, h: f32, frame: u32) -> AureaResult<()> {
    const TILE: f32 = 128.0;
    #[allow(clippy::cast_possible_truncation)]
    let marker = Color::rgba((frame & 0xFF) as u8, 0, 0, 0);
    let mut y = 0.0;
    while y < h {
        let mut x = 0.0;
        while x < w {
            pen.fill(Area::new(x, y, 1.0, 1.0), marker)?;
            x += TILE;
        }
        y += TILE;
    }
    Ok(())
}

fn header(pen: &mut Pen<'_>, w: f32, shared: &Shared, hits: &mut Hits) -> AureaResult<()> {
    let lang = pen.lang;
    pen.fill(Area::new(0.0, 0.0, w, pen.s(HEADER)), color::SURFACE)?;
    pen.fill(Area::new(0.0, pen.s(HEADER) - 1.0, w, 1.0), color::BORDER)?;
    let x = pen.s(MARGIN);
    pen.text(
        "LYKIL STUDIO",
        x,
        pen.s(12.0),
        &pen.bold(11.0),
        color::ACCENT,
    )?;
    let kb = &shared.keyboard;
    let name = if kb.connection == Connection::Connected {
        kb.name.as_str()
    } else {
        lang.tr("No keyboard")
    };
    pen.text(name, x, pen.s(28.0), &pen.bold(17.0), color::TEXT)?;

    // Tabs, centred.
    let tab_w = pen.s(118.0);
    let tab_h = pen.s(34.0);
    #[allow(clippy::cast_precision_loss)]
    let total = tab_w * Tab::ALL.len() as f32 + pen.s(8.0);
    let bar = Area::new((w - total) / 2.0, pen.s(13.0), total, tab_h + pen.s(8.0));
    pen.round(bar, pen.s(10.0), color::BACKGROUND)?;
    let pad = pen.s(4.0);
    let tab_at = |i: f32| Area::new(bar.x + pad + tab_w * i, bar.y + pad, tab_w, tab_h);
    let current = Tab::ALL
        .iter()
        .position(|t| *t == shared.ui.tab)
        .unwrap_or(0);
    #[allow(clippy::cast_precision_loss)]
    let at = pen.anim.to(Key::TabBar, current as f32, rate::SLIDE);
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

    // Connection, right.
    let (text, dot) = match &kb.connection {
        Connection::Connected => (lang.tr("Connected"), color::GOOD),
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

    // Language, left of the connection.
    let other = lang.other().label();
    // In its own language's font: the current one may not have its letters.
    let font = aurea::render::Font::new(lang.other().font_family(), pen.s(12.0));
    let lw = pen.width(other, &font) + pen.s(20.0);
    let lang = Area::new(
        right - tw - pen.s(28.0) - lw,
        pen.s(HEADER / 2.0 - 12.0),
        lw,
        pen.s(24.0),
    );
    let t = pen.hover(lang, Hit::Lang);
    pen.round(
        lang,
        pen.s(12.0),
        color::mix(color::RAISED, color::HOVER, t),
    )?;
    pen.centred(other, lang, &font, color::DIM)?;
    hits.push((lang, Hit::Lang));
    Ok(())
}

/// A tab's icon and name, centred together; the name shrinks to fit.
fn tab_label(pen: &mut Pen<'_>, a: Area, tab: Tab, fg: Color) -> AureaResult<()> {
    let text = pen.lang.tr(tab.name());
    let icon = pen.s(14.0);
    let gap = pen.s(7.0);
    let room = a.w - icon - gap - pen.s(16.0);
    let mut font = pen.bold(13.0);
    while pen.width(text, &font) > room && font.size > pen.s(9.0) {
        font.size -= pen.s(0.5);
    }
    let tw = pen.width(text, &font);
    let x = a.x + (a.w - icon - gap - tw) / 2.0;
    let cy = a.y + a.h / 2.0;
    icons::tab(pen, tab, Area::new(x, cy - icon / 2.0, icon, icon), fg)?;
    pen.text(
        text,
        x + icon + gap,
        a.y + (a.h - font.size) / 2.0,
        &font,
        fg,
    )
}

/// `status` from the page, unless there is a `notice` or a refused change.
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

fn waiting(pen: &mut Pen<'_>, body: Area, kb: &Keyboard) -> AureaResult<()> {
    let lang = pen.lang;
    let card = Area::new(
        body.x + (body.w - pen.s(420.0)) / 2.0,
        body.y + body.h / 2.0 - pen.s(70.0),
        pen.s(420.0),
        pen.s(140.0),
    );
    pen.round(card, pen.s(14.0), color::SURFACE)?;
    let title = Area::new(card.x, card.y + pen.s(30.0), card.w, pen.s(24.0));
    pen.centred(
        lang.tr("Plug in a Lykil keyboard"),
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
    pen.fitted(&line, sub, 13.0, 9.0, color::DIM)
}

/// How keys are coloured and labelled.
pub enum Keys<'a> {
    /// Bindings of a layer; the selected key is outlined.
    Keymap {
        bindings: &'a [Binding],
        layers: &'a [String],
        selected: Option<usize>,
    },
    /// The lighting preview at `time` seconds; `colors` are the per-key
    /// colours by LED, `points` each key's place for the effects, `brush`
    /// the per-key brush shown on the key under the mouse.
    Lighting {
        settings: Settings,
        time: f32,
        colors: &'a [Rgb],
        points: &'a [Option<Point>],
        presses: &'a Presses<'a>,
        brush: Option<Rgb>,
    },
    /// Key ids and what is pressed.
    Device,
}

/// Draws the keyboard in `area`; returns the area it used.
pub fn keyboard(
    pen: &mut Pen<'_>,
    area: Area,
    kb: &Keyboard,
    keys: &Keys<'_>,
    hover_keys: bool,
    hits: &mut Hits,
) -> AureaResult<Area> {
    let Some(desc) = &kb.description else {
        return Ok(Area::default());
    };
    let Some((x0, y0, x1, y1)) = bounds(desc) else {
        return Ok(Area::default());
    };
    #[allow(clippy::cast_possible_truncation)]
    let (units_w, units_h) = ((x1 - x0) as f32, (y1 - y0) as f32);
    let unit = (area.w / units_w)
        .min(area.h / units_h)
        .min(pen.s(MAX_UNIT));
    let used = Area::new(
        area.x + (area.w - unit * units_w) / 2.0,
        area.y,
        unit * units_w,
        unit * units_h,
    );
    let gap = (unit * 0.08).max(2.0);
    for (i, key) in desc.keys.iter().enumerate() {
        let Some([x, y, kw, kh]) = key.geometry else {
            continue;
        };
        #[allow(clippy::cast_possible_truncation)]
        let cap = Area::new(
            used.x + (x - x0) as f32 * unit + gap / 2.0,
            used.y + (y - y0) as f32 * unit + gap / 2.0,
            kw as f32 * unit - gap,
            kh as f32 * unit - gap,
        );
        #[allow(clippy::cast_possible_truncation)]
        let info = Cap {
            index: i,
            id: &key.id,
            led: key.led,
            down: key.cell.is_some_and(|c| kb.closed(c)),
            hovered: if hover_keys {
                pen.hover(cap, Hit::Key(i))
            } else {
                0.0
            },
        };
        keycap(pen, cap, &info, keys)?;
        if hover_keys {
            hits.push((cap, Hit::Key(i)));
        }
    }
    Ok(used)
}

/// One key being drawn.
struct Cap<'a> {
    /// Description index.
    index: usize,
    id: &'a str,
    led: Option<u16>,
    down: bool,
    /// How hovered, 0 to 1.
    hovered: f32,
}

fn keycap(pen: &mut Pen<'_>, cap: Area, info: &Cap<'_>, keys: &Keys<'_>) -> AureaResult<()> {
    let Cap {
        index,
        id,
        led: _,
        down,
        hovered,
    } = *info;
    let radius = pen.s(6.0);
    let (face, selected) = match keys {
        Keys::Lighting { .. } => (lit_face(info, keys), false),
        Keys::Keymap { selected, .. } => (
            color::mix(color::RAISED, color::HOVER, hovered),
            *selected == Some(index),
        ),
        Keys::Device => (color::RAISED, false),
    };
    // A pressed key glows up at once and fades out after release.
    let glow = pen.anim.towards(
        Key::Down(index),
        if down { 1.0 } else { 0.0 },
        rate::PRESS,
        rate::RELEASE,
    );
    let face = if matches!(keys, Keys::Device) {
        color::mix(face, color::PRESSED, glow * 0.55)
    } else {
        face
    };
    // Pressed caps sink a little.
    let sink = pen.s(1.5) * glow;
    let cap = Area::new(cap.x, cap.y + sink, cap.w, cap.h - sink);
    // A darker skirt under the face gives the cap some depth.
    pen.round(cap, radius, color::mix(face, color::BACKGROUND, 0.45))?;
    let top = Area::new(
        cap.x + pen.s(2.0),
        cap.y + pen.s(1.0),
        cap.w - pen.s(4.0),
        cap.h - pen.s(5.0) + sink,
    );
    pen.round(top, radius * 0.8, face)?;
    if glow > 0.01 {
        pen.outline(
            cap,
            radius,
            pen.s(2.0),
            color::mix(face, color::PRESSED, glow),
        )?;
    }
    let sel = pen.anim.to(
        Key::Selected(index),
        if selected { 1.0 } else { 0.0 },
        rate::HOVER,
    );
    if sel > 0.01 {
        pen.outline(
            cap,
            radius,
            pen.s(1.0) + pen.s(1.5) * sel,
            color::mix(face, color::ACCENT, sel),
        )?;
    }
    let label = top.inset(pen.s(3.0));
    match keys {
        Keys::Keymap {
            bindings, layers, ..
        } => {
            let binding = bindings.get(index).copied().unwrap_or_default();
            let (main, sub) = legend::keycap(binding, layers);
            let c = if binding == Binding::Transparent {
                color::FAINT
            } else {
                color::TEXT
            };
            match sub {
                Some(sub) => {
                    let upper = Area::new(label.x, label.y, label.w, label.h * 0.62);
                    let lower =
                        Area::new(label.x, label.y + label.h * 0.58, label.w, label.h * 0.38);
                    pen.fitted(&main, upper, 12.0, 7.0, c)?;
                    pen.fitted(&sub, lower, 9.0, 6.0, color::ACCENT)
                }
                None => pen.fitted(&main, label, 12.0, 7.0, c),
            }
        }
        Keys::Device => pen.fitted(id, label, 10.0, 6.0, color::DIM),
        Keys::Lighting { .. } => Ok(()),
    }
}

/// A key's face in the lighting preview; the per-key brush shows on the
/// hovered key.
fn lit_face(info: &Cap<'_>, keys: &Keys<'_>) -> Color {
    let Keys::Lighting {
        settings,
        time,
        colors,
        points,
        presses,
        brush,
    } = keys
    else {
        return color::RAISED;
    };
    let lit = if settings.effect == Effect::PerKey {
        info.led
            .and_then(|l| colors.get(usize::from(l)).copied())
            .unwrap_or(Rgb::OFF)
            .scale(settings.color.v)
    } else {
        points
            .get(info.index)
            .copied()
            .flatten()
            .map_or(Rgb::OFF, |p| {
                lights::preview(*settings, p, info.index, *time, presses, points)
            })
    };
    let face = color::mix(color::SURFACE, lights::rgb(lit), 0.9);
    match brush {
        Some(b) if info.led.is_some() => color::mix(face, lights::rgb(*b), info.hovered * 0.6),
        _ => color::mix(face, color::TEXT, info.hovered * 0.15),
    }
}

/// Left, top, right and bottom of the keys with geometry, in units.
fn bounds(desc: &Description) -> Option<(f64, f64, f64, f64)> {
    let mut b: Option<(f64, f64, f64, f64)> = None;
    for [x, y, w, h] in desc.keys.iter().filter_map(|k| k.geometry) {
        let (l, t, r, bo) = b.unwrap_or((x, y, x + w, y + h));
        b = Some((l.min(x), t.min(y), r.max(x + w), bo.max(y + h)));
    }
    b
}

/// Pills in a row; returns the right edge.
pub fn pills(
    pen: &mut Pen<'_>,
    x: f32,
    y: f32,
    items: &[(String, Hit, bool)],
    hits: &mut Hits,
) -> AureaResult<f32> {
    let font = pen.bold(12.0);
    let mut x = x;
    for (label, hit, active) in items {
        let w = pen.width(label, &font) + pen.s(24.0);
        let a = Area::new(x, y, w, pen.s(30.0));
        let hover = pen.hover(a, *hit);
        let bg = if *active {
            color::ACCENT
        } else {
            color::mix(color::RAISED, color::HOVER, hover)
        };
        pen.round(a, pen.s(15.0), bg)?;
        let fg = if *active {
            color::ACCENT_TEXT
        } else {
            color::TEXT
        };
        pen.centred(label, a, &font, fg)?;
        hits.push((a, *hit));
        x += w + pen.s(8.0);
    }
    Ok(x)
}

pub fn label(pen: &mut Pen<'_>, text: &str, x: f32, y: f32) -> AureaResult<()> {
    pen.text(text, x, y, &pen.bold(11.0), color::FAINT)
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
    let used = keyboard(
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
        pen.centred(
            lang.tr("Click a key to change what it does"),
            panel,
            &pen.font(14.0),
            color::DIM,
        )?;
        return Ok(hovered.unwrap_or_else(|| {
            lang.tr("Click a key, then pick a binding. Changes are saved on the keyboard.")
                .into()
        }));
    };
    let current = bindings.get(index).copied().unwrap_or_default();

    // Left: the key.
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

    // Right: the palette.
    let right = Area::new(
        left.right() + pen.s(24.0),
        panel.y + pen.s(18.0),
        panel.right() - left.right() - pen.s(44.0),
        panel.h - pen.s(36.0),
    );
    palette(
        pen,
        right,
        &layers,
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

/// The selected key: its keycap, what it does, and its hold and modifier
/// controls.
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

/// `id: binding` of the key under the mouse, for the footer.
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

/// "When held" and "send with" for the selected key, where they apply.
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

/// Small pills that wrap inside `area`; returns the bottom of the last
/// row.
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

/// Layer pills on the left, reset on the right.
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

/// A keycap-shaped button with a main and an optional small line.
fn chip(
    pen: &mut Pen<'_>,
    (a, hit): (Area, Hit),
    main: &str,
    sub: Option<&str>,
    selected: bool,
) -> AureaResult<()> {
    let t = pen.hover(a, hit);
    let face = if selected {
        color::ACCENT
    } else {
        color::mix(color::RAISED, color::HOVER, t)
    };
    // Hovered chips lift a little.
    let a = Area::new(a.x, a.y - pen.s(1.5) * t, a.w, a.h);
    pen.round(a, pen.s(7.0), color::mix(face, color::BACKGROUND, 0.45))?;
    let top = Area::new(
        a.x + pen.s(2.0),
        a.y + pen.s(1.0),
        a.w - pen.s(4.0),
        a.h - pen.s(5.0),
    );
    pen.round(top, pen.s(6.0), face)?;
    let fg = if selected {
        color::ACCENT_TEXT
    } else {
        color::TEXT
    };
    let inner = top.inset(pen.s(3.0));
    match sub {
        Some(sub) => {
            let upper = Area::new(inner.x, inner.y, inner.w, inner.h * 0.62);
            let lower = Area::new(inner.x, inner.y + inner.h * 0.58, inner.w, inner.h * 0.38);
            pen.fitted(main, upper, 12.0, 7.0, fg)?;
            pen.fitted(
                sub,
                lower,
                9.0,
                6.0,
                if selected { fg } else { color::ACCENT },
            )
        }
        None => pen.fitted(main, inner, 12.0, 7.0, fg),
    }
}

/// Group names down the left, the chosen group's bindings as keycaps on
/// the right.
fn palette(
    pen: &mut Pen<'_>,
    area: Area,
    layers: &[String],
    current: Binding,
    (chosen, macros): (usize, bool),
    hits: &mut Hits,
) -> AureaResult<()> {
    let lang = pen.lang;
    let groups = legend::palette(layers, macros);
    let list_w = pen.s(150.0);
    #[allow(clippy::cast_precision_loss)]
    let row_h = pen.s(24.0).min(area.h / groups.len().max(1) as f32);
    let font = pen.font(12.0);
    for (i, group) in groups.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let a = Area::new(
            area.x,
            area.y + row_h * i as f32,
            list_w,
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
        pen.text(
            lang.tr(group.name),
            a.x + pen.s(12.0),
            a.y + pen.s(5.0),
            &font,
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
        hits.push((a, Hit::Palette(*b)));
        x += w + gap;
    }
    Ok(())
}

/// Every macro slot with what it types.
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

/// Multi-line text in `field`, with a caret at the end while editing.
fn text_field(pen: &mut Pen<'_>, field: Area, text: &str, editing: bool) -> AureaResult<()> {
    let lang = pen.lang;
    let font = pen.font(15.0);
    let mut ty = field.y + pen.s(12.0);
    let lines: Vec<&str> = text.split('\n').collect();
    for (n, line) in lines.iter().enumerate() {
        let shown = if editing && n + 1 == lines.len() {
            format!("{line}|")
        } else {
            (*line).to_string()
        };
        pen.text(&shown, field.x + pen.s(12.0), ty, &font, color::TEXT)?;
        ty += pen.s(22.0);
        if ty > field.bottom() - pen.s(20.0) {
            break;
        }
    }
    if text.is_empty() && !editing {
        pen.text(
            lang.tr("Start typing: this macro will type the same text."),
            field.x + pen.s(12.0),
            field.y + pen.s(12.0),
            &font,
            color::FAINT,
        )?;
    }
    Ok(())
}

/// How to use macro `id`, under the editor.
fn macro_note(pen: &mut Pen<'_>, editor: Area, id: usize) -> AureaResult<()> {
    let lang = pen.lang;
    let note = Area::new(
        editor.x,
        editor.bottom() + pen.s(16.0),
        editor.w,
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

/// Macro slots on the left, the chosen macro's text on the right.
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
        pen.s(300.0),
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
    let y = field.bottom() + pen.s(30.0);
    let mut buttons = Vec::new();
    if editing && !full {
        buttons.push((
            lang.tr("Save to keyboard").to_string(),
            Hit::SaveMacro,
            true,
        ));
    }
    if !saved.is_empty() {
        buttons.push((lang.tr("Clear").to_string(), Hit::ClearMacro, false));
    }
    pills(pen, x, y, &buttons, hits)?;
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
    let used = keyboard(pen, kb_area, kb, &Keys::Device, false, hits)?;
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
    match &kb.firmware {
        Some(fw) => {
            cards.push(("FIRMWARE", format!("{} {}", fw.name, fw.version)));
            cards.push(("LYKIL", fw.lykil.to_string()));
            cards.push(("CHIP ID", fw.id.clone()));
        }
        None => cards.push(("FIRMWARE", lang.tr("too old to say").to_string())),
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

/// `1234567` as `1,234,567`.
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
