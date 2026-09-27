//! Drawing: header, diagnostics, and the keyboard.

use aurea::AureaResult;
use aurea::render::{Color, DrawingContext, Font, Paint, PaintStyle, Point, Rect};
use lykil_config::FirmwareIr;
use lykil_config::ir::Diode;
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

/// One key to draw: where, and which matrix cell reads it.
struct Key {
    rect: Rect,
    label: String,
    cell: Option<(usize, usize)>,
}

pub struct View {
    keys: Vec<Key>,
    /// Driven lines are columns (row-to-col diodes) or rows.
    columns_driven: bool,
}

impl View {
    pub fn new(project: Option<FirmwareIr>) -> Self {
        let Some(ir) = project else {
            return Self {
                keys: Vec::new(),
                columns_driven: true,
            };
        };
        let matrix = ir.board.as_ref().and_then(|b| b.matrix.as_ref());
        let mut cells = vec![None; ir.layout.keys.len()];
        if let Some(m) = matrix {
            for (row, cols) in m.positions.iter().enumerate() {
                for (col, position) in cols.iter().enumerate() {
                    if let Some(p) = position {
                        cells[*p] = Some((row, col));
                    }
                }
            }
        }
        let keys = ir
            .layout
            .keys
            .iter()
            .zip(cells)
            .filter_map(|(key, cell)| {
                let g = key.geometry;
                let (x, y) = (g.x?, g.y?);
                #[allow(clippy::cast_possible_truncation)]
                let rect = Rect::new(
                    MARGIN + x as f32 * UNIT,
                    KEYBOARD_TOP + y as f32 * UNIT,
                    g.w.unwrap_or(1.0) as f32 * UNIT - GAP,
                    g.h.unwrap_or(1.0) as f32 * UNIT - GAP,
                );
                Some(Key {
                    rect,
                    label: short(&key.id),
                    cell,
                })
            })
            .collect();
        Self {
            keys,
            columns_driven: matrix.is_none_or(|m| m.diode == Diode::RowToCol),
        }
    }

    pub fn draw(&self, ctx: &mut dyn DrawingContext, state: &State) -> AureaResult<()> {
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
        if self.keys.is_empty() {
            self.draw_grid(ctx, state, &body)
        } else {
            self.draw_keys(ctx, state)
        }
    }

    fn closed(&self, matrix: &[u32], (row, col): (usize, usize)) -> bool {
        let (line, bit) = if self.columns_driven {
            (col, row)
        } else {
            (row, col)
        };
        matrix
            .get(line)
            .is_some_and(|bits| bit < 32 && bits >> bit & 1 == 1)
    }

    fn draw_keys(&self, ctx: &mut dyn DrawingContext, state: &State) -> AureaResult<()> {
        let label = Font::new("", 11.0);
        for key in &self.keys {
            let down = key.cell.is_some_and(|c| self.closed(&state.matrix, c));
            ctx.draw_rect(key.rect, &fill(if down { KEY_DOWN } else { KEY }))?;
            let text = if down { Color::rgb(10, 20, 30) } else { DIM };
            ctx.draw_text_with_font(
                &key.label,
                Point::new(key.rect.x + 5.0, key.rect.y + 16.0),
                &label,
                &fill(text),
            )?;
        }
        Ok(())
    }

    /// Without a project: every matrix cell as a small square.
    fn draw_grid(
        &self,
        ctx: &mut dyn DrawingContext,
        state: &State,
        font: &Font,
    ) -> AureaResult<()> {
        let Some(h) = state.hello else {
            return Ok(());
        };
        ctx.draw_text_with_font(
            "raw matrix (pass the project directory to see the layout)",
            Point::new(MARGIN, KEYBOARD_TOP - 12.0),
            font,
            &fill(DIM),
        )?;
        let size = 26.0;
        for row in 0..usize::from(h.matrix_rows) {
            for col in 0..usize::from(h.matrix_cols) {
                let down = self.closed(&state.matrix, (row, col));
                #[allow(clippy::cast_precision_loss)]
                let rect = Rect::new(
                    MARGIN + col as f32 * (size + GAP),
                    KEYBOARD_TOP + row as f32 * (size + GAP),
                    size,
                    size,
                );
                ctx.draw_rect(rect, &fill(if down { KEY_DOWN } else { KEY }))?;
            }
        }
        Ok(())
    }
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
