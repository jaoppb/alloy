//! [`InputRouter`] — turns window key events into [`UiInput`] (`ADR-0027`).

use platform::HostOs;
use window::{InputText, KeyEvent, Modifiers};

use super::modifier_policy::{AltMeaning, ModifierPolicy};
use crate::domain::input::{Keymap, UiInput};
use crate::domain::omnibox::EditKey;

/// Decides what each key event means.
///
/// A key event is a shortcut, typing, editing or nothing. The window adapter
/// never makes this call: `KeyEvent::text` ignores Ctrl on Linux, so Ctrl+L
/// carries `"l"` and only `modifiers` tell the two apart (#117). Who receives
/// the result is the controller's call, not the router's.
#[derive(Clone, Debug)]
pub struct InputRouter {
    keymap: Keymap,
    policy: ModifierPolicy,
}

impl InputRouter {
    #[must_use]
    pub const fn new(keymap: Keymap, host: HostOs) -> Self {
        Self {
            keymap,
            policy: ModifierPolicy::for_host(host),
        }
    }

    /// Replaces the bindings; the router's other rules do not change.
    pub fn set_keymap(&mut self, keymap: Keymap) {
        self.keymap = keymap;
    }

    /// The input `event` stands for, or `None` when it stands for none.
    ///
    /// For a press or auto-repeat, in order: (1) a keymap binding wins, so
    /// Ctrl+AltGr+L stays bindable; (2) `AltGr` + text inserts; (3) on Windows,
    /// exactly Ctrl+Alt + text inserts (the left Ctrl+Alt Windows accepts as
    /// `AltGr`); (4) Ctrl, Super or Alt without `AltGr` drops the press;
    /// (5) otherwise a named editing key edits and text inserts. Shift never
    /// blocks. A release never produces input, and a held binding fires once.
    #[must_use]
    pub fn route_key(&self, event: &KeyEvent) -> Option<UiInput> {
        if event.state().is_released() {
            return None;
        }
        if let Some(command) = self.keymap.resolve(event) {
            return event
                .state()
                .is_pressed()
                .then_some(UiInput::Command(command));
        }
        self.composed_text(event)
            .or_else(|| self.edit_or_typed(event))
    }

    /// The input an input-method commit stands for.
    #[must_use]
    pub const fn route_text(text: InputText) -> UiInput {
        UiInput::Insert(text)
    }

    /// Steps 2 and 3: text produced with `AltGr`, or its Windows stand-in.
    fn composed_text(&self, event: &KeyEvent) -> Option<UiInput> {
        let modifiers = event.modifiers();
        if !(modifiers.alt_graph() || self.policy.is_alt_gr(modifiers)) {
            return None;
        }
        event.text().cloned().map(UiInput::Insert)
    }

    /// Steps 4 and 5: nothing under a command modifier, else edit or type.
    fn edit_or_typed(&self, event: &KeyEvent) -> Option<UiInput> {
        let modifiers = event.modifiers();
        if let Some(edit) = EditKey::from_logical(event.logical()) {
            return self
                .permits(modifiers, AltMeaning::Command)
                .then_some(UiInput::Edit(edit));
        }
        let text = event.text().cloned().map(UiInput::Insert);
        self.permits(modifiers, AltMeaning::Composer)
            .then_some(text)
            .flatten()
    }

    const fn permits(&self, modifiers: Modifiers, alt: AltMeaning) -> bool {
        !self.policy.holds_command_modifier(modifiers, alt)
    }
}
