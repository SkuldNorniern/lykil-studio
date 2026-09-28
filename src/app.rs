//! UI state and input.
//!
//! The view records what it drew where ([`Ui::hits`]); input is matched
//! against the last frame's hits, so the view is the only place that knows
//! the layout. Changes for the keyboard go to the device thread as
//! [`Command`]s and show at once here.

use std::sync::mpsc::Sender;

use aurea::{KeyCode, MouseButton, WindowEvent};
use lykil::binding::Binding;
use lykil::keycode::Modifiers;
use lykil::lighting::{Effect, Settings};

use crate::device::{Command, Keyboard};
use crate::draw::Area;
use crate::edit::{self, Hold};

/// Everything the draw callback and the input handler share.
#[derive(Debug, Default)]
pub struct Shared {
    pub keyboard: Keyboard,
    pub ui: Ui,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Keymap,
    Lighting,
    Device,
}

impl Tab {
    pub const ALL: [Self; 3] = [Self::Keymap, Self::Lighting, Self::Device];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Keymap => "Keymap",
            Self::Lighting => "Lighting",
            Self::Device => "Device",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slider {
    Hue,
    Saturation,
    Brightness,
    Speed,
}

/// Something that reacts to the mouse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Tab(Tab),
    Layer(u8),
    Key(usize),
    /// A palette group.
    Group(usize),
    Palette(Binding),
    /// What the selected key does when held.
    Hold(Hold),
    /// A modifier sent with the selected key, switched on or off.
    With(Modifiers),
    ResetKeymap,
    Effect(Effect),
    Slider(Slider),
    OsLighting(bool),
}

#[derive(Debug, Default)]
pub struct Ui {
    pub tab: Tab,
    pub layer: u8,
    /// Selected key, by description index (the key position).
    pub selected: Option<usize>,
    /// The palette group shown.
    pub group: usize,
    pub mouse: (f32, f32),
    /// What the last frame drew that reacts to the mouse, back to front.
    pub hits: Vec<(Area, Hit)>,
    /// The slider being dragged and its track.
    drag: Option<(Slider, Area)>,
    /// Lighting settings sent but not confirmed yet, so controls follow the
    /// mouse at once.
    pub draft: Option<Settings>,
    /// Reset keymap was clicked once; the next click does it.
    pub confirm_reset: bool,
    /// Seconds of lighting preview animation.
    pub time: f32,
    /// Canvas pixels per design pixel.
    pub scale: f32,
}

impl Ui {
    /// The hit under the mouse, front first.
    pub fn hovered(&self) -> Option<Hit> {
        let (x, y) = self.mouse;
        self.hits
            .iter()
            .rev()
            .find(|(a, _)| a.contains(x, y))
            .map(|(_, h)| *h)
    }

    pub fn is_hovered(&self, area: Area) -> bool {
        area.contains(self.mouse.0, self.mouse.1)
    }
}

impl Shared {
    /// Lighting settings as the UI shows them.
    pub fn lighting(&self) -> Option<Settings> {
        self.ui
            .draft
            .or_else(|| self.keyboard.lighting.map(|l| l.settings))
    }

    /// Should the preview animate?
    pub fn animating(&self) -> bool {
        self.ui.tab == Tab::Lighting
            && self.lighting().is_some_and(|s| {
                matches!(
                    s.effect,
                    Effect::Breathing | Effect::Cycle | Effect::Wave | Effect::Reactive
                )
            })
    }

    /// Handles one window event; returns whether to redraw.
    pub fn event(&mut self, event: &WindowEvent, tx: &Sender<Command>) -> bool {
        match *event {
            WindowEvent::MouseMove { x, y } => {
                #[allow(clippy::cast_possible_truncation)]
                let now = (x as f32, y as f32);
                let before = self.ui.hovered();
                self.ui.mouse = now;
                if let Some((slider, track)) = self.ui.drag {
                    self.slide(slider, track, tx);
                    return true;
                }
                before != self.ui.hovered()
            }
            WindowEvent::MouseButton {
                button: MouseButton::Left,
                pressed,
                x,
                y,
                ..
            } => {
                #[allow(clippy::cast_possible_truncation)]
                {
                    self.ui.mouse = (x as f32, y as f32);
                }
                if pressed {
                    self.click(tx);
                } else {
                    self.ui.drag = None;
                }
                true
            }
            WindowEvent::KeyInput {
                key: KeyCode::Escape,
                pressed: true,
                ..
            } => {
                self.ui.selected = None;
                self.ui.confirm_reset = false;
                true
            }
            WindowEvent::MouseExited => {
                self.ui.mouse = (-1.0, -1.0);
                true
            }
            _ => false,
        }
    }

