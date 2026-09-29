//! Eased values for the UI.

use std::collections::HashMap;
use std::time::Instant;

use crate::app::{Hit, Slider};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Hover(Hit),
    Selected(usize),
    Down(usize),
    TabBar,
    Page,
    Knob(Slider),
}

pub mod rate {
    pub const HOVER: f32 = 18.0;
    pub const SLIDE: f32 = 14.0;
    pub const PAGE: f32 = 11.0;
    pub const KNOB: f32 = 20.0;
    pub const PRESS: f32 = 40.0;
    pub const RELEASE: f32 = 5.0;
}

const SNAP: f32 = 0.002;
/// Longest step one frame takes, so a frame after a quiet spell does not
/// jump to the end.
const MAX_STEP: f32 = 1.0 / 30.0;

#[derive(Debug)]
struct Value {
    now: f32,
    used: bool,
}

#[derive(Debug)]
pub struct Anim {
    values: HashMap<Key, Value>,
    start: Instant,
    last: Option<Instant>,
    dt: f32,
    busy: bool,
}

impl Default for Anim {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            start: Instant::now(),
            last: None,
            dt: 0.0,
            busy: false,
        }
    }
}

impl Anim {
    pub fn frame(&mut self) {
        let now = Instant::now();
        self.dt = self
            .last
            .map_or(0.0, |l| now.duration_since(l).as_secs_f32().min(MAX_STEP));
        self.last = Some(now);
        self.values.retain(|_, v| std::mem::take(&mut v.used));
        self.busy = false;
    }

    pub fn time(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }

    pub const fn busy(&self) -> bool {
        self.busy
    }

    /// `key` moved towards `target` at `rate`. A new value starts at its
    /// target.
    pub fn to(&mut self, key: Key, target: f32, rate: f32) -> f32 {
        self.towards(key, target, rate, rate)
    }

    pub fn towards(&mut self, key: Key, target: f32, up: f32, down: f32) -> f32 {
        let dt = self.dt;
        let v = self.values.entry(key).or_insert(Value {
            now: target,
            used: true,
        });
        v.used = true;
        let rate = if target > v.now { up } else { down };
        v.now += (target - v.now) * (1.0 - (-rate * dt).exp());
        let scale = target.abs().max(1.0);
        if (target - v.now).abs() < SNAP * scale {
            v.now = target;
        } else {
            self.busy = true;
        }
        v.now
    }

    pub fn set(&mut self, key: Key, value: f32) {
        self.values.insert(
            key,
            Value {
                now: value,
                used: true,
            },
        );
        self.busy = true;
    }
}

pub fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_ease_to_their_target_and_settle() {
        let mut a = Anim::default();
        a.frame();
        assert!((a.to(Key::TabBar, 0.0, rate::SLIDE)).abs() < f32::EPSILON);
        a.dt = 0.016;
        let first = a.to(Key::TabBar, 2.0, rate::SLIDE);
        assert!(first > 0.0 && first < 2.0);
        assert!(a.busy());
        for _ in 0..200 {
            a.busy = false;
            a.to(Key::TabBar, 2.0, rate::SLIDE);
        }
        assert!((a.to(Key::TabBar, 2.0, rate::SLIDE) - 2.0).abs() < f32::EPSILON);
        assert!(!a.busy());
    }

    #[test]
    fn unused_values_are_forgotten() {
        let mut a = Anim::default();
        a.frame();
        a.set(Key::Page, 0.0);
        a.frame();
        a.frame();
        assert!((a.to(Key::Page, 1.0, rate::PAGE) - 1.0).abs() < f32::EPSILON);
    }
}
