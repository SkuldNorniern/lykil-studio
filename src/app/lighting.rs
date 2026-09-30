//! Lighting page input: effects, the colour picker, the brush and VIA
//! settings.

use std::sync::mpsc::Sender;

use aurea::{KeyCode, WindowEvent};
use lykil::lighting::{Effect, Hsv, Palette, Rgb, Settings};

use crate::colour;
use crate::devices::Command;

use super::{Field, Hit, RECENT, Shared, Tab};

impl Shared {
    pub(crate) fn pick(&mut self, hsv: Hsv, tx: &Sender<Command>) {
        if self.brushing() {
            self.ui.brush = Some(hsv);
        } else if self.editing_second() {
            // The second colour shares the brightness.
            self.change_lighting(tx, |s| {
                s.color2 = Hsv::new(hsv.h, hsv.s, 255);
                s.color.v = hsv.v;
            });
        } else {
            self.change_lighting(tx, |s| s.color = hsv);
        }
    }

    pub fn picked(&self) -> Hsv {
        let s = self.lighting().unwrap_or_default();
        if self.brushing() {
            self.ui.brush.unwrap_or(Hsv::new(s.color.h, s.color.s, 255))
        } else if self.editing_second() {
            s.second()
        } else {
            s.color
        }
    }

    pub fn brushing(&self) -> bool {
        self.lighting().is_some_and(|s| s.effect == Effect::PerKey)
    }

    pub fn editing_second(&self) -> bool {
        self.ui.second
            && !self.brushing()
            && self.lighting().is_some_and(|s| s.palette == Palette::Two)
    }

