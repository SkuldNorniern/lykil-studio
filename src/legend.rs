//! What a binding looks like on a keycap, and the palette of bindings to
//! pick from.

use lykil::binding::Binding;
use lykil::keycode::{ConsumerCode, KeyCode, Modifiers};
use lykil::layer::LayerId;
use lykil::oneshot::{OneShotTarget, Switch};

use crate::edit::MODS;
use lykil_config::names;

/// Short keycap text for key names that are too long.
const SHORT: &[(&str, &str)] = &[
    ("escape", "Esc"),
    ("enter", "Enter"),
    ("backspace", "Bksp"),
    ("tab", "Tab"),
    ("space", "Space"),
    ("minus", "-"),
    ("equal", "="),
    ("left-bracket", "["),
    ("right-bracket", "]"),
    ("backslash", "\\"),
    ("non-us-hash", "#"),
    ("semicolon", ";"),
    ("quote", "'"),
    ("grave", "`"),
    ("comma", ","),
    ("dot", "."),
    ("slash", "/"),
    ("caps-lock", "Caps"),
    ("print-screen", "PrtSc"),
    ("scroll-lock", "ScrLk"),
    ("pause", "Pause"),
    ("insert", "Ins"),
    ("home", "Home"),
    ("page-up", "PgUp"),
    ("delete", "Del"),
    ("end", "End"),
    ("page-down", "PgDn"),
    ("right", "Right"),
    ("left", "Left"),
    ("down", "Down"),
    ("up", "Up"),
    ("num-lock", "NumLk"),
    ("non-us-backslash", "\\|"),
    ("application", "Menu"),
    ("lctrl", "Ctrl"),
    ("lshift", "Shift"),
    ("lalt", "Alt"),
    ("lgui", "Win"),
    ("rctrl", "RCtrl"),
    ("rshift", "RShift"),
    ("ralt", "RAlt"),
    ("rgui", "RWin"),
    ("mute", "Mute"),
    ("volume-up", "Vol+"),
    ("volume-down", "Vol-"),
    ("next-track", "Next"),
    ("prev-track", "Prev"),
    ("stop", "Stop"),
    ("play-pause", "Play"),
    ("brightness-up", "Bri+"),
    ("brightness-down", "Bri-"),
    ("fast-forward", "FFwd"),
    ("rewind", "Rew"),
    ("calculator", "Calc"),
    ("my-computer", "PC"),
    ("www-search", "Search"),
    ("www-home", "Web"),
    ("www-back", "Back"),
    ("www-forward", "Fwd"),
    ("show-all-windows", "Tasks"),
    ("assistant", "Assist"),
];

fn short(name: &str) -> String {
    if let Some((_, s)) = SHORT.iter().find(|(n, _)| *n == name) {
        return (*s).to_string();
    }
    if let Some(rest) = name.strip_prefix("kp-") {
        return format!("P{rest}");
    }
    if name.len() == 1 || name.starts_with('f') && name[1..].parse::<u8>().is_ok() {
        return name.to_uppercase();
    }
    let mut s: String = name.chars().take(6).collect();
    if let Some(first) = s.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    s
}

fn key(k: KeyCode) -> String {
    names::key_name(k).map_or_else(|| format!("0x{:02X}", k.0), short)
}

fn mods(m: Modifiers) -> String {
    let parts: Vec<&str> = [
        (Modifiers::LCTRL.0 | Modifiers::RCTRL.0, "Ctrl"),
        (Modifiers::LSHIFT.0 | Modifiers::RSHIFT.0, "Shift"),
        (Modifiers::LALT.0 | Modifiers::RALT.0, "Alt"),
        (Modifiers::LGUI.0 | Modifiers::RGUI.0, "Win"),
    ]
    .into_iter()
    .filter(|(bits, _)| m.0 & bits != 0)
    .map(|(_, n)| n)
    .collect();
    parts.join("+")
}

fn layer(l: LayerId, layers: &[String]) -> String {
    layers
        .get(usize::from(l.0))
        .cloned()
        .unwrap_or_else(|| format!("L{}", l.0))
}

/// Keycap text: the main line and an optional small second line (what a
/// dual-role key does when held, or what kind of layer key it is).
pub fn keycap(b: Binding, layers: &[String]) -> (String, Option<String>) {
    match b {
        Binding::None => (String::new(), None),
        Binding::Transparent => ("\u{b7}".into(), None),
        Binding::Key(k) => (key(k), None),
        Binding::ModifiedKey(m, k) => (key(k), Some(mods(m))),
        Binding::Modifiers(m) => (mods(m), None),
        Binding::MomentaryLayer(l) => (layer(l, layers), Some("hold".into())),
        Binding::LayerModifiers { layer: l, mods: m } => (layer(l, layers), Some(mods(m))),
        Binding::ToggleLayer(l) => (layer(l, layers), Some("toggle".into())),
        Binding::ToLayer(l) => (layer(l, layers), Some("to".into())),
        Binding::DefaultLayer(l) => (layer(l, layers), Some("default".into())),
        Binding::TapToggleLayer(l) => (layer(l, layers), Some("tap-tog".into())),
        Binding::ModTap { tap, hold } => (key(tap), Some(mods(hold))),
        Binding::LayerTap { tap, layer: l } => (key(tap), Some(layer(l, layers))),
        Binding::Consumer(c) => (
            names::consumer_name(c).map_or_else(|| format!("C{:03X}", c.0), short),
            None,
        ),
        Binding::System(s) => (
            names::system_name(s).map_or_else(|| format!("S{:02X}", s.0), short),
            None,
        ),
        Binding::OneShot(OneShotTarget::Modifiers(m)) => (mods(m), Some("one-shot".into())),
        Binding::OneShot(OneShotTarget::Layer(l)) => (layer(l, layers), Some("one-shot".into())),
        Binding::OneShotSwitch(s) => (
            match s {
                Switch::On => "OS on",
                Switch::Off => "OS off",
                Switch::Toggle => "OS tog",
            }
            .into(),
            None,
        ),
    }
}

