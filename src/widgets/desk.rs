//! The desk on the Windows page: every Dynamic Lighting device where it
//! sits, its lamps in their live colours.

use aurea::AureaResult;

use crate::app::{Hit, Shared};
use crate::colour::rgb;
use crate::components::surface::well;
use crate::components::text::hint;
use crate::draw::{Area, Hits, Pen, color};
use crate::widgets::status::notice;

/// The desk: every device where it sits, its lamps in their live colours.
/// Drag a device to move it.
pub fn desk(pen: &mut Pen<'_>, area: Area, shared: &Shared, hits: &mut Hits) -> AureaResult<()> {
    let lang = pen.lang;
    well(pen, area, false)?;
    let (mut left, mut top, mut right, mut bottom) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for d in &shared.lamps {
        left = left.min(d.place.x);
        top = top.min(d.place.y);
        right = right.max(d.place.x + d.size.0);
        bottom = bottom.max(d.place.y + d.size.1);
    }
    let pad = pen.s(24.0);
    let scale = ((area.w - 2.0 * pad) / (right - left).max(0.05))
        .min((area.h - 2.0 * pad - pen.s(14.0)) / (bottom - top).max(0.05))
        .max(1.0);
    let ox = area.x + (area.w - (right - left) * scale) / 2.0;
    let oy = area.y + (area.h - (bottom - top) * scale) / 2.0;
    let settings = shared.lighting().filter(|_| shared.lamp_sync);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let now = lykil::time::Tick((pen.anim.time() * 1000.0) as u32);
    let colours = settings.map(|s| crate::lamps::colours(&shared.lamps, s, now));
    // The picked device last, so it is on top where devices overlap.
    let picked_id = shared.ui.desk_selected.as_deref();
    let mut order: Vec<usize> = (0..shared.lamps.len()).collect();
    order.sort_by_key(|&i| Some(shared.lamps[i].id.as_str()) == picked_id);
    for (i, d) in order.into_iter().map(|i| (i, &shared.lamps[i])) {
        let rect = Area::new(
            ox + (d.place.x - left) * scale,
            oy + (d.place.y - top) * scale,
            d.size.0 * scale,
            d.size.1 * scale,
        );
        let hover = pen.hover(rect, Hit::DeskDevice(i));
        let face = if d.place.follow {
            color::RAISED
        } else {
            color::mix(color::BACKGROUND, color::RAISED, 0.5)
        };
        pen.round(rect, pen.s(6.0), color::mix(face, color::HOVER, hover))?;
        let crowded = shared.lamps.iter().any(|o| {
            o.id != d.id && crate::lamps::desk::overlaps((d.place, d.size), (o.place, o.size))
        });
        if crowded {
            pen.outline(rect, pen.s(6.0), pen.s(2.0), color::BAD)?;
        } else if picked_id == Some(d.id.as_str()) {
            pen.outline(rect, pen.s(6.0), pen.s(2.0), color::ACCENT)?;
        }
        let dot = (pen.s(2.5) + scale * 0.004).min(pen.s(5.0));
        for (k, (lx, ly)) in d.lamps.iter().enumerate() {
            let c = colours
                .as_ref()
                .filter(|_| d.place.follow)
                .and_then(|c| c.get(i)?.get(k).copied())
                .map_or(color::FAINT, rgb);
            pen.circle(rect.x + lx * scale, rect.y + ly * scale, dot, c)?;
        }
        // Inside the device, so names never run into a neighbour.
        let name = Area::new(
            rect.x + pen.s(6.0),
            rect.y + pen.s(4.0),
            (rect.w - pen.s(12.0)).max(pen.s(10.0)),
            pen.s(12.0).min(rect.h),
        );
        pen.fitted_left(&d.name, name, 9.0, 6.0, color::DIM)?;
        hits.push((rect, Hit::DeskDevice(i)));
    }
    let top_line = Area::new(
        area.x + pen.s(10.0),
        area.y + pen.s(6.0),
        area.w,
        pen.s(12.0),
    );
    hint(pen, top_line, lang.tr("Drag the devices to where they sit"))?;
    if !shared.lamp_sync {
        let bar = Area::new(
            area.x + pen.s(10.0),
            area.bottom() - pen.s(44.0),
            area.w - pen.s(20.0),
            pen.s(34.0),
        );
        notice(
            pen,
            bar,
            lang.tr("Windows controls these lights now. Studio only shows where they sit."),
            color::DIM,
        )?;
    }
    Ok(())
}
