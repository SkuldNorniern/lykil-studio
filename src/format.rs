//! Numbers and times as the pages write them.

/// `t` (`0..=1`) as a percentage.
pub fn percent(t: f32) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let p = (t.clamp(0.0, 1.0) * 100.0).round() as u32;
    format!("{p}%")
}

pub fn uptime(ms: u32) -> String {
    let s = ms / 1000;
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, s / 60 % 60)
    } else {
        format!("{}m {:02}s", s / 60, s % 60)
    }
}

/// `n` with a comma every three digits.
pub fn group(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_times() {
        assert_eq!(group(1_234_567), "1,234,567");
        assert_eq!(group(12), "12");
        assert_eq!(uptime(61_000), "1m 01s");
        assert_eq!(uptime(3_725_000), "1h 02m");
        assert_eq!(percent(0.5), "50%");
    }
}
