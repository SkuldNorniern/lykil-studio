//! UI state and input.
//!
//! The view records what it drew where ([`Ui::hits`]); input is matched
//! against the last frame's hits, so the view is the only place that knows
//! the layout. Changes for the keyboard go to the device thread as
//! [`Command`]s and show at once here.

use std::collections::VecDeque;
use std::sync::mpsc::Sender;

use aurea::{KeyCode, MouseButton, WindowEvent};
use lykil::binding::Binding;
use lykil::keycode::Modifiers;
use lykil::lighting::{Effect, Hsv, Palette, RIPPLES, Rgb, Settings};

use crate::anim::{Anim, Key};
use crate::colour;
use crate::device::{Command, Keyboard};
use crate::draw::Area;
use crate::edit::{self, Hold};

/// Everything the draw callback and the input handler share.
#[derive(Debug, Default)]
pub struct Shared {
    pub keyboard: Keyboard,
    pub ui: Ui,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slider {
    Brightness,
    Speed,
    Background,
    Size,
}

/// A text field of the colour picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Hex,
    Red,
    Green,
    Blue,
}

impl Field {
    const ORDER: [Self; 4] = [Self::Hex, Self::Red, Self::Green, Self::Blue];

    fn next(self) -> Self {
        let at = Self::ORDER.iter().position(|f| *f == self).unwrap_or(0);
        Self::ORDER[(at + 1) % Self::ORDER.len()]
    }

    /// The field's text for colour `c`.
    pub fn text(self, c: Rgb) -> String {
        match self {
            Self::Hex => colour::hex(c),
            Self::Red => c.r.to_string(),
            Self::Green => c.g.to_string(),
            Self::Blue => c.b.to_string(),
        }
    }

    fn takes(self, ch: char) -> bool {
        match self {
            Self::Hex => ch.is_ascii_hexdigit() || "#,() xrgb".contains(ch),
            _ => ch.is_ascii_digit(),
        }
    }

    const fn max_len(self) -> usize {
        match self {
            Self::Hex => 20,
            _ => 3,
        }
    }
}

/// Something that reacts to the mouse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// Every key dark.
    ClearAll,
    /// A macro slot.
    Macro(usize),
    SaveMacro,
    ClearMacro,
    Effect(Effect),
    Slider(Slider),
    OsLighting(bool),
    /// Saturation across, brightness up.
    Square,
    HueBar,
    Field(Field),
    /// A ready-made or recent colour.
    Swatch(Rgb),
    Colours(Palette),
    /// Which colour the picker edits: the second when true.
    Second(bool),
    LayerKeys(bool),
}

/// Recent key presses, for the reactive and ripple previews.
#[derive(Debug, Default)]
pub struct Presses {
    down: Vec<bool>,
    /// When each key (by description index) last went down, in
    /// [`Anim::time`] seconds.
    pub at: Vec<Option<f32>>,
    /// The latest presses, oldest first.
    pub recent: VecDeque<(usize, f32)>,
    /// Heatmap warmth per key, `0..=1`, and when it was last set.
    heat: Vec<(f32, f32)>,
}

/// Heat one press adds, as the firmware's 12000 of 65535.
const HEAT_PER_PRESS: f32 = 12_000.0 / 65_535.0;

impl Presses {
    pub fn press(&mut self, key: usize, time: f32) {
        if self.at.len() <= key {
            self.at.resize(key + 1, None);
        }
        self.at[key] = Some(time);
        if self.heat.len() <= key {
            self.heat.resize(key + 1, (0.0, time));
        }
        // Speed only sets how fast heat goes; for adding, any will do.
        let warm = self.heat_at(key, time, 128);
        self.heat[key] = ((warm + HEAT_PER_PRESS).min(1.0), time);
        if self.recent.len() == RIPPLES {
            self.recent.pop_front();
        }
        self.recent.push_back((key, time));
    }

