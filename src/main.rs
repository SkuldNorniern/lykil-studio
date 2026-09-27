//! Lykil Studio.
//!
//! `lykil-studio [project dir]`: finds a connected Lykil keyboard and shows
//! what it reports: device info, live diagnostics, and the keyboard with
//! the keys that are down right now. With the project directory the keys
//! are drawn from `layout.tav` and placed through `board.tav`; without it
//! the raw matrix is drawn as a grid.
//!
//! A background thread talks to the keyboard through `lykil-device`; the
//! canvas is redrawn when what it shows changes.

mod device;
mod view;

use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use aurea::elements::{Orientation, Stack};
use aurea::render::{Canvas, Color, RendererBackend};
use aurea::{Container, Window};

const WIDTH: u32 = 1040;
const HEIGHT: u32 = 520;

fn main() -> ExitCode {
    let project = std::env::args().nth(1).map(|dir| load(Path::new(&dir)));
    let project = match project {
        None => None,
        Some(Ok(ir)) => Some(ir),
        Some(Err(e)) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match run(project) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run(project: Option<lykil_config::FirmwareIr>) -> aurea::AureaResult<()> {
    let mut window = Window::new("Lykil Studio", WIDTH.cast_signed(), HEIGHT.cast_signed())?;
    let canvas = Canvas::new(WIDTH, HEIGHT, RendererBackend::Cpu)?;
    canvas.set_background_color(Color::rgb(18, 20, 24));

    let state = Arc::new(Mutex::new(device::State::default()));
    device::spawn(Arc::clone(&state), canvas.id());

    let view = view::View::new(project);
    let shown = Arc::clone(&state);
    canvas.set_draw_callback(move |ctx| {
        let state = shown
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        view.draw(ctx, &state)
    })?;

    let mut layout = Stack::new(Orientation::Vertical)?;
    layout.add(canvas)?;
    window.set_content(layout)?;
    window.show();
    window.run()
}

/// Compiles the project in `dir` for its layout and board.
fn load(dir: &Path) -> Result<lykil_config::FirmwareIr, String> {
    let read = |file: &str| {
        std::fs::read_to_string(dir.join(file))
            .map_err(|e| format!("{}: {e}", dir.join(file).display()))
    };
    let (project, layout, keymap) = (
        read("project.tav")?,
        read("layout.tav")?,
        read("keymap.tav")?,
    );
    let board = read("board.tav").ok();
    lykil_config::compile(&lykil_config::Sources {
        project: &project,
        layout: &layout,
        keymap: &keymap,
        board: board.as_deref(),
    })
    .map_err(|d| d.to_string())
}
