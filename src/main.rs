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

mod anim;
mod app;
mod colour;
mod device;
mod draw;
mod edit;
mod icons;
mod lang;
mod legend;
mod lights;
mod via;
mod view;
mod windows;

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
/// Frame time while something moves.
const FRAME: Duration = Duration::from_millis(16);

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
    {
        let mut s = lock(&shared);
        s.ui.scale = canvas.scale_factor();
        s.ui.lang = lang::Lang::detect();
    }
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

    // Frames come only while something moves: eased values every frame,
    // the lighting preview every other one.
    let ticking = Arc::clone(&shared);
    thread::spawn(move || {
        let mut odd = false;
        loop {
            thread::sleep(FRAME);
            odd = !odd;
            let s = lock(&ticking);
            let moving = s.ui.anim.busy() || (odd && s.animating());
            drop(s);
            if moving {
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
