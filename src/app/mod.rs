//! UI state and input, matched against the last frame's hits.

mod desk;
mod keymap;
mod lighting;
mod macros;
mod presses;

pub use presses::Presses;

use std::sync::mpsc::Sender;

use aurea::{KeyCode, MouseButton, WindowEvent};
use lykil::binding::Binding;
use lykil::keycode::Modifiers;
use lykil::lighting::{Effect, Hsv, Palette, Rgb, Settings};

use crate::anim::{Anim, Key};
use crate::colour;
use crate::device::{Command, Keyboard};
use crate::draw::Area;
use crate::edit::{self, Hold};

#[derive(Debug, Default)]
pub struct Shared {
    pub keyboard: Keyboard,
    pub ui: Ui,
    pub lamps: Vec<crate::lamps::Lamp>,
    pub lamp_sync: bool,
    pub lamp_tx: Option<Sender<crate::lamps::LampCommand>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tab {
    #[default]
    Keymap,
    Macros,
    Lighting,
    Device,
    Windows,
}

impl Tab {
    /// Dynamic Lighting is Windows only, so is its tab.
    #[cfg(windows)]
    pub const ALL: [Self; 5] = [
        Self::Keymap,
        Self::Macros,
        Self::Lighting,
        Self::Device,
        Self::Windows,
    ];
    #[cfg(not(windows))]
    pub const ALL: [Self; 4] = [Self::Keymap, Self::Macros, Self::Lighting, Self::Device];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Keymap => "Keymap",
            Self::Macros => "Macros",
            Self::Lighting => "Lighting",
            Self::Device => "Device",
            Self::Windows => "Windows",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slider {
    Brightness,
    Speed,
    Background,
    Size,
}

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
    Group(usize),
    Palette(Binding),
    Hold(Hold),
    With(Modifiers),
    ResetKeymap,
    Lang,
    PaintAll,
    ClearAll,
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
    Swatch(Rgb),
    Colours(Palette),
    LightingSettings,
    ViaFolder,
    LampSync(bool),
    LampFollow(usize),
    /// A device on the desk plane, by its place in the list.
    DeskDevice(usize),
    /// The brightness of the device picked on the desk.
    DeviceLevel,
    Second(bool),
    LayerKeys(bool),
    /// VIA settings, by their place in `Keyboard::via_settings`.
    ViaRange(usize),
    ViaOption(usize, u8),
    ViaToggle(usize, bool),
    ViaHue(usize),
    ViaSat(usize),
}

/// The tab Ctrl + `key` goes to from `now`: Ctrl+1 to Ctrl+5, and
/// Ctrl+Tab (with Shift, backwards) round the bar.
fn tab_shortcut(key: KeyCode, shift: bool, now: Tab) -> Option<Tab> {
    let n = Tab::ALL.len();
    let at = match key {
        KeyCode::Key1 => 0,
        KeyCode::Key2 => 1,
        KeyCode::Key3 => 2,
        KeyCode::Key4 => 3,
        KeyCode::Key5 => 4,
        KeyCode::Tab if shift => (now.index() + n - 1) % n,
        KeyCode::Tab => (now.index() + 1) % n,
        _ => return None,
    };
    Tab::ALL.get(at).copied()
}

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

const RECENT: usize = 8;

/// A desk device being moved: where it and the mouse were when the drag
/// began, and pixels per metre.
#[derive(Clone, Debug)]
struct DeskGrab {
    id: String,
    from: (f32, f32),
    mouse: (f32, f32),
    scale: f32,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
pub struct Ui {
    pub tab: Tab,
    pub layer: u8,
    pub selected: Option<usize>,
    pub group: usize,
    pub macro_id: usize,
    pub macro_text: Option<String>,
    pub mouse: (f32, f32),
    pub hits: Vec<(Area, Hit)>,
    drag: Option<(Hit, Area)>,
    /// A desk device being moved: its id, where it and the mouse were
    /// when the drag began, and pixels per metre.
    desk_grab: Option<DeskGrab>,
    /// The device picked on the desk.
    pub desk_selected: Option<String>,
    painting: bool,
    /// Lighting settings sent but not confirmed yet, so controls follow the
    /// mouse at once.
    pub draft: Option<Settings>,
    /// Per-key brush; until picked, the lighting colour at full value.
    pub brush: Option<Hsv>,
    pub second: bool,
    pub recent: Vec<Rgb>,
    pub editing: Option<(Field, String)>,
    /// The next typed letter replaces the field's text.
    fresh: bool,
    pub confirm_reset: bool,
    pub presses: Presses,
    pub anim: Anim,
    pub scale: f32,
    pub lang: crate::lang::Lang,
    pub notice: Option<String>,
}

impl Ui {
    pub fn hovered(&self) -> Option<Hit> {
        let (x, y) = self.mouse;
        self.hits
            .iter()
            .rev()
            .find(|(a, _)| a.contains(x, y))
            .map(|(_, h)| *h)
    }

