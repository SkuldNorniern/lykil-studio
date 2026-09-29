//! Drawing helpers: the palette, rounded shapes and text placement.
//!
//! Everything is in canvas pixels; [`Pen::scale`] turns design sizes
//! (at 100 % display scale) into pixels.

use aurea::AureaResult;

use crate::anim::{Anim, Key, rate};
use crate::app::Hit;
use aurea::render::{
    Color, DrawingContext, Font, FontWeight, Paint, PaintStyle, Path, PathCommand, Point, Rect,
};

pub mod color {
    use aurea::render::Color;

    pub const BACKGROUND: Color = Color::rgb(15, 17, 21);
    pub const SURFACE: Color = Color::rgb(24, 27, 33);
    pub const RAISED: Color = Color::rgb(34, 38, 46);
    pub const HOVER: Color = Color::rgb(46, 51, 61);
    pub const BORDER: Color = Color::rgb(52, 57, 68);
    pub const TEXT: Color = Color::rgb(226, 230, 236);
    pub const DIM: Color = Color::rgb(134, 141, 153);
    pub const FAINT: Color = Color::rgb(86, 92, 104);
    pub const ACCENT: Color = Color::rgb(118, 146, 255);
    pub const ACCENT_TEXT: Color = Color::rgb(12, 16, 32);
    pub const PRESSED: Color = Color::rgb(255, 196, 92);
    pub const GOOD: Color = Color::rgb(110, 205, 140);
    pub const BAD: Color = Color::rgb(238, 112, 104);

    /// `c` mixed towards `other` by `t` (0..=1).
    pub fn mix(c: Color, other: Color, t: f32) -> Color {
        let m = |a: u8, b: u8| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let v =
                (f32::from(a) + (f32::from(b) - f32::from(a)) * t.clamp(0.0, 1.0)).round() as u8;
            v
        };
        Color::rgb(m(c.r, other.r), m(c.g, other.g), m(c.b, other.b))
    }
}

/// An axis-aligned box in canvas pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Area {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Area {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Shrunk by `d` on every side.
    pub fn inset(&self, d: f32) -> Self {
        Self::new(
            self.x + d,
            self.y + d,
            (self.w - 2.0 * d).max(0.0),
            (self.h - 2.0 * d).max(0.0),
        )
    }

    fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }
}

/// A drawing context with the display scale, the mouse position and the
/// UI's eased values.
pub struct Pen<'a> {
    pub ctx: &'a mut dyn DrawingContext,
    pub scale: f32,
    pub mouse: (f32, f32),
    pub lang: crate::lang::Lang,
    pub anim: &'a mut Anim,
}

