//! The whole window, drawn on one canvas.

use aurea::AureaResult;
use aurea::render::{Color, DrawingContext};

use crate::anim::{self, Key, rate};
use crate::app::{Hit, Shared, Tab};
use crate::devices::{Connection, Keyboard};
use crate::draw::Hits;
use crate::draw::{Area, Pen, color};
use crate::format::uptime;
use crate::pages::lighting as lights;
use crate::pages::windows;
use crate::{icons, pages};

const HEADER: f32 = 60.0;
const FOOTER: f32 = 30.0;
const MARGIN: f32 = 24.0;
pub fn draw(ctx: &mut dyn DrawingContext, shared: &mut Shared) -> AureaResult<()> {
    #[allow(clippy::cast_precision_loss)]
    let (w, h) = (ctx.width() as f32, ctx.height() as f32);
    // Aurea's context is in logical pixels and scales for the display
    // itself; scaling here as well drew everything twice as big on Retina.
    let scale = 1.0;
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
            Tab::Keymap => pages::keymap::keymap_tab(&mut pen, body, shared, &mut hits)?,
            Tab::Macros => pages::macros::macros_tab(&mut pen, body, shared, &mut hits)?,
            Tab::Lighting if shared.keyboard.via.is_some() => {
                crate::pages::via_lighting::tab(&mut pen, body, shared, &mut hits)?
            }
            Tab::Lighting => lights::tab(&mut pen, body, shared, &mut hits)?,
            Tab::Device => pages::device::device_tab(&mut pen, body, shared, &mut hits)?,
            Tab::Windows => String::new(),
        }
    } else {
        pages::connect::waiting(&mut pen, body, &shared.keyboard, &mut hits)?;
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