    /// Key `key`'s heat at `time`, cooling as the firmware does: about 4 s
    /// from hot to cold at speed 128.
    pub fn heat_at(&self, key: usize, time: f32, speed: u8) -> f32 {
        let per_second = (f32::from(speed) + 16.0) / 9.0 * 1000.0 / 65_535.0;
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

/// Colours offered next to the picker.
pub const SWATCHES: [Rgb; 8] = [
    Rgb::new(255, 0, 0),
    Rgb::new(255, 110, 0),
    Rgb::new(255, 220, 0),
    Rgb::new(0, 255, 60),
    Rgb::new(0, 220, 255),
    Rgb::new(0, 60, 255),
    Rgb::new(170, 0, 255),
    Rgb::new(255, 255, 255),
];

/// Recent brush colours kept.
const RECENT: usize = 8;

/// Flags of the UI's own state (dragging, painting and so on).
#[allow(clippy::struct_excessive_bools)]
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
    /// The control being dragged and its area.
    drag: Option<(Hit, Area)>,
    /// The mouse is down on a key in per-key painting.
    painting: bool,
    /// Lighting settings sent but not confirmed yet, so controls follow the
    /// mouse at once.
    pub draft: Option<Settings>,
    /// Per-key brush; until picked, the lighting colour at full value.
    pub brush: Option<Hsv>,
    /// The picker edits the second colour.
    pub second: bool,
    /// Brush colours used lately, newest first.
    pub recent: Vec<Rgb>,
    /// The picker field being typed in, and its text.
    pub editing: Option<(Field, String)>,
    /// The next typed letter replaces the field's text.
    fresh: bool,
    /// Reset keymap was clicked once; the next click does it.
    pub confirm_reset: bool,
    pub presses: Presses,
    pub anim: Anim,
    /// Canvas pixels per design pixel.
    pub scale: f32,
    /// Frames drawn, for [`crate::view`]'s repaint workaround.
    pub frame: u32,
    pub lang: crate::lang::Lang,
    /// A message from Studio itself for the footer, until the next click.
    pub notice: Option<String>,
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

    /// Is the mouse dragging `hit`?
    pub fn dragging(&self, hit: Hit) -> bool {
        self.drag.is_some_and(|(h, _)| h == hit)
    }
}

impl Shared {
    /// Lighting settings as the UI shows them.
    pub fn lighting(&self) -> Option<Settings> {
        self.ui
            .draft
            .or_else(|| self.keyboard.lighting.map(|l| l.settings))
    }

    /// Does the page change from frame to frame by itself?
    pub fn animating(&self) -> bool {
        (self.ui.tab == Tab::Lighting && self.lighting().is_some()) || self.ui.editing.is_some()
    }

    /// Is the picker editing the per-key brush (else the effect colour)?
    pub fn brushing(&self) -> bool {
        self.lighting().is_some_and(|s| s.effect == Effect::PerKey)
    }

    /// Does the picker edit the second colour?
    pub fn editing_second(&self) -> bool {
        self.ui.second
            && !self.brushing()
            && self.lighting().is_some_and(|s| s.palette == Palette::Two)
    }

    /// The colour the picker shows.
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