    fn click(&mut self, tx: &Sender<Command>) {
        let hit = self.ui.hovered();
        if hit != Some(Hit::ResetKeymap) {
            self.ui.confirm_reset = false;
        }
        let Some(hit) = hit else {
            return;
        };
        match hit {
            Hit::Tab(t) => self.ui.tab = t,
            Hit::Layer(l) => self.ui.layer = l,
            Hit::Key(k) => {
                self.ui.selected = if self.ui.selected == Some(k) {
                    None
                } else {
                    Some(k)
                };
            }
            Hit::Group(g) => self.ui.group = g,
            Hit::Palette(binding) => self.assign(binding, tx, true),
            Hit::Hold(hold) => {
                if let Some(b) = self
                    .selected_binding()
                    .and_then(|b| edit::set_hold(b, hold))
                {
                    self.assign(b, tx, false);
                }
            }
            Hit::With(m) => {
                if let Some(b) = self
                    .selected_binding()
                    .and_then(|b| edit::toggle_with(b, m))
                {
                    self.assign(b, tx, false);
                }
            }
            Hit::ResetKeymap => {
                if self.ui.confirm_reset {
                    self.ui.confirm_reset = false;
                    let _ = tx.send(Command::ResetKeymap);
                } else {
                    self.ui.confirm_reset = true;
                }
            }
            Hit::Effect(effect) => self.change_lighting(tx, |s| s.effect = effect),
            Hit::OsLighting(on) => self.change_lighting(tx, |s| s.os_lighting = on),
            Hit::Slider(slider) => {
                if let Some((area, _)) = self
                    .ui
                    .hits
                    .iter()
                    .rev()
                    .find(|(_, h)| *h == Hit::Slider(slider))
                {
                    let track = *area;
                    self.ui.drag = Some((slider, track));
                    self.slide(slider, track, tx);
                }
            }
        }
    }

    /// The binding of the selected key on the shown layer.
    pub fn selected_binding(&self) -> Option<Binding> {
        self.keyboard
            .keymap
            .get(usize::from(self.ui.layer))?
            .get(self.ui.selected?)
            .copied()
    }

    /// Puts `binding` on the selected key; `advance` moves on to the next
    /// key.
    fn assign(&mut self, binding: Binding, tx: &Sender<Command>, advance: bool) {
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

    fn slide(&mut self, slider: Slider, track: Area, tx: &Sender<Command>) {
        let t = ((self.ui.mouse.0 - track.x) / track.w).clamp(0.0, 1.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (t * 255.0).round() as u8;
        self.change_lighting(tx, |s| match slider {
            Slider::Hue => s.color.h = v,
            Slider::Saturation => s.color.s = v,
            Slider::Brightness => s.color.v = v,
            Slider::Speed => s.speed = v,
        });
    }

    fn change_lighting(&mut self, tx: &Sender<Command>, change: impl FnOnce(&mut Settings)) {
        let Some(mut s) = self.lighting() else {
            return;
        };
        change(&mut s);
        if Some(s) == self.lighting() {
            return;
        }
        self.ui.draft = Some(s);
        let _ = tx.send(Command::SetLighting(s));
    }

    /// Drops the draft once the keyboard reports it back.
    pub fn settle(&mut self) {
        if self.ui.drag.is_none()
            && self.ui.draft.is_some()
            && self.ui.draft == self.keyboard.lighting.map(|l| l.settings)
        {
            self.ui.draft = None;
        }
    }
}
