//! Effect cards in a grid: a name, a line about it and a strip playing it,
//! on pages when they do not all fit.

use aurea::AureaResult;
use lykil::lighting::{Effect, Rgb, Settings};

use crate::app::Hit;
use crate::components::led_strip::led_strip;
use crate::components::surface::{choice, dim};
use crate::draw::{Area, Hits, Pen, color};
use crate::widgets::pager::pager;

/// One effect to pick.
pub struct EffectCard<'a> {
    pub name: &'a str,
    /// A line about it; empty for none.
    pub about: &'a str,
    pub hit: Hit,
    pub active: bool,
    /// The device runs it. Otherwise the card is greyed out and `about`
    /// should say why.
    pub runs: bool,
    /// What its strip plays: these settings with this effect.
    pub plays: (Settings, Effect),
}

/// What the strips need besides their cards.
#[derive(Clone, Copy)]
pub struct Strips<'a> {
    /// Seconds, for the animation.
    pub time: f32,
    /// For per-key cards.
    pub key_colors: &'a [Rgb],
}

/// Cards four across. If they do not all fit, they go on pages of whole
/// rows with a pager under them; `page` is the one shown.
pub fn effect_grid(
    pen: &mut Pen<'_>,
    area: Area,
    cards: &[EffectCard<'_>],
    (page, strips): (usize, Strips<'_>),
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
        effect_card(pen, a, (card, i), strips)?;
        hits.push((a, card.hit));
    }
    if pages > 1 {
        let under = Area::new(area.x, area.bottom() - pen.s(22.0), area.w, pen.s(22.0));
        pager(pen, under, (page, pages), Hit::EffectPage, hits)?;
    }
    Ok(())
}

/// One card: the name, shrunk then cut short if long, the line about it
/// and, when there is room, its strip.
fn effect_card(
    pen: &mut Pen<'_>,
    a: Area,
    (card, i): (&EffectCard<'_>, usize),
    strips: Strips<'_>,
) -> AureaResult<()> {
    let fg = choice(pen, a, (card.hit, i), card.active)?;
    let fg = if card.runs { fg } else { color::FAINT };
    let inner = a.inset(pen.s(10.0));
    let title = Area::new(inner.x, inner.y, inner.w, pen.s(16.0));
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
        led_strip(pen, bar, card.plays, strips.time, strips.key_colors)?;
    }
    if !card.runs {
        dim(pen, a)?;
    }
    Ok(())
}
