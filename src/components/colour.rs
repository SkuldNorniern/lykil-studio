//! The colour square and the hue strip, drawn from cached gradient
//! images.

use std::cell::RefCell;

use aurea::AureaResult;
use aurea::render::{Image, Rect};
use lykil::lighting::Hsv;

use super::dot::ring;
use crate::app::Hit;
use crate::colour::rgb;
use crate::draw::{Area, Hits, Pen, color};

thread_local! {
    static SQUARE: RefCell<Option<((u8, u32), Image)>> = const { RefCell::new(None) };
    static HUES: RefCell<Option<((u32, u32), Image)>> = const { RefCell::new(None) };
}

fn square_image(h: u8, size: u32) -> Image {
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

fn hue_image(w: u32, h: u32) -> Image {
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

/// The image `cell` holds for `key`, made again when the key changes.
fn cached<K: PartialEq + Copy>(
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

/// Saturation across and brightness up for hue `c.h`, a ring at `c`.
/// Dragging it is `hit`.
pub fn square(pen: &mut Pen<'_>, a: Area, c: Hsv, hit: Hit, hits: &mut Hits) -> AureaResult<()> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = a.w.round() as u32;
    cached(pen, &SQUARE, (c.h, px), a, || square_image(c.h, px))?;
    pen.outline(a, pen.s(2.0), pen.s(1.0), color::BORDER)?;
    let (kx, ky) = (
        a.x + a.w * f32::from(c.s) / 255.0,
        a.y + a.h * (1.0 - f32::from(c.v) / 255.0),
    );
    ring(pen, kx, ky, pen.s(7.0), rgb(c.to_rgb()))?;
    hits.push((a, hit));
    Ok(())
}

/// The rainbow strip with a ring at `hue`. Dragging it is `hit`.
pub fn hue_strip(
    pen: &mut Pen<'_>,
    bar: Area,
    hue: u8,
    hit: Hit,
    hits: &mut Hits,
) -> AureaResult<()> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let px = (bar.w.round() as u32, bar.h.round().max(1.0) as u32);
    cached(pen, &HUES, px, bar, || hue_image(px.0, px.1))?;
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

/// A box showing `c`, bordered so a dark colour still reads.
pub fn sample(pen: &mut Pen<'_>, a: Area, c: lykil::lighting::Rgb) -> AureaResult<()> {
    pen.round(a, pen.s(8.0), rgb(c))?;
    pen.outline(a, pen.s(8.0), pen.s(1.0), color::BORDER)
}
