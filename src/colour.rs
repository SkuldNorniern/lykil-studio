//! Colour codes: `#rrggbb`, `#rgb` and `r, g, b`, and the way back to
//! [`Hsv`].

use lykil::lighting::{Hsv, Rgb};

/// `#rrggbb`.
pub fn hex(c: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

/// A colour typed or pasted: `#ff8800`, `ff8800`, `#f80`, `0xff8800`,
/// `255, 136, 0`, `255 136 0` or `rgb(255, 136, 0)`.
pub fn parse(text: &str) -> Option<Rgb> {
    let t = text.trim();
    let t = t
        .strip_prefix("rgb(")
        .and_then(|t| t.strip_suffix(')'))
        .unwrap_or(t);
    let parts: Vec<&str> = t
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() == 3 {
        let n = |p: &str| p.parse::<u8>().ok();
        return Some(Rgb::new(n(parts[0])?, n(parts[1])?, n(parts[2])?));
    }
    let h = t
        .strip_prefix('#')
        .or_else(|| t.strip_prefix("0x"))
        .unwrap_or(t);
    if !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |s: &str| u8::from_str_radix(s, 16).ok();
    match h.len() {
        6 => Some(Rgb::new(byte(&h[0..2])?, byte(&h[2..4])?, byte(&h[4..6])?)),
        3 => {
            let d = |i: usize| byte(&h[i..=i]).map(|v| v * 17);
            Some(Rgb::new(d(0)?, d(1)?, d(2)?))
        }
        _ => None,
    }
}

/// The [`Hsv`] that [`Hsv::to_rgb`] turns back into about `c`: hue in
/// sixths of 43, value the largest channel.
#[allow(clippy::many_single_char_names)]
pub fn to_hsv(c: Rgb) -> Hsv {
    let (r, g, b) = (i32::from(c.r), i32::from(c.g), i32::from(c.b));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    if max == 0 || delta == 0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        return Hsv::new(0, 0, max as u8);
    }
    let s = (delta * 255 + max / 2) / max;
    let h = if max == r {
        43 * (g - b) / delta
    } else if max == g {
        85 + 43 * (b - r) / delta
    } else {
        171 + 43 * (r - g) / delta
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Hsv::new(h.rem_euclid(256) as u8, s as u8, max as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_parse() {
        let orange = Some(Rgb::new(255, 136, 0));
        assert_eq!(parse("#FF8800"), orange);
        assert_eq!(parse(" ff8800 "), orange);
        assert_eq!(parse("0xff8800"), orange);
        assert_eq!(parse("#f80"), orange);
        assert_eq!(parse("255, 136, 0"), orange);
        assert_eq!(parse("255 136 0"), orange);
        assert_eq!(parse("rgb(255,136,0)"), orange);
        assert_eq!(parse("256, 0, 0"), None);
        assert_eq!(parse("#ff88"), None);
        assert_eq!(parse("#gg8800"), None);
        assert_eq!(hex(Rgb::new(255, 136, 0)), "#FF8800");
    }

    #[test]
    fn hsv_comes_back_close() {
        for c in [
            Rgb::new(255, 0, 0),
            Rgb::new(0, 255, 0),
            Rgb::new(0, 0, 255),
            Rgb::new(255, 136, 0),
            Rgb::new(40, 200, 180),
            Rgb::new(120, 60, 250),
            Rgb::new(250, 20, 120),
            Rgb::new(90, 90, 90),
            Rgb::new(0, 0, 0),
        ] {
            let back = to_hsv(c).to_rgb();
            let off = |a: u8, b: u8| a.abs_diff(b);
            assert!(
                off(c.r, back.r) <= 8 && off(c.g, back.g) <= 8 && off(c.b, back.b) <= 8,
                "{c:?} came back as {back:?}"
            );
        }
    }
}
