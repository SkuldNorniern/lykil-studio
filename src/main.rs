//! Lykil Studio.

mod anim;
mod app;
mod colour;
mod devices;
mod draw;
mod edit;
mod icons;
mod keyboard;
mod lamps;
mod lang;
mod legend;
mod pages;
mod view;
mod widgets;

use std::path::PathBuf;
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
const FRAME: Duration = Duration::from_millis(16);

/// Where Studio keeps its files: the VIA folder and the desk.
fn data_dir() -> PathBuf {
    let var = |name| std::env::var_os(name).map(PathBuf::from);
    let base = if cfg!(windows) {
        var("APPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|h| h.join(".config")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("Lykil Studio")
}

/// Opens a folder or link in the system's file manager or browser.
fn reveal(target: impl AsRef<std::ffi::OsStr>) {
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener).arg(target).spawn();
}

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
        s.ui.lang = lang::Lang::detect();
    }
    let tx = devices::spawn(Arc::clone(&shared), canvas.id());
    let lamps = lamps::spawn(Arc::clone(&shared));
    lock(&shared).lamp_tx = Some(lamps);

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
            s.event(&event, &tx) || matches!(event, WindowEvent::Resized { .. })
        };
        if redraw {
            request_canvas_redraw(id);
        }
    });

    // Frames come only while something moves.
    let ticking = Arc::clone(&shared);
    thread::spawn(move || {
        loop {
            thread::sleep(FRAME);
            let s = lock(&ticking);
            let moving = s.ui.anim.busy() || s.animating();
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
