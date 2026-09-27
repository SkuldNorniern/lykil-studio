//! Lykil Studio.
//!
//! Finds a connected Lykil keyboard and shows what it reports: device
//! info, live diagnostics, and the keyboard with the keys that are down
//! right now. The keyboard describes itself (keys, layers, geometry), so
//! Studio needs no files for it.
//!
//! A background thread talks to the keyboard through `lykil-device`; the
//! canvas is redrawn when what it shows changes.

mod device;
mod view;

use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use aurea::elements::{Orientation, Stack};
use aurea::render::{Canvas, Color, RendererBackend};
use aurea::{Container, Window};

const WIDTH: u32 = 1040;
const HEIGHT: u32 = 520;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> aurea::AureaResult<()> {
    let mut window = Window::new("Lykil Studio", WIDTH.cast_signed(), HEIGHT.cast_signed())?;
    let canvas = Canvas::new(WIDTH, HEIGHT, RendererBackend::Cpu)?;
    canvas.set_background_color(Color::rgb(18, 20, 24));

    let state = Arc::new(Mutex::new(device::State::default()));
    device::spawn(Arc::clone(&state), canvas.id());

    let shown = Arc::clone(&state);
    canvas.set_draw_callback(move |ctx| {
        let state = shown
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        view::draw(ctx, &state)
    })?;

    let mut layout = Stack::new(Orientation::Vertical)?;
    layout.add(canvas)?;
    window.set_content(layout)?;
    window.show();
    window.run()
}