    pub(crate) fn click_lighting(&mut self, hit: Hit, tx: &Sender<Command>) {
        match hit {
            Hit::PaintAll => {
                self.remember_brush();
                let color = self.picked().to_rgb();
                self.fill(color, tx);
            }
            Hit::ClearAll => self.fill(Rgb::OFF, tx),
            Hit::Effect(effect) => self.change_lighting(tx, |s| s.effect = effect),
            Hit::OsLighting(on) => self.change_lighting(tx, |s| s.os_lighting = on),
            Hit::Colours(p) => self.change_lighting(tx, |s| s.palette = p),
            Hit::Second(second) => self.ui.second = second,
            Hit::LayerKeys(on) => self.change_lighting(tx, |s| s.layer_keys = on),
            Hit::Field(f) => {
                if self.ui.editing.as_ref().is_none_or(|(e, _)| *e != f) {
                    self.edit(f);
                }
            }
            Hit::Swatch(c) => self.pick(colour::to_hsv(c), tx),
            Hit::ViaOption(i, v) => self.set_via(i, 0, v, tx),
            Hit::ViaToggle(i, on) => self.set_via(i, 0, u8::from(on), tx),
            Hit::Slider(_)
            | Hit::Square
            | Hit::HueBar
            | Hit::ViaRange(_)
            | Hit::ViaHue(_)
            | Hit::ViaSat(_) => {
                if let Some((area, _)) = self.ui.hits.iter().rev().find(|(_, h)| *h == hit) {
                    let area = *area;
                    self.ui.drag = Some((hit, area));
                    self.drag_to(hit, area, tx);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn right_click(&mut self) {
        let Some(Hit::Key(k)) = self.ui.hovered() else {
            return;
        };
        if self.ui.tab != Tab::Lighting || !self.brushing() {
            return;
        }
        let own = self
            .keyboard
            .description
            .as_ref()
            .and_then(|d| d.keys.get(k)?.led)
            .and_then(|led| self.keyboard.key_colors.get(usize::from(led)).copied());
        if let Some(c) = own {
            self.ui.brush = Some(colour::to_hsv(c));
        }
    }

    pub(crate) fn copy_paste(&mut self, key: KeyCode, tx: &Sender<Command>) -> bool {
        match key {
            KeyCode::C => {
                let _ = aurea::set_clipboard_text(&colour::hex(self.picked().to_rgb()));
                false
            }
            KeyCode::V => {
                if let Some(c) = aurea::clipboard_text().as_deref().and_then(colour::parse) {
                    self.pick(colour::to_hsv(c), tx);
                }
                true
            }
            _ => false,
        }
    }

    pub(crate) fn field_event(
        &mut self,
        event: &WindowEvent,
        tx: &Sender<Command>,
    ) -> Option<bool> {
        let (field, text) = self.ui.editing.as_mut()?;
        let field = *field;
        match event {
            WindowEvent::TextInput { text: typed } => {
                if std::mem::take(&mut self.ui.fresh) {
                    text.clear();
                }
                for ch in typed.chars().filter(|c| field.takes(*c)) {
                    if text.len() < field.max_len() {
                        text.push(ch);
                    }
                }
                Some(true)
            }
            WindowEvent::KeyInput {
                key,
                pressed: true,
                modifiers,
            } => match key {
                KeyCode::Backspace => {
                    if std::mem::take(&mut self.ui.fresh) {
                        text.clear();
                    } else {
                        text.pop();
                    }
                    Some(true)
                }
                KeyCode::V if modifiers.ctrl => {
                    if let Some(pasted) = aurea::clipboard_text() {
                        *text = pasted.trim().chars().take(field.max_len()).collect();
                        self.ui.fresh = false;
                    }
                    Some(true)
                }
                KeyCode::Enter => {
                    self.commit(tx);
                    Some(true)
                }
                KeyCode::Tab => {
                    self.commit(tx);
                    self.edit(field.next());
                    Some(true)
                }
                KeyCode::Escape => {
                    self.ui.editing = None;
                    Some(true)
                }
                _ => Some(false),
            },
            _ => None,
        }
    }

    pub(crate) fn edit(&mut self, field: Field) {
        let text = field.text(self.picked().to_rgb());
        self.ui.editing = Some((field, text));
        self.ui.fresh = true;
    }

    pub(crate) fn commit(&mut self, tx: &Sender<Command>) {
        let Some((field, text)) = self.ui.editing.take() else {
            return;
        };
        let current = self.picked().to_rgb();
        let value = || text.parse::<u8>().ok();
        let rgb = match field {
            Field::Hex => colour::parse(&text),
            Field::Red => value().map(|r| Rgb { r, ..current }),
            Field::Green => value().map(|g| Rgb { g, ..current }),
            Field::Blue => value().map(|b| Rgb { b, ..current }),
        };
        match rgb {
            Some(c) if c != current => self.pick(colour::to_hsv(c), tx),
            Some(_) => {}
            None => {
                self.ui.notice = Some(self.ui.lang.fill("not a colour: {}", &[text.as_str()]));
            }
        }
    }

    /// Sets byte `at` of VIA setting `i` here and on the keyboard.
    pub(crate) fn set_via(&mut self, i: usize, at: usize, byte: u8, tx: &Sender<Command>) {
        let Some(s) = self.keyboard.via_settings.get_mut(i) else {
            return;
        };
        if s.value.len() <= at {
            s.value.resize(at + 1, 0);
        }
        if s.value[at] == byte {
            return;
        }
        s.value[at] = byte;
        let _ = tx.send(Command::SetVia {
            setting: i,
            value: s.value.clone(),
        });
    }

    pub(crate) fn remember_brush(&mut self) {
        let c = self.picked().to_rgb();
        self.ui.recent.retain(|r| *r != c);
        self.ui.recent.insert(0, c);
        self.ui.recent.truncate(RECENT);
    }

    pub(crate) fn fill(&mut self, color: Rgb, tx: &Sender<Command>) {
        let count = self.keyboard.key_colors.len();
        self.keyboard.key_colors = vec![color; count];
        let _ = tx.send(Command::SetKeyColors {
            start: 0,
            colors: vec![color; count],
        });
    }

    pub(crate) fn paint(&mut self, k: usize, tx: &Sender<Command>) {
        let color = self.picked().to_rgb();
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

    pub(crate) fn change_lighting(
        &mut self,
        tx: &Sender<Command>,
        change: impl FnOnce(&mut Settings),
    ) {
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

    pub fn settle(&mut self) {
        let confirmed = self.ui.draft == self.keyboard.lighting.map(|l| l.settings);
        // A refused change shows what the keyboard kept, not the draft.
        let refused = self.keyboard.error.is_some();
        if self.ui.drag.is_none() && self.ui.draft.is_some() && (confirmed || refused) {
            self.ui.draft = None;
        }
    }
}