/// The binding as `keymap.tav` writes it, for the details line.
pub fn full(b: Binding, layers: &[String]) -> String {
    let refs: Vec<&str> = layers.iter().map(String::as_str).collect();
    lykil_config::emit::binding(&b, &refs).unwrap_or_else(|_| format!("{b:?}"))
}

/// One group of the binding palette.
pub struct Group {
    pub name: &'static str,
    pub items: Vec<Binding>,
}

fn keys(range: impl IntoIterator<Item = u8>) -> Vec<Binding> {
    range
        .into_iter()
        .filter(|c| names::key_name(KeyCode(*c)).is_some())
        .map(|c| Binding::Key(KeyCode(c)))
        .collect()
}

/// Everything the palette offers, for a keymap with `layers`.
pub fn palette(layers: &[String]) -> Vec<Group> {
    let n = u8::try_from(layers.len()).unwrap_or(u8::MAX);
    let consumer = [
        0xE2, 0xEA, 0xE9, 0xCD, 0xB6, 0xB5, 0xB7, 0x70, 0x6F, 0x192, 0x194, 0x18A, 0x221, 0x223,
        0x29F,
    ];
    vec![
        Group {
            name: "Letters",
            items: keys(0x04..=0x1D),
        },
        Group {
            name: "Numbers and symbols",
            items: keys((0x1E..=0x27).chain(0x2D..=0x38)),
        },
        Group {
            name: "Editing",
            items: keys((0x28..=0x2C).chain([0x39, 0x49, 0x4C, 0x65])),
        },
        Group {
            name: "Navigation",
            items: keys([0x4A, 0x4D, 0x4B, 0x4E, 0x50, 0x52, 0x51, 0x4F]),
        },
        Group {
            name: "Function",
            items: keys((0x3A..=0x45).chain([0x46, 0x47, 0x48]).chain(0x68..=0x73)),
        },
        Group {
            name: "Modifiers",
            items: keys(0xE0..=0xE7),
        },
        Group {
            name: "Media",
            items: consumer
                .into_iter()
                .map(|c| Binding::Consumer(ConsumerCode(c)))
                .collect(),
        },
        Group {
            name: "Keypad",
            items: keys(0x53..=0x63),
        },
        Group {
            name: "Layers: hold",
            items: (0..n)
                .map(|l| Binding::MomentaryLayer(LayerId(l)))
                .collect(),
        },
        Group {
            name: "Layers: toggle",
            items: (0..n).map(|l| Binding::ToggleLayer(LayerId(l))).collect(),
        },
        Group {
            name: "One-shot",
            items: MODS
                .iter()
                .map(|(_, m)| Binding::OneShot(OneShotTarget::Modifiers(*m)))
                .chain((0..n).map(|l| Binding::OneShot(OneShotTarget::Layer(LayerId(l)))))
                .collect(),
        },
        Group {
            name: "Special",
            items: vec![Binding::Transparent, Binding::None],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keycaps() {
        let layers = vec!["base".to_string(), "fn".to_string()];
        assert_eq!(keycap(Binding::Key(KeyCode(0x04)), &layers).0, "A");
        assert_eq!(keycap(Binding::Key(KeyCode(0x3A)), &layers).0, "F1");
        assert_eq!(keycap(Binding::Key(KeyCode(0x29)), &layers).0, "Esc");
        assert_eq!(
            keycap(Binding::MomentaryLayer(LayerId(1)), &layers),
            ("fn".into(), Some("hold".into()))
        );
        assert_eq!(
            keycap(
                Binding::ModTap {
                    tap: KeyCode(0x04),
                    hold: Modifiers::LSHIFT
                },
                &layers
            ),
            ("A".into(), Some("Shift".into()))
        );
        assert_eq!(keycap(Binding::None, &layers).0, "");
    }

    #[test]
    fn palette_has_every_letter_and_layer() {
        let layers = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let p = palette(&layers);
        assert_eq!(p[0].items.len(), 26);
        assert_eq!(
            p.iter()
                .find(|g| g.name == "Layers: hold")
                .unwrap()
                .items
                .len(),
            3
        );
        assert!(p.iter().all(|g| !g.items.is_empty()));
    }
}
