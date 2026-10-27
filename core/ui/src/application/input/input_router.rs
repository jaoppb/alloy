//! [`InputRouter`] — turns window key events into [`UiInput`] (`ADR-0027`).

use platform::HostOs;
use window::{InputText, KeyEvent, Modifiers};

use crate::domain::input::{FocusTarget, Keymap, UiInput};
use crate::domain::omnibox::EditKey;

/// Decides what each key event means.
///
/// A key event is a shortcut, typing, editing or nothing. The window adapter
/// never makes this call: `KeyEvent::text` ignores Ctrl on Linux, so Ctrl+L
/// carries `"l"` and only `modifiers` tell the two apart (#117).
#[derive(Clone, Debug)]
pub struct InputRouter {
    keymap: Keymap,
    host: HostOs,
}

impl InputRouter {
    #[must_use]
    pub const fn new(keymap: Keymap, host: HostOs) -> Self {
        Self { keymap, host }
    }

    /// Replaces the bindings; the router's other rules do not change.
    pub fn set_keymap(&mut self, keymap: Keymap) {
        self.keymap = keymap;
    }

    /// The input `event` stands for while `focus` holds the keyboard, or
    /// `None` when it stands for none.
    ///
    /// For a press or auto-repeat, in order: (1) a keymap binding wins, so
    /// Ctrl+AltGr+L stays bindable; (2) `AltGr` + text inserts; (3) on Windows,
    /// exactly Ctrl+Alt + text inserts (the left Ctrl+Alt Windows accepts as
    /// `AltGr`); (4) Ctrl, Super or Alt without `AltGr` drops the press;
    /// (5) otherwise a named editing key edits and text inserts. Shift never
    /// blocks. A release never produces input, and a held binding fires once.
    #[must_use]
    pub fn route_key(&self, event: &KeyEvent, focus: FocusTarget) -> Option<UiInput> {
        if event.state().is_released() {
            return None;
        }
        let input = self.resolve(event)?;
        input.reaches(focus).then_some(input)
    }

    /// The input an input-method commit stands for.
    #[must_use]
    pub fn route_text(&self, text: InputText, focus: FocusTarget) -> Option<UiInput> {
        let input = UiInput::Insert(text);
        input.reaches(focus).then_some(input)
    }

    fn resolve(&self, event: &KeyEvent) -> Option<UiInput> {
        if let Some(command) = self.keymap.resolve(event) {
            return event
                .state()
                .is_pressed()
                .then_some(UiInput::Command(command));
        }
        let modifiers = event.modifiers();
        let text = event.text().cloned().map(UiInput::Insert);
        if text.is_some() && (modifiers.alt_graph() || self.is_windows_alt_gr(modifiers)) {
            return text;
        }
        if self.holds_command_modifier(modifiers) {
            return None;
        }
        EditKey::from_logical(event.logical())
            .map(UiInput::Edit)
            .or(text)
    }

    /// Windows reports the left Ctrl+Alt, which it also accepts as `AltGr`, as
    /// plain Ctrl and Alt with no `alt_graph` bit. Shift never blocks.
    const fn is_windows_alt_gr(&self, modifiers: Modifiers) -> bool {
        matches!(self.host, HostOs::Windows)
            && modifiers.control()
            && modifiers.alt()
            && !modifiers.super_key()
    }

    /// Whether Ctrl, Super or Alt is held without `AltGr`. On macOS Alt is
    /// Option, which composes text, so only Ctrl and Cmd count there.
    const fn holds_command_modifier(&self, modifiers: Modifiers) -> bool {
        if modifiers.alt_graph() {
            return false;
        }
        let alt_is_command = !matches!(self.host, HostOs::MacOs);
        modifiers.control() || modifiers.super_key() || (modifiers.alt() && alt_is_command)
    }
}