impl Pen<'_> {
    pub fn hovered(&self, area: Area) -> bool {
        area.contains(self.mouse.0, self.mouse.1)
    }

    /// How hovered `hit` at `area` is, easing between 0 and 1.
    pub fn hover(&mut self, area: Area, hit: Hit) -> f32 {
        let target = if self.hovered(area) { 1.0 } else { 0.0 };
        self.anim.to(Key::Hover(hit), target, rate::HOVER)
    }

    /// Design pixels to canvas pixels.
    pub fn s(&self, v: f32) -> f32 {
        v * self.scale
    }

    pub fn font(&self, size: f32) -> Font {
        Font::new(self.lang.font_family(), self.s(size))
    }

    pub fn bold(&self, size: f32) -> Font {
        Font::new(self.lang.font_family(), self.s(size)).with_weight(FontWeight::Bold)
    }

    /// `c` over what is there, `alpha` 0 (nothing) to 1.
    pub fn veil(&mut self, area: Area, c: Color, alpha: f32) -> AureaResult<()> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        self.ctx
            .draw_rect(area.rect(), &fill(Color::rgba(c.r, c.g, c.b, a)))
    }

    pub fn fill(&mut self, area: Area, c: Color) -> AureaResult<()> {
        self.ctx.draw_rect(area.rect(), &fill(c))
    }

    /// A filled box with rounded corners.
    pub fn round(&mut self, area: Area, radius: f32, c: Color) -> AureaResult<()> {
        let path = rounded(area, radius);
        self.ctx.draw_path(&path, &fill(c))
    }

    /// A rounded outline.
    pub fn outline(&mut self, area: Area, radius: f32, width: f32, c: Color) -> AureaResult<()> {
        let path = rounded(area.inset(width / 2.0), radius);
        self.ctx.draw_path(
            &path,
            &Paint::new()
                .color(c)
                .style(PaintStyle::Stroke)
                .stroke_width(width),
        )
    }

    /// A filled polygon through `points`.
    pub fn polygon(&mut self, points: &[(f32, f32)], c: Color) -> AureaResult<()> {
        let mut path = Path::new();
        for (i, &(x, y)) in points.iter().enumerate() {
            let p = Point::new(x, y);
            path.commands.push(if i == 0 {
                PathCommand::MoveTo(p)
            } else {
                PathCommand::LineTo(p)
            });
        }
        path.commands.push(PathCommand::Close);
        self.ctx.draw_path(&path, &fill(c))
    }

    pub fn circle(&mut self, x: f32, y: f32, r: f32, c: Color) -> AureaResult<()> {
        self.ctx.draw_circle(Point::new(x, y), r, &fill(c))
    }

    pub fn width(&mut self, text: &str, font: &Font) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let guess = text.chars().count() as f32 * font.size * 0.55;
        self.ctx.measure_text(text, font).map_or(guess, |m| m.width)
    }

    /// Text with its top left at `(x, y)`.
    pub fn text(&mut self, text: &str, x: f32, y: f32, font: &Font, c: Color) -> AureaResult<()> {
        self.ctx
            .draw_text_with_font(text, Point::new(x, y + font.size * 0.8), font, &fill(c))
    }

    /// Text centred in `area`.
    pub fn centred(&mut self, text: &str, area: Area, font: &Font, c: Color) -> AureaResult<()> {
        let w = self.width(text, font);
        let x = area.x + (area.w - w) / 2.0;
        let y = area.y + (area.h - font.size) / 2.0;
        self.text(text, x, y, font, c)
    }

    /// Text at the left of `area`, vertically centred, made smaller until
    /// it fits (down to `min`).
    pub fn fitted_left(
        &mut self,
        text: &str,
        area: Area,
        size: f32,
        min: f32,
        c: Color,
    ) -> AureaResult<()> {
        let mut font = self.font(size);
        while self.width(text, &font) > area.w && font.size > self.s(min) {
            font.size -= self.s(0.5);
        }
        let y = area.y + (area.h - font.size) / 2.0;
        self.text(text, area.x, y, &font, c)
    }

    /// Text centred in `area`, made smaller until it fits (down to
    /// `min`).
    pub fn fitted(
        &mut self,
        text: &str,
        area: Area,
        size: f32,
        min: f32,
        c: Color,
    ) -> AureaResult<()> {
        let mut font = self.font(size);
        while self.width(text, &font) > area.w && font.size > self.s(min) {
            font.size -= self.s(0.5);
        }
        self.centred(text, area, &font, c)
    }
}

pub fn fill(c: Color) -> Paint {
    Paint::new().color(c).style(PaintStyle::Fill)
}

/// A rounded rectangle; corners are cubic quarter circles.
fn rounded(a: Area, radius: f32) -> Path {
    let r = radius.min(a.w / 2.0).min(a.h / 2.0).max(0.0);
    // Control point distance for a quarter circle.
    let k = r * 0.552_284_8;
    let (x0, y0, x1, y1) = (a.x, a.y, a.right(), a.bottom());
    let p = Point::new;
    let mut path = Path::new();
    path.commands = vec![
        PathCommand::MoveTo(p(x0 + r, y0)),
        PathCommand::LineTo(p(x1 - r, y0)),
        PathCommand::CubicTo(p(x1 - r + k, y0), p(x1, y0 + r - k), p(x1, y0 + r)),
        PathCommand::LineTo(p(x1, y1 - r)),
        PathCommand::CubicTo(p(x1, y1 - r + k), p(x1 - r + k, y1), p(x1 - r, y1)),
        PathCommand::LineTo(p(x0 + r, y1)),
        PathCommand::CubicTo(p(x0 + r - k, y1), p(x0, y1 - r + k), p(x0, y1 - r)),
        PathCommand::LineTo(p(x0, y0 + r)),
        PathCommand::CubicTo(p(x0, y0 + r - k), p(x0 + r - k, y0), p(x0 + r, y0)),
        PathCommand::Close,
    ];
    path
}
