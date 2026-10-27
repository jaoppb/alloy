//! [`Keymap`] — chord → command bindings, as data.

use std::collections::HashMap;

use window::{InputText, KeyCode, KeyEvent, Modifiers};

use super::{BrowserCommand, ChordKey, KeyChord, KeymapError};

/// One binding as a script or config hands it over: unvalidated text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapBinding {
    chord: String,
    action: String,
}

impl KeymapBinding {
    #[must_use]
    pub fn new(chord: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            chord: chord.into(),
            action: action.into(),
        }
    }

    fn parse(&self) -> Result<(KeyChord, BrowserCommand), KeymapError> {
        Ok((self.chord.parse()?, self.action.parse()?))
    }
}

/// The result of [`Keymap::from_bindings`]: the keymap to use, and why each
/// rejected binding was left out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapLoad {
    keymap: Keymap,
    rejected: Vec<KeymapError>,
}

impl KeymapLoad {
    #[must_use]
    pub const fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    #[must_use]
    pub fn rejected(&self) -> &[KeymapError] {
        &self.rejected
    }

    #[must_use]
    pub fn into_keymap(self) -> Keymap {
        self.keymap
    }
}

/// Chord → [`BrowserCommand`]. Resolved in Rust on every key press; a script
/// only ever supplies the bindings once, at init or reload (`ADR-0027`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(
    clippy::zero_sized_map_values,
    reason = "BrowserCommand has one variant today; it gains more with #115 and #50"
)]
pub struct Keymap {
    bindings: HashMap<KeyChord, BrowserCommand>,
}

impl Keymap {
    /// Builds a keymap from unvalidated bindings. A malformed chord or an
    /// unknown action rejects that binding alone and is reported; a later
    /// binding of the same chord replaces an earlier one. When no binding is
    /// valid the result is [`Keymap::default`], so a bad script never leaves
    /// the browser without shortcuts (`ADR-0004`).
    #[must_use]
    pub fn from_bindings(bindings: impl IntoIterator<Item = KeymapBinding>) -> KeymapLoad {
        let mut keymap = Self::empty();
        let mut rejected = Vec::new();
        for binding in bindings {
            match binding.parse() {
                Ok((chord, command)) => keymap.bind(chord, command),
                Err(error) => rejected.push(error),
            }
        }
        if keymap.bindings.is_empty() {
            keymap = Self::default();
        }
        KeymapLoad { keymap, rejected }
    }

    /// The command bound to `event`'s chord: the logical key is tried first,
    /// then the physical key (non-Latin layouts).
    #[must_use]
    pub fn resolve(&self, event: &KeyEvent) -> Option<BrowserCommand> {
        KeyChord::candidates(event)
            .iter()
            .find_map(|chord| self.bindings.get(chord).copied())
    }

    #[allow(
        clippy::zero_sized_map_values,
        reason = "BrowserCommand has one variant today; it gains more with #115 and #50"
    )]
    fn empty() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    fn bind(&mut self, chord: KeyChord, command: BrowserCommand) {
        self.bindings.insert(chord, command);
    }

    fn bind_letter(&mut self, modifiers: Modifiers, letter: &str, code: KeyCode) {
        let command = BrowserCommand::FocusOmnibox;
        if let Some(text) = InputText::new(letter) {
            self.bindings
                .insert(KeyChord::letter(modifiers, &text), command);
        }
        self.bindings
            .insert(KeyChord::new(modifiers, ChordKey::Physical(code)), command);
    }
}

impl Default for Keymap {
    /// Ctrl+L, Super+L (Cmd+L on macOS) and Alt+D focus the address bar, by
    /// logical key and by position. No Ctrl+Alt chord, so Windows' left
    /// Ctrl+Alt (`AltGr`) still types.
    fn default() -> Self {
        let mut keymap = Self::empty();
        keymap.bind_letter(Modifiers::NONE.with_control(), "l", KeyCode::KEY_L);
        keymap.bind_letter(Modifiers::NONE.with_super_key(), "l", KeyCode::KEY_L);
        keymap.bind_letter(Modifiers::NONE.with_alt(), "d", KeyCode::KEY_D);
        keymap
    }
}
