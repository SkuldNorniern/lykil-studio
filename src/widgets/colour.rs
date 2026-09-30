//! Colour controls: the hue bar and the cached gradient images behind
//! the colour square.

use std::cell::RefCell;

use aurea::AureaResult;
use aurea::render::{Color, Image, Rect};
use lykil::lighting::{Hsv, Rgb};

use super::ring;
use crate::app::Hit;
use crate::draw::{Area, Hits, Pen, color};

thread_local! {
    static SQUARE: RefCell<Option<((u8, u32), Image)>> = const { RefCell::new(None) };
    static HUES: RefCell<Option<((u32, u32), Image)>> = const { RefCell::new(None) };
}

pub fn square_image(h: u8, size: u32) -> Image {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let last = size.saturating_sub(1).max(1);
    for y in 0..size {
        for x in 0..size {
            #[allow(clippy::cast_possible_truncation)]
            let c = Hsv::new(h, (x * 255 / last) as u8, (255 - y * 255 / last) as u8).to_rgb();
            data.extend_from_slice(&[c.r, c.g, c.b, 255]);
        }
    }
    Image::new(size, size, data)
}

pub fn hue_image(w: u32, h: u32) -> Image {
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    let last = w.saturating_sub(1).max(1);
    for _ in 0..h {
        for x in 0..w {
            #[allow(clippy::cast_possible_truncation)]
            let c = Hsv::new((x * 254 / last) as u8, 255, 255).to_rgb();
            data.extend_from_slice(&[c.r, c.g, c.b, 255]);
        }
    }
    Image::new(w, h, data)
}

pub fn cached<K: PartialEq + Copy>(
    pen: &mut Pen<'_>,
    cell: &'static std::thread::LocalKey<RefCell<Option<(K, Image)>>>,
    key: K,
    area: Area,
    make: impl FnOnce() -> Image,
) -> AureaResult<()> {
    let image = cell.with(|c| {
        let mut c = c.borrow_mut();
        match &*c {
            Some((k, img)) if *k == key => img.clone(),
            _ => {
                let img = make();
                *c = Some((key, img.clone()));
                img
            }
        }
    });
    pen.ctx
        .draw_image_rect(&image, Rect::new(area.x, area.y, area.w, area.h))
}

/// The rainbow bar with a ring at `hue`; dragging it is `hit`.
pub fn hue_bar(
    pen: &mut Pen<'_>,
    bar: Area,
    hue: u8,
    hit: Hit,
    hits: &mut Hits,
) -> AureaResult<()> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bar_px = (bar.w.round() as u32, bar.h.round().max(1.0) as u32);
    cached(pen, &HUES, bar_px, bar, || hue_image(bar_px.0, bar_px.1))?;
    let hx = bar.x + bar.w * f32::from(hue.min(254)) / 254.0;
    ring(
        pen,
        hx,
        bar.y + bar.h / 2.0,
        pen.s(8.0),
        rgb(Hsv::new(hue, 255, 255).to_rgb()),
    )?;
    hits.push((
        Area::new(bar.x, bar.y - pen.s(6.0), bar.w, bar.h + pen.s(12.0)),
        hit,
    ));
    Ok(())
}

pub fn square_and_bar(pen: &mut Pen<'_>, square: Area, c: Hsv, hits: &mut Hits) -> AureaResult<()> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = square.w.round() as u32;
    cached(pen, &SQUARE, (c.h, px), square, || square_image(c.h, px))?;
    pen.outline(square, pen.s(2.0), pen.s(1.0), color::BORDER)?;
    let (kx, ky) = (
        square.x + square.w * f32::from(c.s) / 255.0,
        square.y + square.h * (1.0 - f32::from(c.v) / 255.0),
    );
    ring(pen, kx, ky, pen.s(7.0), rgb(c.to_rgb()))?;
    hits.push((square, Hit::Square));

    let bar = Area::new(
        square.x,
        square.bottom() + pen.s(12.0),
        square.w,
        pen.s(14.0),
    );
    hue_bar(pen, bar, c.h, Hit::HueBar, hits)
}

pub fn rgb(c: Rgb) -> Color {
    Color::rgb(c.r, c.g, c.b)
}