    pub fn dragging(&self, hit: Hit) -> bool {
        self.drag.is_some_and(|(h, _)| h == hit)
    }
}

impl Shared {
    pub fn lighting(&self) -> Option<Settings> {
        self.ui
            .draft
            .or_else(|| self.keyboard.lighting.map(|l| l.settings))
    }

    pub fn animating(&self) -> bool {
        (self.ui.tab == Tab::Lighting && self.lighting().is_some())
            || (self.ui.tab == Tab::Windows && self.lamp_sync)
            || self.ui.editing.is_some()
    }

    pub fn follow_presses(&mut self) {
        let Some(desc) = &self.keyboard.description else {
            return;
        };
        let down: Vec<bool> = desc
            .keys
            .iter()
            .map(|k| k.cell.is_some_and(|c| self.keyboard.closed(c)))
            .collect();
        let points = crate::pages::lighting::points(desc);
        let size = self.lighting().map_or(0, |s| s.size);
        self.ui.presses.set_reach(&points, size);
        let time = self.ui.anim.time();
        self.ui.presses.follow(&down, time);
    }

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
            && let Some(redraw) = self.shortcut(key, modifiers.shift, tx)
        {
            return redraw;
        }
        match *event {
            WindowEvent::MouseMove { x, y, buttons, .. } => {
                // A release that went missing: the drag is over.
                if buttons.is_empty() && (self.ui.drag.is_some() || self.ui.painting) {
                    self.mouse_button(MouseButton::Left, false, tx);
                }
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
            // Aurea holds the leave while a button is down, so a drag ends
            // on its release, not here.
            WindowEvent::MouseExited => {
                self.ui.mouse = (-1.0, -1.0);
                self.ui.painting = false;
                true
            }
            WindowEvent::MouseWheel { delta_y, .. } => self.wheel(delta_y, tx),
            WindowEvent::KeyInput {
                key, pressed: true, ..
            } if self.ui.tab == Tab::Keymap && self.ui.selected.is_some() => {
                self.keymap_key(key, tx)
            }
            _ => false,
        }
    }

    /// The wheel nudges the slider or hue under the mouse, or steps
    /// through tabs and layers.
    fn wheel(&mut self, delta_y: f64, tx: &Sender<Command>) -> bool {
        let up = delta_y > 0.0;
        let step = |v: u8| {
            if up {
                v.saturating_add(4)
            } else {
                v.saturating_sub(4)
            }
        };
        match self.ui.hovered() {
            Some(Hit::Slider(which)) => {
                self.change_lighting(tx, |s| match which {
                    Slider::Brightness => s.color.v = step(s.color.v),
                    Slider::Speed => s.speed = step(s.speed),
                    Slider::Background => s.background = step(s.background),
                    Slider::Size => s.size = step(s.size),
                });
            }
            Some(Hit::HueBar) => {
                let mut c = self.picked();
                c.h = step(c.h).min(254);
                self.pick(c, tx);
            }
            Some(Hit::Tab(_)) => {
                let at = self.ui.tab.index();
                let next = if up {
                    at.checked_sub(1)
                } else {
                    Some(at + 1).filter(|n| *n < Tab::ALL.len())
                };
                if let Some(n) = next {
                    self.switch_tab(Tab::ALL[n]);
                }
            }
            Some(Hit::Layer(_)) => {
                let layers = self.keyboard.layer_names().len();
                let l = usize::from(self.ui.layer);
                let next = if up {
                    l.checked_sub(1)
                } else {
                    Some(l + 1).filter(|n| *n < layers)
                };
                if let Some(n) = next.and_then(|n| u8::try_from(n).ok()) {
                    self.ui.layer = n;
                }
            }
            _ => return false,
        }
        true
    }

