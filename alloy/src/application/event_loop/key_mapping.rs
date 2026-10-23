//! Interim `KeyEvent` → `OmniboxInput` mapping.
//!
//! Temporary: `core/ui`'s router and data-driven keymap replace it (#113,
//! epic #110). Until then it applies the router's rule for keys under command
//! modifiers (epic decision 18) so the port's unified key event does not
//! reintroduce #117: `winit`'s `KeyEvent::text` ignores Ctrl on Linux, so
//! Ctrl+L arrives with text `"l"` — a press types or edits only when no
//! command modifier is held.
//!
//! Known gaps, left to #113: on Windows `AltGr` is reported as Ctrl+Alt with
//! no `alt_graph` bit, and on macOS Option is reported as Alt, so `AltGr` and
//! Option text is dropped here until the router adds its platform rules
//! (decision 18 step 2, decision 24).

use window::{KeyCode, KeyEvent, Modifiers};

use crate::application::omnibox::{EditKey, OmniboxInput};

/// The omnibox input `event` stands for, or `None` when it stands for none.
///
/// Shortcuts match the **physical** key — what the hardcoded shortcut matched
/// before — so they survive a non-Latin layout.
pub fn omnibox_input(event: &KeyEvent) -> Option<OmniboxInput> {
    if event.state().is_released() {
        return None;
    }
    if event.state().is_pressed() && is_focus_shortcut(event) {
        return Some(OmniboxInput::Focus);
    }
    if !is_unmodified_by_commands(event.modifiers()) {
        return None;
    }
    EditKey::from_logical(event.logical())
        .map(OmniboxInput::Edit)
        .or_else(|| event.text().cloned().map(OmniboxInput::Insert))
}

fn is_focus_shortcut(event: &KeyEvent) -> bool {
    let modifiers = event.modifiers();
    let ctrl_or_super_l =
        (modifiers.control() || modifiers.super_key()) && event.physical() == KeyCode::KEY_L;
    let alt_d = modifiers.alt() && event.physical() == KeyCode::KEY_D;
    ctrl_or_super_l || alt_d
}

/// Typing and editing happen unless Ctrl, Super or Alt is held without
/// `AltGr`; Shift never blocks them. A chord is the keymap's to bind, so an
/// unbound Ctrl+K types nothing and Ctrl+Backspace edits nothing.
const fn is_unmodified_by_commands(modifiers: Modifiers) -> bool {
    if modifiers.alt_graph() {
        return true;
    }
    !(modifiers.control() || modifiers.super_key() || modifiers.alt())
}