    fn pick(&mut self, hsv: Hsv, tx: &Sender<Command>) {
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

    /// Keeps the key presses the previews show up to date.
    pub fn follow_presses(&mut self) {
        let Some(desc) = &self.keyboard.description else {
            return;
        };
        let down: Vec<bool> = desc
            .keys
            .iter()
            .map(|k| k.cell.is_some_and(|c| self.keyboard.closed(c)))
            .collect();
        let time = self.ui.anim.time();
        self.ui.presses.follow(&down, time);
    }

    /// Handles one window event; returns whether to redraw.
    pub fn event(&mut self, event: &WindowEvent, tx: &Sender<Command>) -> bool {
        if self.ui.editing.is_some()
            && let Some(redraw) = self.field_event(event, tx)
        {
            return redraw;
        }
        if let WindowEvent::KeyInput {
            key,
            pressed: true,
            modifiers,
        } = *event
            && modifiers.ctrl
            && self.ui.tab == Tab::Lighting
        {
            return self.copy_paste(key, tx);
        }
        match *event {
            WindowEvent::MouseMove { x, y } => {
                #[allow(clippy::cast_possible_truncation)]
                let now = (x as f32, y as f32);
                let before = self.ui.hovered();
                self.ui.mouse = now;
                if let Some((hit, area)) = self.ui.drag {
                    self.drag_to(hit, area, tx);
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
                button,
                pressed,
                x,
                y,
                ..
            } => {
                #[allow(clippy::cast_possible_truncation)]
                {
                    self.ui.mouse = (x as f32, y as f32);
                }
                self.mouse_button(button, pressed, tx)
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
                // A release outside the window never arrives.
                self.ui.mouse = (-1.0, -1.0);
                self.ui.drag = None;
                self.ui.painting = false;
                true
            }
            WindowEvent::KeyInput {
                key, pressed: true, ..
            } if self.ui.tab == Tab::Keymap && self.ui.selected.is_some() => {
                self.keymap_key(key, tx)
            }
            _ => false,
        }
    }

    /// Arrows move the selected key, Delete clears it.
    fn keymap_key(&mut self, key: KeyCode, tx: &Sender<Command>) -> bool {
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

    fn mouse_button(&mut self, button: MouseButton, pressed: bool, tx: &Sender<Command>) -> bool {
        match (button, pressed) {
            (MouseButton::Left, true) => self.click(tx),
            (MouseButton::Right, true) => self.right_click(),
            (MouseButton::Left, false) => {
                self.ui.drag = None;
                self.ui.painting = false;
            }
            _ => return false,
        }
        true
    }

    /// Ctrl+C copies the picked colour as hex, Ctrl+V takes a colour
    /// code.
    fn copy_paste(&mut self, key: KeyCode, tx: &Sender<Command>) -> bool {
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

    /// Typing into a picker field. `None` if the event is not for it.
    fn field_event(&mut self, event: &WindowEvent, tx: &Sender<Command>) -> Option<bool> {
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

    /// Starts typing into `field`.
    fn edit(&mut self, field: Field) {
        let text = field.text(self.picked().to_rgb());
        self.ui.editing = Some((field, text));
        self.ui.fresh = true;
    }

    /// Takes what was typed, if it is a colour.
    fn commit(&mut self, tx: &Sender<Command>) {
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

    fn click(&mut self, tx: &Sender<Command>) {
        self.ui.notice = None;
        self.keyboard.error = None;
        let hit = self.ui.hovered();
        if let Some((field, _)) = self.ui.editing
            && hit != Some(Hit::Field(field))
        {
            self.commit(tx);
        }
        if hit != Some(Hit::ResetKeymap) {
            self.ui.confirm_reset = false;
        }
        let Some(hit) = hit else {
            return;
        };
        match hit {
            Hit::Tab(t) => {
                if self.ui.tab != t {
                    self.ui.anim.set(Key::Page, 0.0);
                }
                self.ui.tab = t;
            }
            Hit::Lang => self.ui.lang = self.ui.lang.other(),
            Hit::Layer(l) => self.ui.layer = l,
            Hit::Key(k) if self.ui.tab == Tab::Lighting => {
                if self.brushing() {
                    self.ui.painting = true;
                    self.remember_brush();
                    self.paint(k, tx);
                } else {
                    // Try the press effects without the keyboard.
                    let time = self.ui.anim.time();
                    self.ui.presses.press(k, time);
                }
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
            _ => self.click_lighting(hit, tx),
        }
    }

    /// A click on one of the lighting controls.
    fn click_lighting(&mut self, hit: Hit, tx: &Sender<Command>) {
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
            Hit::Slider(_) | Hit::Square | Hit::HueBar => {
                if let Some((area, _)) = self.ui.hits.iter().rev().find(|(_, h)| *h == hit) {
                    let area = *area;
                    self.ui.drag = Some((hit, area));
                    self.drag_to(hit, area, tx);
                }
            }
            _ => {}
        }
    }

    /// Right click on a key in per-key painting takes its colour.
    fn right_click(&mut self) {
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

    /// Moves the selection to the nearest key in direction `(dx, dy)`.
    fn step_selection(&mut self, dx: f64, dy: f64) -> bool {
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
            self.ui.notice = Some(self.ui.lang.fill(
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

    /// Puts the brush first in the recent colours.
    fn remember_brush(&mut self) {
        let c = self.picked().to_rgb();
        self.ui.recent.retain(|r| *r != c);
        self.ui.recent.insert(0, c);
        self.ui.recent.truncate(RECENT);
    }

    /// Every key `color`.
    fn fill(&mut self, color: Rgb, tx: &Sender<Command>) {
        let count = self.keyboard.key_colors.len();
        self.keyboard.key_colors = vec![color; count];
        let _ = tx.send(Command::SetKeyColors {
            start: 0,
            colors: vec![color; count],
        });
    }

    /// Paints key `k` (by description index) with the brush.
    fn paint(&mut self, k: usize, tx: &Sender<Command>) {
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

    /// Follows the mouse on a dragged slider, square or hue bar.
    fn drag_to(&mut self, hit: Hit, area: Area, tx: &Sender<Command>) {
        let across = ((self.ui.mouse.0 - area.x) / area.w).clamp(0.0, 1.0);
        let down = ((self.ui.mouse.1 - area.y) / area.h).clamp(0.0, 1.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let byte = |t: f32| (t * 255.0).round() as u8;
        let mut c = self.picked();
        match hit {
            Hit::Slider(Slider::Brightness) => {
                self.change_lighting(tx, |s| s.color.v = byte(across));
            }
            Hit::Slider(Slider::Speed) => self.change_lighting(tx, |s| s.speed = byte(across)),
            Hit::Slider(Slider::Background) => {
                self.change_lighting(tx, |s| s.background = byte(across));
            }
            Hit::Slider(Slider::Size) => self.change_lighting(tx, |s| s.size = byte(across)),
            Hit::Square => {
                c.s = byte(across);
                c.v = byte(1.0 - down);
                self.pick(c, tx);
            }
            Hit::HueBar => {
                // 255 would wrap round to red at the right end.
                c.h = byte(across).min(254);
                self.pick(c, tx);
            }
            _ => {}
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presses_follow_the_matrix() {
        let mut p = Presses::default();
        p.follow(&[false, true], 1.0);
        p.follow(&[false, true], 2.0);
        assert_eq!(p.at, vec![None, Some(2.0)]);
        p.follow(&[true, false], 3.0);
        p.follow(&[true, true], 4.0);
        assert_eq!(p.at, vec![Some(4.0), Some(4.0)]);
        assert_eq!(p.recent.len(), 3);
        for i in 0..20 {
            p.press(0, f32::from(u8::try_from(i).unwrap()));
        }
        assert_eq!(p.recent.len(), RIPPLES);
    }

    #[test]
    fn fields_step_round() {
        assert_eq!(Field::Hex.next(), Field::Red);
        assert_eq!(Field::Blue.next(), Field::Hex);
        assert_eq!(Field::Hex.text(Rgb::new(1, 2, 255)), "#0102FF");
        assert!(Field::Red.takes('7') && !Field::Red.takes('a'));
    }
}
