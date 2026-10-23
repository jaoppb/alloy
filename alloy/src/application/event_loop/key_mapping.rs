//! Interim `KeyEvent` → `OmniboxInput` mapping.
//!
//! Temporary: `core/ui`'s router and data-driven keymap replace it (#113,
//! epic #110). Until then it applies the router's rule for text under
//! command modifiers (epic decision 18) so the port's unified key event does
//! not reintroduce #117: `winit`'s `KeyEvent::text` ignores Ctrl on Linux,
//! so Ctrl+L arrives with text `"l"` — a press is typing only when no
//! command modifier is held.
//!
//! Known gap, left to #113: on Windows `AltGr` is reported as Ctrl+Alt with no
//! `alt_graph` bit, so `AltGr` text is dropped here until the router adds its
//! platform fallback (decision 18, step 2).

use window::{KeyCode, KeyEvent, LogicalKey, Modifiers};

use crate::application::omnibox::OmniboxInput;

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
    if is_edit_key(event.logical()) {
        return Some(OmniboxInput::Edit(event.logical().clone()));
    }
    if is_typing(event.modifiers()) {
        return event.text().cloned().map(OmniboxInput::Insert);
    }
    None
}

fn is_focus_shortcut(event: &KeyEvent) -> bool {
    let modifiers = event.modifiers();
    let ctrl_or_super_l =
        (modifiers.control() || modifiers.super_key()) && event.physical() == KeyCode::KEY_L;
    let alt_d = modifiers.alt() && event.physical() == KeyCode::KEY_D;
    ctrl_or_super_l || alt_d
}

const fn is_edit_key(key: &LogicalKey) -> bool {
    matches!(
        key,
        LogicalKey::Enter
            | LogicalKey::Escape
            | LogicalKey::Backspace
            | LogicalKey::Delete
            | LogicalKey::ArrowLeft
            | LogicalKey::ArrowRight
            | LogicalKey::Home
            | LogicalKey::End
    )
}

/// Text is typed unless Ctrl, Super or Alt is held without `AltGr`; Shift never
/// blocks it.
const fn is_typing(modifiers: Modifiers) -> bool {
    if modifiers.alt_graph() {
        return true;
    }
    !(modifiers.control() || modifiers.super_key() || modifiers.alt())
}
