//! [`KeyEvent`] — one physical key press, with everything about it.

use core::fmt;

use crate::domain::key::KeyCode;
use crate::domain::logical_key::LogicalKey;
use crate::domain::modifiers::Modifiers;
use crate::domain::text::InputText;

/// Where in its life a key press is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeyState {
    /// The key went down. A held key yields exactly one `Pressed`.
    Pressed,
    /// The key is still down and the platform's key-repeat fired again.
    Repeated,
    Released,
}

impl KeyState {
    /// `true` only for the initial press — not for an auto-repeat.
    #[must_use]
    pub const fn is_pressed(self) -> bool {
        matches!(self, Self::Pressed)
    }

    #[must_use]
    pub const fn is_released(self) -> bool {
        matches!(self, Self::Released)
    }
}

impl fmt::Display for KeyState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Pressed => "pressed",
            Self::Repeated => "repeated",
            Self::Released => "released",
        };
        formatter.write_str(name)
    }
}

/// One physical key press: its meaning, its position, the text it would
/// insert and the modifiers held — in a single event (`ADR-0027`).
///
/// The adapter never decides whether a press is a shortcut or typing; it
/// reports everything and the consumer's keymap decides. In particular
/// [`Self::text`] is what the press would insert **ignoring Ctrl** (`winit`
/// semantics on Linux): Ctrl+L carries `"l"`. A consumer that treats text as
/// typing must check [`Self::modifiers`] first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    logical: LogicalKey,
    physical: KeyCode,
    text: Option<InputText>,
    modifiers: Modifiers,
    state: KeyState,
}

impl KeyEvent {
    /// A press with no text and no modifiers held; add them with
    /// [`Self::with_text`] / [`Self::with_modifiers`].
    #[must_use]
    pub const fn new(logical: LogicalKey, physical: KeyCode, state: KeyState) -> Self {
        Self {
            logical,
            physical,
            text: None,
            modifiers: Modifiers::NONE,
            state,
        }
    }

    #[must_use]
    pub fn with_text(mut self, text: InputText) -> Self {
        self.text = Some(text);
        self
    }

    #[must_use]
    pub const fn with_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// The key's meaning under the active layout.
    #[must_use]
    pub const fn logical(&self) -> &LogicalKey {
        &self.logical
    }

    /// The key's layout-independent position — the fallback for chords.
    #[must_use]
    pub const fn physical(&self) -> KeyCode {
        self.physical
    }

    /// What the press would insert (dead keys composed, Shift applied, Ctrl
    /// ignored), or `None` for a press that produces no printable text.
    #[must_use]
    pub const fn text(&self) -> Option<&InputText> {
        self.text.as_ref()
    }

    /// The modifiers held when this press happened. A modifier key's own
    /// events follow the DOM convention: its press reports it held, its
    /// release reports it released.
    #[must_use]
    pub const fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    #[must_use]
    pub const fn state(&self) -> KeyState {
        self.state
    }
}
