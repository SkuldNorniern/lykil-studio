//! Changing one part of a binding: hold action and modifiers.

use lykil::binding::Binding;
use lykil::keycode::{KeyCode, Modifiers};
use lykil::layer::LayerId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hold {
    Nothing,
    Mods(Modifiers),
    Layer(LayerId),
}

pub const MODS: [(&str, Modifiers); 4] = [
    ("Ctrl", Modifiers::LCTRL),
    ("Shift", Modifiers::LSHIFT),
    ("Alt", Modifiers::LALT),
    ("Win", Modifiers::LGUI),
];

pub const fn tap_key(b: Binding) -> Option<KeyCode> {
    match b {
        Binding::Key(k)
        | Binding::ModifiedKey(_, k)
        | Binding::ModTap { tap: k, .. }
        | Binding::LayerTap { tap: k, .. } => Some(k),
        _ => None,
    }
}

pub const fn hold(b: Binding) -> Option<Hold> {
    match b {
        Binding::Key(_) | Binding::ModifiedKey(..) => Some(Hold::Nothing),
        Binding::ModTap { hold, .. } => Some(Hold::Mods(hold)),
        Binding::LayerTap { layer, .. } => Some(Hold::Layer(layer)),
        _ => None,
    }
}

/// `b` with its hold action set to `hold`. A key sent with modifiers
/// loses them when it becomes dual-role.
pub const fn set_hold(b: Binding, hold: Hold) -> Option<Binding> {
    let Some(tap) = tap_key(b) else {
        return None;
    };
    Some(match hold {
        Hold::Nothing => Binding::Key(tap),
        Hold::Mods(m) => Binding::ModTap { tap, hold: m },
        Hold::Layer(layer) => Binding::LayerTap { tap, layer },
    })
}

pub const fn with(b: Binding) -> Option<Modifiers> {
    match b {
        Binding::Key(_) => Some(Modifiers::NONE),
        Binding::ModifiedKey(m, _) => Some(m),
        _ => None,
    }
}

pub const fn toggle_with(b: Binding, m: Modifiers) -> Option<Binding> {
    let (Some(mods), Some(key)) = (with(b), tap_key(b)) else {
        return None;
    };
    let mods = Modifiers(mods.0 ^ m.0);
    Some(if mods.0 == 0 {
        Binding::Key(key)
    } else {
        Binding::ModifiedKey(mods, key)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: KeyCode = KeyCode(0x04);

    #[test]
    fn hold_turns_a_key_dual_role_and_back() {
        let shift_a = set_hold(Binding::Key(A), Hold::Mods(Modifiers::LSHIFT)).unwrap();
        assert_eq!(
            shift_a,
            Binding::ModTap {
                tap: A,
                hold: Modifiers::LSHIFT
            }
        );
        assert_eq!(hold(shift_a), Some(Hold::Mods(Modifiers::LSHIFT)));
        let layer = set_hold(shift_a, Hold::Layer(LayerId(1))).unwrap();
        assert_eq!(
            layer,
            Binding::LayerTap {
                tap: A,
                layer: LayerId(1)
            }
        );
        assert_eq!(set_hold(layer, Hold::Nothing), Some(Binding::Key(A)));
        assert_eq!(set_hold(Binding::Transparent, Hold::Nothing), None);
        assert_eq!(hold(Binding::MomentaryLayer(LayerId(1))), None);
    }

    #[test]
    fn modifiers_toggle_on_and_off() {
        let shifted = toggle_with(Binding::Key(A), Modifiers::LSHIFT).unwrap();
        assert_eq!(shifted, Binding::ModifiedKey(Modifiers::LSHIFT, A));
        let both = toggle_with(shifted, Modifiers::LCTRL).unwrap();
        assert_eq!(with(both), Some(Modifiers::LSHIFT.union(Modifiers::LCTRL)));
        let back = toggle_with(
            toggle_with(both, Modifiers::LCTRL).unwrap(),
            Modifiers::LSHIFT,
        );
        assert_eq!(back, Some(Binding::Key(A)));
        assert_eq!(
            toggle_with(
                Binding::ModTap {
                    tap: A,
                    hold: Modifiers::LALT
                },
                Modifiers::LSHIFT
            ),
            None
        );
    }
}
