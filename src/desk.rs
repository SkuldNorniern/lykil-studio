//! Where each lit device sits on the desk, and how bright it is.
//! Effects run in desk space, so a wave goes from one device to the next.

use std::collections::BTreeMap;
use std::path::PathBuf;

use lykil::lighting::Point;

/// A device on the desk: its top left in metres, and its brightness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub x: f32,
    pub y: f32,
    pub level: u8,
    pub follow: bool,
}

impl Default for Place {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            level: 255,
            follow: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Desk {
    places: BTreeMap<String, Place>,
}

fn file() -> PathBuf {
    let base = std::env::var_os("APPDATA").map_or_else(|| PathBuf::from("."), PathBuf::from);
    base.join("Lykil Studio").join("desk.txt")
}

impl Desk {
    /// The saved desk; empty if there is none.
    pub fn load() -> Self {
        std::fs::read_to_string(file())
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = file();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, self.text());
    }

    /// One device per line: `x y level follow id`, tab separated.
    fn text(&self) -> String {
        use std::fmt::Write as _;
        self.places.iter().fold(String::new(), |mut out, (id, p)| {
            let _ = writeln!(
                out,
                "{:.3}\t{:.3}\t{}\t{}\t{id}",
                p.x,
                p.y,
                p.level,
                u8::from(p.follow)
            );
            out
        })
    }

    fn parse(text: &str) -> Self {
        let places = text
            .lines()
            .filter_map(|line| {
                let mut f = line.splitn(5, '\t');
                let place = Place {
                    x: f.next()?.parse().ok()?,
                    y: f.next()?.parse().ok()?,
                    level: f.next()?.parse().ok()?,
                    follow: f.next()? == "1",
                };
                Some((f.next()?.to_string(), place))
            })
            .collect();
        Self { places }
    }

    pub fn get(&self, id: &str) -> Option<Place> {
        self.places.get(id).copied()
    }

    pub fn set(&mut self, id: &str, place: Place) {
        self.places.insert(id.to_string(), place);
    }

    /// Puts a device first seen right of everything placed so far.
    pub fn place_new(&mut self, id: &str, placed: &[(Place, f32)]) -> Place {
        if let Some(p) = self.get(id) {
            return p;
        }
        let right = placed.iter().map(|(p, w)| p.x + w).fold(0.0f32, f32::max);
        let p = Place {
            x: if placed.is_empty() { 0.0 } else { right + 0.05 },
            ..Place::default()
        };
        self.set(id, p);
        p
    }
}

/// Gap kept between devices, in metres.
pub const GAP: f32 = 0.01;

/// Do two devices (top left, size) overlap, gap included?
pub fn overlaps(a: (Place, (f32, f32)), b: (Place, (f32, f32))) -> bool {
    let ((pa, (wa, ha)), (pb, (wb, hb))) = (a, b);
    pa.x < pb.x + wb + GAP
        && pb.x < pa.x + wa + GAP
        && pa.y < pb.y + hb + GAP
        && pb.y < pa.y + ha + GAP
}

/// `moving` shifted the least it takes to overlap none of `others`.
pub fn free_spot(moving: (Place, (f32, f32)), others: &[(Place, (f32, f32))]) -> Place {
    let (start, size) = moving;
    // Places touching some device's side, and the start itself.
    let mut spots = vec![start];
    for &(p, (w, h)) in others {
        spots.push(Place {
            x: p.x + w + GAP,
            ..start
        });
        spots.push(Place {
            x: p.x - size.0 - GAP,
            ..start
        });
        spots.push(Place {
            y: p.y + h + GAP,
            ..start
        });
        spots.push(Place {
            y: p.y - size.1 - GAP,
            ..start
        });
    }
    let free = |p: &Place| others.iter().all(|o| !overlaps((*p, size), *o));
    let far = |p: &Place| (p.x - start.x).hypot(p.y - start.y);
    spots
        .into_iter()
        .filter(free)
        .min_by(|a, b| far(a).total_cmp(&far(b)))
        .unwrap_or(start)
}

/// Maps desk metres onto effect coordinates: the devices' span across
/// becomes `0..=255`, `y` on the same scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub left: f32,
    pub top: f32,
    pub width: f32,
}

impl Frame {
    /// The frame around devices at `places` with sizes `(w, h)`.
    pub fn around(devices: impl Iterator<Item = (Place, (f32, f32))>) -> Self {
        let (mut l, mut t, mut r) = (f32::MAX, f32::MAX, f32::MIN);
        for (p, (w, _)) in devices {
            l = l.min(p.x);
            t = t.min(p.y);
            r = r.max(p.x + w);
        }
        if l > r {
            return Self {
                left: 0.0,
                top: 0.0,
                width: 1.0,
            };
        }
        Self {
            left: l,
            top: t,
            width: (r - l).max(1e-3),
        }
    }

    /// Where a lamp at `(x, y)` on a device at `place` lands.
    pub fn point(&self, place: Place, (x, y): (f32, f32)) -> Point {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let to = |v: f32| (v / self.width * 255.0).clamp(0.0, 255.0) as u8;
        Point::new(to(place.x + x - self.left), to(place.y + y - self.top))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desks_read_back() {
        let mut d = Desk::default();
        d.set(
            "a\tb",
            Place {
                x: 0.5,
                y: -0.25,
                level: 128,
                follow: true,
            },
        );
        assert_eq!(Desk::parse(&d.text()).get("a\tb"), d.get("a\tb"));
    }

    #[test]
    fn new_devices_go_right() {
        let mut d = Desk::default();
        let first = d.place_new("a", &[]);
        let second = d.place_new("b", &[(first, 0.4)]);
        assert!((second.x - 0.45).abs() < 1e-6);
    }

    #[test]
    fn dropped_devices_slide_off_others() {
        let at = |x: f32, y: f32| Place {
            x,
            y,
            ..Place::default()
        };
        let keyboard = (at(0.0, 0.0), (0.4, 0.15));
        let mouse = (at(0.35, 0.02), (0.07, 0.12));
        assert!(overlaps(keyboard, mouse));
        let p = free_spot(mouse, &[keyboard]);
        assert!(!overlaps((p, mouse.1), keyboard));
        // Nearest: just right of the keyboard, same height.
        assert!((p.x - 0.41).abs() < 1e-5 && (p.y - 0.02).abs() < 1e-6);
        let clear = (at(1.0, 0.0), (0.1, 0.1));
        assert_eq!(free_spot(clear, &[keyboard]), clear.0);
    }

    #[test]
    fn the_frame_spans_every_device() {
        let a = Place::default();
        let b = Place {
            x: 0.5,
            ..Place::default()
        };
        let f = Frame::around([(a, (0.4, 0.1)), (b, (0.5, 0.1))].into_iter());
        assert_eq!(f.point(a, (0.0, 0.0)).x, 0);
        assert_eq!(f.point(b, (0.5, 0.0)).x, 255);
    }
}
