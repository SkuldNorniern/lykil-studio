//! Lykil Studio.
//!
//! Finds a connected Lykil keyboard and manages it: the keymap (click a
//! key, pick a binding), lighting (effect, colour, speed, and whether
//! Windows Dynamic Lighting may take over), and the device (live key
//! test, counters). The keyboard describes itself, so Studio needs no
//! files for it.
//!
//! The window is one canvas drawn by [`view`]; input goes through
//! [`app`], the keyboard lives on a thread in [`device`].

mod app;
mod device;
mod draw;
mod legend;
mod view;

use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use aurea::elements::{Orientation, Stack};
use aurea::render::{Canvas, RendererBackend, request_canvas_redraw};
use aurea::{Container, Window, WindowEvent};

use crate::app::Shared;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 800;
/// Lighting preview frame time.
const PREVIEW_FRAME: Duration = Duration::from_millis(33);

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
    canvas.set_background_color(draw::color::BACKGROUND);

    let shared = Arc::new(Mutex::new(Shared::default()));
    lock(&shared).ui.scale = canvas.scale_factor();
    let tx = device::spawn(Arc::clone(&shared), canvas.id());

    let drawn = Arc::clone(&shared);
    canvas.set_draw_callback(move |ctx| {
        let result = view::draw(ctx, &mut lock(&drawn));
        if let Err(e) = &result {
            eprintln!("draw: {e}");
        }
        result
    })?;

    let input = Arc::clone(&shared);
    let id = canvas.id();
    window.on_event(move |event| {
        let redraw = {
            let mut s = lock(&input);
            if let WindowEvent::ScaleFactorChanged { scale_factor } = event {
                s.ui.scale = scale_factor;
            }
            s.event(&event, &tx) || matches!(event, WindowEvent::Resized { .. })
        };
        if redraw {
            request_canvas_redraw(id);
        }
    });

    // The lighting preview runs only while it has something to show.
    let preview = Arc::clone(&shared);
    thread::spawn(move || {
        loop {
            thread::sleep(PREVIEW_FRAME);
            let mut s = lock(&preview);
            if s.animating() {
                s.ui.time += PREVIEW_FRAME.as_secs_f32();
                drop(s);
                request_canvas_redraw(id);
            }
        }
    });

    let mut layout = Stack::new(Orientation::Vertical)?;
    layout.add(canvas)?;
    window.set_content(layout)?;
    window.show();
    window.run()
}

fn lock(shared: &Mutex<Shared>) -> std::sync::MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