    /// Ctrl with `key`: switching pages, or copy and paste on the
    /// lighting page. `None` if it means nothing.
    fn shortcut(&mut self, key: KeyCode, shift: bool, tx: &Sender<Command>) -> Option<bool> {
        if let Some(tab) = tab_shortcut(key, shift, self.ui.tab) {
            self.switch_tab(tab);
            return Some(true);
        }
        (self.ui.tab == Tab::Lighting).then(|| self.copy_paste(key, tx))
    }

    fn switch_tab(&mut self, tab: Tab) {
        if self.ui.tab != tab {
            self.ui.anim.set(Key::Page, 0.0);
        }
        self.ui.tab = tab;
    }

    fn mouse_button(&mut self, button: MouseButton, pressed: bool, tx: &Sender<Command>) -> bool {
        match (button, pressed) {
            (MouseButton::Left, true) => self.click(tx),
            (MouseButton::Right, true) => self.right_click(),
            (MouseButton::Left, false) => {
                self.ui.drag = None;
                if let Some(grab) = self.ui.desk_grab.take() {
                    self.drop_device(&grab.id);
                }
                self.ui.painting = false;
            }
            _ => return false,
        }
        true
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
            Hit::Tab(t) => self.switch_tab(t),
            Hit::LampSync(_) | Hit::DeskDevice(_) | Hit::DeviceLevel | Hit::LampFollow(_) => {
                self.click_desk(hit, tx);
            }
            Hit::ViaFolder => crate::reveal(crate::via::folder()),
            Hit::LightingSettings => crate::reveal("ms-settings:personalization-lighting"),
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
            Hit::DeskDevice(i) => self.move_device(i),
            Hit::ViaRange(i) => {
                let span = self
                    .keyboard
                    .via_settings
                    .get(i)
                    .map(|s| match s.control.kind {
                        lykil_qmk::import::ViaControlKind::Range { min, max } => (min, max),
                        _ => (0, 255),
                    });
                if let Some((min, max)) = span {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let v = min + (across * f32::from(max.saturating_sub(min))).round() as u8;
                    self.set_via(i, 0, v, tx);
                }
            }
            Hit::ViaHue(i) => self.set_via(i, 0, byte(across).min(254), tx),
            Hit::ViaSat(i) => self.set_via(i, 1, byte(across), tx),
            Hit::DeviceLevel => {
                let id = self.ui.desk_selected.clone();
                if let Some(l) = self.lamps.iter_mut().find(|l| Some(&l.id) == id.as_ref()) {
                    l.place.level = byte(across);
                    let command = crate::lamps::LampCommand::Level(l.id.clone(), l.place.level);
                    self.lamp(command);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use lykil::lighting::RIPPLES;

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
    fn tab_shortcuts_go_round() {
        assert_eq!(
            tab_shortcut(KeyCode::Key3, false, Tab::Keymap),
            Some(Tab::Lighting)
        );
        let last = Tab::ALL[Tab::ALL.len() - 1];
        assert_eq!(tab_shortcut(KeyCode::Tab, false, last), Some(Tab::Keymap));
        assert_eq!(tab_shortcut(KeyCode::Tab, true, Tab::Keymap), Some(last));
        assert_eq!(tab_shortcut(KeyCode::A, false, Tab::Keymap), None);
    }

    #[test]
    fn refused_lighting_drops_the_draft() {
        let mut s = Shared::default();
        let keyboard = lykil_protocol::lcp::LightingInfo {
            settings: Settings::DEFAULT,
            leds: 1,
            limit: 255,
            host: false,
            drivers_ok: true,
        };
        s.keyboard.lighting = Some(keyboard);
        s.ui.draft = Some(Settings {
            effect: Effect::Ripple,
            ..Settings::DEFAULT
        });
        s.settle();
        assert!(s.ui.draft.is_some(), "still waiting for the keyboard");
        s.keyboard.error = Some("BadValue".into());
        s.settle();
        assert_eq!(s.lighting(), Some(Settings::DEFAULT));
    }

    #[test]
    fn fields_step_round() {
        assert_eq!(Field::Hex.next(), Field::Red);
        assert_eq!(Field::Blue.next(), Field::Hex);
        assert_eq!(Field::Hex.text(Rgb::new(1, 2, 255)), "#0102FF");
        assert!(Field::Red.takes('7') && !Field::Red.takes('a'));
    }
}
