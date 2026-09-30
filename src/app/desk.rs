//! Windows page input: the desk and the devices on it.

use std::sync::mpsc::Sender;

use lykil::lighting::{Effect, Settings};

use crate::devices::Command;
use crate::lamps::OWN_EFFECTS;

use super::{DeskGrab, Hit, Shared};

impl Shared {
    pub(crate) fn lamp(&self, command: crate::lamps::LampCommand) {
        if let Some(tx) = &self.lamp_tx {
            let _ = tx.send(command);
        }
    }

    /// A click on the Windows page's desk and device list.
    /// Changes the picked device's own effect, here and in the lamp thread.
    pub(crate) fn change_own(&mut self, change: impl FnOnce(&mut Option<Settings>)) {
        let id = self.ui.desk_selected.clone();
        let Some(l) = self.lamps.iter_mut().find(|l| Some(&l.id) == id.as_ref()) else {
            return;
        };
        let before = l.place.own;
        change(&mut l.place.own);
        if l.place.own != before {
            let command = crate::lamps::LampCommand::Own(l.id.clone(), l.place.own);
            self.lamp(command);
        }
    }

    pub(crate) fn click_desk(&mut self, hit: Hit, tx: &Sender<Command>) {
        match hit {
            Hit::DeviceOwn(on) => {
                // A device has no keys, so its own effect starts from the
                // keyboard's without the press effects.
                let start = self.lighting().unwrap_or(Settings::DEFAULT);
                let start = Settings {
                    effect: if OWN_EFFECTS.contains(&start.effect) {
                        start.effect
                    } else {
                        Effect::Cycle
                    },
                    ..start
                };
                self.change_own(|own| *own = on.then_some(own.unwrap_or(start)));
            }
            Hit::DeviceEffect(e) => self.change_own(|own| {
                if let Some(s) = own {
                    s.effect = e;
                }
            }),
            Hit::LampSync(on) => {
                self.lamp_sync = on;
                self.lamp(crate::lamps::LampCommand::Sync(on));
            }
            Hit::DeskDevice(i) => {
                if let Some(l) = self.lamps.get(i) {
                    let area = self
                        .ui
                        .hits
                        .iter()
                        .rev()
                        .find(|(_, h)| *h == hit)
                        .map(|(a, _)| *a);
                    if let Some(a) = area {
                        let scale = a.w / l.size.0.max(1e-3);
                        self.ui.desk_grab = Some(DeskGrab {
                            id: l.id.clone(),
                            from: (l.place.x, l.place.y),
                            mouse: self.ui.mouse,
                            scale,
                        });
                        self.ui.drag = Some((hit, a));
                    }
                    self.ui.desk_selected = Some(l.id.clone());
                }
            }
            Hit::DeviceLevel | Hit::DeviceHue | Hit::DeviceSpeed => {
                if let Some((area, _)) = self.ui.hits.iter().rev().find(|(_, h)| *h == hit) {
                    let area = *area;
                    self.ui.drag = Some((hit, area));
                    self.drag_to(hit, area, tx);
                }
            }
            Hit::LampFollow(i) => {
                if let Some(l) = self.lamps.get_mut(i) {
                    l.place.follow = !l.place.follow;
                    let command = crate::lamps::LampCommand::Follow(l.id.clone(), l.place.follow);
                    self.lamp(command);
                }
            }
            _ => {}
        }
    }

    /// Slides a dropped device off any it overlaps.
    pub(crate) fn drop_device(&mut self, id: &str) {
        let Some(moving) = self.lamps.iter().find(|l| l.id == id) else {
            return;
        };
        let others: Vec<_> = self
            .lamps
            .iter()
            .filter(|l| l.id != id)
            .map(|l| (l.place, l.size))
            .collect();
        let spot = crate::lamps::desk::free_spot((moving.place, moving.size), &others);
        if spot != moving.place {
            if let Some(l) = self.lamps.iter_mut().find(|l| l.id == id) {
                l.place = spot;
            }
            self.lamp(crate::lamps::LampCommand::Place(
                id.to_string(),
                spot.x,
                spot.y,
            ));
        }
    }

    /// Follows the mouse with the grabbed desk device.
    pub(crate) fn move_device(&mut self, i: usize) {
        let Some(DeskGrab {
            id,
            from: (x0, y0),
            mouse: (mx, my),
            scale,
        }) = self.ui.desk_grab.clone()
        else {
            return;
        };
        let (x, y) = (
            x0 + (self.ui.mouse.0 - mx) / scale,
            y0 + (self.ui.mouse.1 - my) / scale,
        );
        if let Some(l) = self.lamps.get_mut(i).filter(|l| l.id == id) {
            l.place.x = x;
            l.place.y = y;
        }
        self.lamp(crate::lamps::LampCommand::Place(id, x, y));
    }
}
