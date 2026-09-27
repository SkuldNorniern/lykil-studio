//! Drawing: header, diagnostics, and the keyboard.

use aurea::AureaResult;
use aurea::render::{Color, DrawingContext, Font, Paint, PaintStyle, Point, Rect};
use lykil_protocol::describe::Description;
use lykil_protocol::lcp::{self, Diagnostics};

use crate::device::{Connection, State};

/// Pixels per key unit.
const UNIT: f32 = 50.0;
const KEYBOARD_TOP: f32 = 120.0;
const MARGIN: f32 = 24.0;
const GAP: f32 = 4.0;

const TEXT: Color = Color::rgb(220, 224, 230);
const DIM: Color = Color::rgb(130, 136, 146);
const KEY: Color = Color::rgb(44, 48, 56);
const KEY_DOWN: Color = Color::rgb(90, 170, 255);
const GOOD: Color = Color::rgb(110, 200, 130);
const BAD: Color = Color::rgb(235, 110, 100);

/// Draws whatever the connected keyboard describes; nothing is known
/// about any keyboard in advance.
pub fn draw(ctx: &mut dyn DrawingContext, state: &State) -> AureaResult<()> {
    ctx.clear(Color::rgb(18, 20, 24))?;
    let title = Font::new("", 22.0);
    let body = Font::new("", 14.0);
    let (heading, color) = match &state.connection {
        Connection::Searching => ("looking for a Lykil keyboard...".to_string(), DIM),
        Connection::Connected => (state.name.clone(), TEXT),
        Connection::Lost(why) => (format!("connection lost: {why}"), BAD),
    };
    ctx.draw_text_with_font(&heading, Point::new(MARGIN, 40.0), &title, &fill(color))?;
    if let Some(h) = state.hello {
        let line = format!(
            "{} layers, {} keys, {} x {} matrix, LCP {}",
            h.layers, h.keys, h.matrix_rows, h.matrix_cols, h.version
        );
        ctx.draw_text_with_font(&line, Point::new(MARGIN, 66.0), &body, &fill(DIM))?;
    }
    if let Some(d) = &state.diagnostics {
        draw_diagnostics(ctx, d, &body)?;
    }
    match &state.description {
        Some(d) if d.keys.iter().any(|k| k.geometry.is_some()) => draw_keys(ctx, state, d),
        Some(d) => draw_grid(ctx, state, d, &body),
        None => Ok(()),
    }
}

/// Is matrix cell `(row, col)` closed in the raw lines?
fn closed(d: &Description, matrix: &[u32], (row, col): (u8, u8)) -> bool {
    let (line, bit) = if d.columns_driven {
        (col, row)
    } else {
        (row, col)
    };
    matrix
        .get(usize::from(line))
        .is_some_and(|bits| bit < 32 && bits >> bit & 1 == 1)
}

fn draw_keys(ctx: &mut dyn DrawingContext, state: &State, desc: &Description) -> AureaResult<()> {
    let label = Font::new("", 11.0);
    for key in &desc.keys {
        let Some([left, top, width, height]) = key.geometry else {
            continue;
        };
        #[allow(clippy::cast_possible_truncation)]
        let rect = Rect::new(
            MARGIN + left as f32 * UNIT,
            KEYBOARD_TOP + top as f32 * UNIT,
            width as f32 * UNIT - GAP,
            height as f32 * UNIT - GAP,
        );
        let down = key.cell.is_some_and(|c| closed(desc, &state.matrix, c));
        ctx.draw_rect(rect, &fill(if down { KEY_DOWN } else { KEY }))?;
        let text = if down { Color::rgb(10, 20, 30) } else { DIM };
        ctx.draw_text_with_font(
            &short(&key.id),
            Point::new(rect.x + 5.0, rect.y + 16.0),
            &label,
            &fill(text),
        )?;
    }
    Ok(())
}

/// A keyboard without geometry: every matrix cell as a small square.
fn draw_grid(
    ctx: &mut dyn DrawingContext,
    state: &State,
    d: &Description,
    font: &Font,
) -> AureaResult<()> {
    let Some(h) = state.hello else {
        return Ok(());
    };
    ctx.draw_text_with_font(
        "raw matrix (the keyboard describes no key positions)",
        Point::new(MARGIN, KEYBOARD_TOP - 12.0),
        font,
        &fill(DIM),
    )?;
    let size = 26.0;
    for row in 0..h.matrix_rows {
        for col in 0..h.matrix_cols {
            let down = closed(d, &state.matrix, (row, col));
            let rect = Rect::new(
                MARGIN + f32::from(col) * (size + GAP),
                KEYBOARD_TOP + f32::from(row) * (size + GAP),
                size,
                size,
            );
            ctx.draw_rect(rect, &fill(if down { KEY_DOWN } else { KEY }))?;
        }
    }
    Ok(())
}

fn draw_diagnostics(ctx: &mut dyn DrawingContext, d: &Diagnostics, font: &Font) -> AureaResult<()> {
    let x = 620.0;
    let storage = match d.storage {
        1 => "ok",
        2 => "failed",
        _ => "none",
    };
    let lines = [
        format!("uptime {} s   scans {}", d.uptime_ms / 1000, d.scans),
        format!(
            "transitions {} raw, {} stable",
            d.raw_transitions, d.stable_transitions
        ),
        format!(
            "last reset {}   watchdog resets {}   storage {storage}",
            lcp::reset::name(d.reset_cause),
            d.watchdog_resets
        ),
    ];
    for (i, line) in (0u8..).zip(lines.iter()) {
        ctx.draw_text_with_font(
            line,
            Point::new(x, 30.0 + f32::from(i) * 20.0),
            font,
            &fill(DIM),
        )?;
    }
    let (health, color) = if d.faults == 0 && d.watchdog_resets == 0 {
        ("healthy".to_string(), GOOD)
    } else {
        (
            format!("faults {}, watchdog resets {}", d.faults, d.watchdog_resets),
            BAD,
        )
    };
    ctx.draw_text_with_font(&health, Point::new(x, 92.0), font, &fill(color))
}

fn fill(color: Color) -> Paint {
    Paint::new().color(color).style(PaintStyle::Fill)
}

/// A key id short enough for a 1-unit key.
fn short(id: &str) -> String {
    const NAMES: &[(&str, &str)] = &[
        ("escape", "esc"),
        ("backspace", "bksp"),
        ("left-bracket", "["),
        ("right-bracket", "]"),
        ("backslash", "\\"),
        ("semicolon", ";"),
        ("quote", "'"),
        ("comma", ","),
        ("dot", "."),
        ("slash", "/"),
        ("grave", "`"),
        ("minus", "-"),
        ("equal", "="),
        ("caps-lock", "caps"),
        ("print-screen", "prt"),
        ("scroll-lock", "scrl"),
        ("page-up", "pgup"),
        ("page-down", "pgdn"),
        ("insert", "ins"),
        ("delete", "del"),
    ];
    NAMES
        .iter()
        .find(|(long, _)| *long == id)
        .map_or_else(|| id.chars().take(6).collect(), |(_, s)| (*s).to_string())
}
