//! Macros page input.

use std::sync::mpsc::Sender;

use crate::device::Command;

use super::Shared;

impl Shared {
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

    pub(crate) fn send_macro(&mut self, steps: Vec<lykil::macros::Step>, tx: &Sender<Command>) {
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
}
