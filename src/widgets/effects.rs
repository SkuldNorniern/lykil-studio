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

/// Cards in as many columns as it takes to fit `area`, at least four.
/// Cards tall enough get their `about` line and a strip `strip` paints
/// (by card index); small ones show only the name.
pub fn effect_grid(
    pen: &mut Pen<'_>,
    area: Area,
    cards: &[EffectCard<'_>],
    strip: &mut dyn FnMut(&mut Pen<'_>, Area, usize) -> AureaResult<()>,
    hits: &mut Hits,
) -> AureaResult<()> {
    // Four columns of full cards if they fit; for long lists (VIA's 45
    // effects) short rows, in as few columns as keep them readable.
    let full = cards.len().div_ceil(4).max(1);
    #[allow(clippy::cast_precision_loss)]
    let full_h = (area.h - pen.s(8.0) * (full - 1) as f32) / full as f32;
    let compact = full_h < pen.s(56.0);
    let gap = pen.s(if compact { 5.0 } else { 8.0 });
    let height = |cols: usize| {
        let rows = cards.len().div_ceil(cols).max(1);
        #[allow(clippy::cast_precision_loss)]
        let h = (area.h - gap * (rows - 1) as f32) / rows as f32;
        h.min(pen.s(if compact { 30.0 } else { 110.0 }))
    };
    // No more columns than cards about 90 px wide fit, and at least two.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let fit = ((area.w + gap) / (pen.s(90.0) + gap)).floor().max(2.0) as usize;
    let mut cols = (if compact { 3 } else { 4 }).min(fit);
    while height(cols) < pen.s(24.0) && cols < 6 {
        cols += 1;
    }
    let ch = height(cols);
    #[allow(clippy::cast_precision_loss)]
    let cw = (area.w - gap * (cols - 1) as f32) / cols as f32;
    for (i, card) in cards.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let (col, row) = ((i % cols) as f32, (i / cols) as f32);
        let a = Area::new(area.x + col * (cw + gap), area.y + row * (ch + gap), cw, ch);
        if a.bottom() > area.bottom() + 1.0 {
            break;
        }
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
        if compact {
            let inner = Area::new(a.x + pen.s(10.0), a.y, a.w - pen.s(16.0), a.h);
            pen.clipped_left(card.name, inner, &pen.bold(11.0), fg)?;
            hits.push((a, card.hit));
            continue;
        }
        let inner = a.inset(pen.s(10.0));
        let title = Area::new(inner.x, inner.y, inner.w, pen.s(16.0));
        pen.clipped_left(card.name, title, &pen.bold(13.0), fg)?;
        let about = Area::new(inner.x, inner.y + pen.s(18.0), inner.w, pen.s(14.0));
        if !card.about.is_empty() && about.bottom() <= inner.bottom() {
            pen.fitted_left(card.about, about, 10.0, 7.0, color::FAINT)?;
        }
        let bar = Area::new(inner.x, inner.bottom() - pen.s(12.0), inner.w, pen.s(12.0));
        if bar.y > about.bottom() {
            strip(pen, bar, i)?;
        }
        hits.push((a, card.hit));
    }
    Ok(())
}
