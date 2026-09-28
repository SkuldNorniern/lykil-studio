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
use lykil::lighting::{Effect, Hsv, Settings};

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
    Macros,
    Lighting,
    Device,
}

impl Tab {
    pub const ALL: [Self; 4] = [Self::Keymap, Self::Macros, Self::Lighting, Self::Device];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Keymap => "Keymap",
            Self::Macros => "Macros",
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
    /// Switches between English and Korean.
    Lang,
    /// Every key the brush colour.
    PaintAll,
    /// A macro slot.
    Macro(usize),
    SaveMacro,
    ClearMacro,
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
    /// The macro shown on the macros page.
    pub macro_id: usize,
    /// Text being typed for that macro, until saved.
    pub macro_text: Option<String>,
    pub mouse: (f32, f32),
    /// What the last frame drew that reacts to the mouse, back to front.
    pub hits: Vec<(Area, Hit)>,
    /// The slider being dragged and its track.
    drag: Option<(Slider, Area)>,
    /// The mouse is down on a key in per-key painting.
    painting: bool,
    /// Lighting settings sent but not confirmed yet, so controls follow the
    /// mouse at once.
    pub draft: Option<Settings>,
    /// Reset keymap was clicked once; the next click does it.
    pub confirm_reset: bool,
    /// Seconds of lighting preview animation.
    pub time: f32,
    /// Canvas pixels per design pixel.
    pub scale: f32,
    pub lang: crate::lang::Lang,
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
                if self.ui.painting
                    && let Some(Hit::Key(k)) = self.ui.hovered()
                {
                    self.paint(k, tx);
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
                    self.ui.painting = false;
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
                self.ui.macro_text = None;
                true
            }
            WindowEvent::KeyInput {
                key: KeyCode::Backspace,
                pressed: true,
                ..
            } if self.ui.tab == Tab::Macros => {
                self.macro_text().pop();
                true
            }
            WindowEvent::KeyInput {
                key: KeyCode::Enter,
                pressed: true,
                ..
            } if self.ui.tab == Tab::Macros => {
                self.macro_text().push('\n');
                true
            }
            WindowEvent::TextInput { ref text } if self.ui.tab == Tab::Macros => {
                let typable: String = text
                    .chars()
                    .filter(|c| {
                        !c.is_control() && lykil_config::text::steps(&c.to_string()).is_ok()
                    })
                    .collect();
                self.macro_text().push_str(&typable);
                !typable.is_empty()
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
            Hit::Lang => self.ui.lang = self.ui.lang.other(),
            Hit::Layer(l) => self.ui.layer = l,
            Hit::Key(k) if self.ui.tab == Tab::Lighting => {
                self.ui.painting = true;
                self.paint(k, tx);
            }
            Hit::Macro(id) => {
                self.ui.macro_id = id;
                self.ui.macro_text = None;
            }
            Hit::SaveMacro => {
                if let Some(text) = self.ui.macro_text.take()
                    && let Ok(steps) = lykil_config::text::steps(&text)
                {
                    self.send_macro(steps, tx);
                }
            }
            Hit::ClearMacro => {
                self.ui.macro_text = None;
                self.send_macro(Vec::new(), tx);
            }
            Hit::PaintAll => {
                let color = self.brush();
                let count = self.keyboard.key_colors.len();
                self.keyboard.key_colors = vec![color; count];
                let _ = tx.send(Command::SetKeyColors {
                    start: 0,
                    colors: vec![color; count],
                });
            }
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

    /// The text being edited for the shown macro, starting from what the
    /// macro types now.
    pub fn macro_text(&mut self) -> &mut String {
        let id = self.ui.macro_id;
        let current = self
            .keyboard
            .macros
            .get(id)
            .and_then(|s| lykil_config::text::text(s))
            .unwrap_or_default();
        self.ui.macro_text.get_or_insert(current)
    }

    fn send_macro(&mut self, steps: Vec<lykil::macros::Step>, tx: &Sender<Command>) {
        let id = self.ui.macro_id;
        if steps.len() > lykil::macros::MACRO_STEPS {
            self.keyboard.error = Some(self.ui.lang.fill(
                "too long: {} steps, a macro holds {}",
                &[
                    &steps.len().to_string(),
                    &lykil::macros::MACRO_STEPS.to_string(),
                ],
            ));
            return;
        }
        if let Some(slot) = self.keyboard.macros.get_mut(id) {
            slot.clone_from(&steps);
        }
        let _ = tx.send(Command::SetMacro {
            id: u8::try_from(id).unwrap_or(0),
            steps,
        });
    }

    /// The per-key brush: the lighting hue and saturation at full value.
    pub fn brush(&self) -> lykil::lighting::Rgb {
        let c = self.lighting().map_or(Settings::DEFAULT.color, |s| s.color);
        Hsv::new(c.h, c.s, 255).to_rgb()
    }

    /// Paints key `k` (by description index) with the brush.
    fn paint(&mut self, k: usize, tx: &Sender<Command>) {
        let color = self.brush();
        let Some(led) = self
            .keyboard
            .description
            .as_ref()
            .and_then(|d| d.keys.get(k)?.led)
        else {
            return;
        };
        let Some(slot) = self.keyboard.key_colors.get_mut(usize::from(led)) else {
            return;
        };
        if *slot == color {
            return;
        }
        *slot = color;
        let _ = tx.send(Command::SetKeyColors {
            start: led,
            colors: vec![color],
        });
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
