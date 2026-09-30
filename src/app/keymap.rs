//! Keymap page input: picking keys and giving them bindings.

use std::sync::mpsc::Sender;

use aurea::KeyCode;
use lykil::binding::Binding;

use crate::devices::Command;

use super::Shared;

impl Shared {
    pub(crate) fn keymap_key(&mut self, key: KeyCode, tx: &Sender<Command>) -> bool {
        match key {
            KeyCode::Left => self.step_selection(-1.0, 0.0),
            KeyCode::Right => self.step_selection(1.0, 0.0),
            KeyCode::Up => self.step_selection(0.0, -1.0),
            KeyCode::Down => self.step_selection(0.0, 1.0),
            KeyCode::Delete => {
                self.assign(Binding::None, tx, false);
                true
            }
            _ => false,
        }
    }

    pub(crate) fn step_selection(&mut self, dx: f64, dy: f64) -> bool {
        let (Some(desc), Some(current)) = (&self.keyboard.description, self.ui.selected) else {
            return false;
        };
        let centre = |i: usize| {
            desc.keys
                .get(i)?
                .geometry
                .map(|[x, y, w, h]| (x + w / 2.0, y + h / 2.0))
        };
        let Some((cx, cy)) = centre(current) else {
            return false;
        };
        // Along the direction counts once, across it counts three times,
        // so a step stays in its row or column when it can.
        let next = (0..desc.keys.len())
            .filter(|&i| i != current)
            .filter_map(|i| {
                let (x, y) = centre(i)?;
                let along = (x - cx) * dx + (y - cy) * dy;
                let across = ((x - cx) * dy).abs() + ((y - cy) * dx).abs();
                (along > 0.1).then_some((i, along + 3.0 * across))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        match next {
            Some(i) => {
                self.ui.selected = Some(i);
                true
            }
            None => false,
        }
    }

    pub fn selected_binding(&self) -> Option<Binding> {
        self.keyboard
            .keymap
            .get(usize::from(self.ui.layer))?
            .get(self.ui.selected?)
            .copied()
    }

    /// Puts `binding` on the selected key; `advance` moves on to the next
    /// key.
    pub(crate) fn assign(&mut self, binding: Binding, tx: &Sender<Command>, advance: bool) {
        let Some(key) = self.ui.selected else {
            return;
        };
        let layer = self.ui.layer;
        let (Ok(position), Some(slot)) = (
            u16::try_from(key),
            self.keyboard
                .keymap
                .get_mut(usize::from(layer))
                .and_then(|l| l.get_mut(key)),
        ) else {
            return;
        };
        *slot = binding;
        let _ = tx.send(Command::SetBinding {
            layer,
            key: position,
            binding,
        });
        if advance {
            let keys = self
                .keyboard
                .description
                .as_ref()
                .map_or(0, |d| d.keys.len());
            self.ui.selected = (key + 1 < keys).then_some(key + 1);
        }
    }
}
