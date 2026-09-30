//! Recent key presses and heat, for the reactive, ripple and heatmap
//! previews.

use std::collections::VecDeque;

use lykil::lighting::RIPPLES;

/// Recent key presses, for the reactive and ripple previews.
#[derive(Debug, Default)]
pub struct Presses {
    down: Vec<bool>,
    /// When each key (by description index) last went down, in
    /// [`Anim::time`] seconds.
    pub at: Vec<Option<f32>>,
    pub recent: VecDeque<(usize, f32)>,
    /// Heatmap warmth per key, `0..=1`, and when it was last set.
    heat: Vec<(f32, f32)>,
    /// For each key, the keys a press warms and by how much, with the
    /// size it was worked out for.
    reach: (u8, Vec<Vec<(usize, f32)>>),
}

/// Heat one press adds, as the firmware's 22000 of 65535.
pub const HEAT_PER_PRESS: f32 = 22_000.0 / 65_535.0;

impl Presses {
    pub fn press(&mut self, key: usize, time: f32) {
        if self.at.len() <= key {
            self.at.resize(key + 1, None);
        }
        self.at[key] = Some(time);
        let near = self
            .reach
            .1
            .get(key)
            .cloned()
            .unwrap_or_else(|| vec![(key, 1.0)]);
        for (k, share) in near {
            if self.heat.len() <= k {
                self.heat.resize(k + 1, (0.0, time));
            }
            // Speed only sets how fast heat goes; for adding, any will do.
            let warm = self.heat_at(k, time, 128);
            self.heat[k] = ((warm + HEAT_PER_PRESS * share).min(1.0), time);
        }
        if self.recent.len() == RIPPLES {
            self.recent.pop_front();
        }
        self.recent.push_back((key, time));
    }

    /// Works out which keys a press warms, as the firmware does: keys
    /// within `size / 3 + 1` layout units, less the further they are.
    pub fn set_reach(&mut self, points: &[Option<lykil::lighting::Point>], size: u8) {
        if self.reach.0 == size && self.reach.1.len() == points.len() {
            return;
        }
        let reach = f32::from(size / 3 + 1);
        let near = |from: Option<lykil::lighting::Point>| -> Vec<(usize, f32)> {
            let Some(a) = from else {
                return Vec::new();
            };
            points
                .iter()
                .enumerate()
                .filter_map(|(k, p)| {
                    let b = (*p)?;
                    let dx = f32::from(a.x.abs_diff(b.x));
                    let dy = f32::from(a.y.abs_diff(b.y));
                    let d = (dx * dx + dy * dy).sqrt().floor();
                    (d < reach).then(|| (k, (reach - d) / reach))
                })
                .collect()
        };
        self.reach = (size, points.iter().map(|p| near(*p)).collect());
    }

    /// Key `key`'s heat at `time`, cooling as the firmware does: about 10 s
    /// from hot to cold at speed 128.
    pub fn heat_at(&self, key: usize, time: f32, speed: u8) -> f32 {
        let per_second = (f32::from(speed) + 16.0) / 22.0 * 1000.0 / 65_535.0;
        self.heat
            .get(key)
            .map_or(0.0, |(h, at)| (h - (time - at) * per_second).max(0.0))
    }

    /// Records the keys that went down since the last call; held keys
    /// stay lit, as on the keyboard.
    pub fn follow(&mut self, down: &[bool], time: f32) {
        for (key, &d) in down.iter().enumerate() {
            if !d {
                continue;
            }
            if self.down.get(key).copied().unwrap_or(false) {
                self.at[key] = Some(time);
            } else {
                self.press(key, time);
            }
        }
        self.down = down.to_vec();
    }
}
