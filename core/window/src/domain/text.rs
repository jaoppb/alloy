//! [`InputText`] — the text a key press would insert, or an IME committed.

use core::fmt;

/// Text bound for an editable field: non-empty and free of control characters.
///
/// One type for both payloads that carry text — [`KeyEvent::text`] and
/// [`WindowEvent::TextCommitted`] (`ADR-0027`, epic #110 decision 17) — so a
/// consumer validates once at the port edge and never again. Control
/// characters (`\r`, `\x08`, `\x7f`, …) are what a press of Enter, Backspace
/// or a Ctrl chord reports as "text"; they are commands, not content, so the
/// adapter drops them instead of handing them to a keymap as if typed.
///
/// [`KeyEvent::text`]: crate::domain::key_event::KeyEvent::text
/// [`WindowEvent::TextCommitted`]: crate::domain::event::WindowEvent::TextCommitted
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InputText(String);

impl InputText {
    /// `None` when `text` is empty or holds a control character.
    #[must_use]
    pub fn new(text: impl AsRef<str>) -> Option<Self> {
        let text = text.as_ref();
        if text.is_empty() || text.chars().any(char::is_control) {
            return None;
        }
        Some(Self(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InputText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
