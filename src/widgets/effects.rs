//! The grid of effect cards both lighting pages use.

use aurea::AureaResult;

use crate::anim::{Key, rate};
use crate::app::Hit;
use crate::draw::{Area, Hits, Pen, color};

/// One effect to pick.
pub struct EffectCard<'a> {
    pub name: &'a str,
    /// A line about it; empty for none.
    pub about: &'a str,
    pub hit: Hit,
    pub active: bool,
}

/// Cards four across. Each gets its `about` line and a strip `strip`
/// paints (by card index) when tall enough. If they do not all fit, they
/// go on pages of whole rows with a pager under them; `page` is the one
/// shown.
pub fn effect_grid(
    pen: &mut Pen<'_>,
    area: Area,
    cards: &[EffectCard<'_>],
    page: usize,
    strip: &mut dyn FnMut(&mut Pen<'_>, Area, usize) -> AureaResult<()>,
    hits: &mut Hits,
) -> AureaResult<()> {
    let gap = pen.s(8.0);
    // No more columns than cards about 90 px wide fit, and at least two.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let cols = ((area.w + gap) / (pen.s(90.0) + gap))
        .floor()
        .clamp(2.0, 4.0) as usize;
    let fits = |h: f32| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let rows = ((h + gap) / (pen.s(64.0) + gap)).floor().max(1.0) as usize;
        rows
    };
    let all_rows = cards.len().div_ceil(cols).max(1);
    let (grid, rows, pages) = if all_rows <= fits(area.h) {
        (area, all_rows, 1)
    } else {
        let grid = Area::new(area.x, area.y, area.w, area.h - pen.s(30.0));
        let rows = fits(grid.h);
        (grid, rows, all_rows.div_ceil(rows))
    };
    let page = page.min(pages - 1);
    #[allow(clippy::cast_precision_loss)]
    let ch = ((grid.h - gap * (rows - 1) as f32) / rows as f32).min(pen.s(110.0));
    #[allow(clippy::cast_precision_loss)]
    let cw = (grid.w - gap * (cols - 1) as f32) / cols as f32;
    let first = page * rows * cols;
    for (i, card) in cards.iter().enumerate().skip(first).take(rows * cols) {
        #[allow(clippy::cast_precision_loss)]
        let (col, row) = (((i - first) % cols) as f32, ((i - first) / cols) as f32);
        let a = Area::new(grid.x + col * (cw + gap), grid.y + row * (ch + gap), cw, ch);
        card_at(pen, a, (card, i), strip)?;
        hits.push((a, card.hit));
    }
    if pages > 1 {
        pager(
            pen,
            Area::new(area.x, area.bottom() - pen.s(22.0), area.w, pen.s(22.0)),
            page,
            pages,
            hits,
        )?;
    }
    Ok(())
}

fn card_at(
    pen: &mut Pen<'_>,
    a: Area,
    (card, i): (&EffectCard<'_>, usize),
    strip: &mut dyn FnMut(&mut Pen<'_>, Area, usize) -> AureaResult<()>,
) -> AureaResult<()> {
    let t = pen.hover(a, card.hit);
    let bg = if card.active {
        color::mix(color::RAISED, color::ACCENT, 0.18)
    } else {
        color::mix(color::RAISED, color::HOVER, t)
    };
    pen.round(a, pen.s(10.0), bg)?;
    let sel = pen.anim.to(
        Key::Selected(1000 + i),
        if card.active { 1.0 } else { 0.0 },
        rate::HOVER,
    );
    if sel > 0.01 {
        pen.outline(
            a,
            pen.s(10.0),
            pen.s(2.0),
            color::mix(bg, color::ACCENT, sel),
        )?;
    }
    let fg = if card.active { color::TEXT } else { color::DIM };
    let inner = a.inset(pen.s(10.0));
    let title = Area::new(inner.x, inner.y, inner.w, pen.s(16.0));
    // Long names get smaller first, then cut short.
    let mut font = pen.bold(13.0);
    while pen.width(card.name, &font) > title.w && font.size > pen.s(10.5) {
        font.size -= pen.s(0.5);
    }
    pen.clipped_left(card.name, title, &font, fg)?;
    let about = Area::new(inner.x, inner.y + pen.s(18.0), inner.w, pen.s(14.0));
    if !card.about.is_empty() && about.bottom() <= inner.bottom() {
        pen.fitted_left(card.about, about, 10.0, 7.0, color::FAINT)?;
    }
    let bar = Area::new(inner.x, inner.bottom() - pen.s(12.0), inner.w, pen.s(12.0));
    if bar.y > about.bottom() {
        strip(pen, bar, i)?;
    }
    Ok(())
}

/// `‹ 2 / 4 ›`, centred in `area`.
fn pager(
    pen: &mut Pen<'_>,
    area: Area,
    page: usize,
    pages: usize,
    hits: &mut Hits,
) -> AureaResult<()> {
    let mid = area.x + area.w / 2.0;
    let text = format!("{} / {pages}", page + 1);
    pen.centred(&text, area, &pen.bold(12.0), color::DIM)?;
    let side = area.h;
    let half = pen.s(44.0);
    let arrows = [
        (page.checked_sub(1), mid - half - side, -1.0),
        (Some(page + 1).filter(|p| *p < pages), mid + half, 1.0),
    ];
    for (to, x, dir) in arrows {
        let a = Area::new(x, area.y, side, side);
        let Some(to) = to else {
            continue;
        };
        let hit = Hit::EffectPage(to);
        let t = pen.hover(a, hit);
        pen.round(a, side / 2.0, color::mix(color::RAISED, color::HOVER, t))?;
        let (cx, cy, r) = (a.x + side / 2.0, a.y + side / 2.0, pen.s(4.0));
        pen.polygon(
            &[
                (cx - dir * r * 0.6, cy - r),
                (cx + dir * r * 0.8, cy),
                (cx - dir * r * 0.6, cy + r),
            ],
            color::TEXT,
        )?;
        hits.push((a, hit));
    }
    Ok(())
}
