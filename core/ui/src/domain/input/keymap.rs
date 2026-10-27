//! [`Keymap`] — chord → command bindings, as data.

// One allow for the file: `BrowserCommand` has a single variant today, so the
// map's values are zero-sized; it gains more with #115 and #50.
#![allow(clippy::zero_sized_map_values)]

use std::collections::HashMap;

use window::KeyEvent;

use super::{BrowserCommand, KeyChord, KeymapBinding, KeymapError};

/// What Alloy binds out of the box, in the same text a script supplies: by
/// logical key, and by position for layouts whose logical key is not Latin.
/// No Ctrl+Alt chord, so Windows' left Ctrl+Alt (`AltGr`) still types.
const DEFAULT_BINDINGS: [(&str, &str); 6] = [
    ("ctrl+l", "focus_omnibox"),
    ("super+l", "focus_omnibox"),
    ("alt+d", "focus_omnibox"),
    ("ctrl+code:l", "focus_omnibox"),
    ("super+code:l", "focus_omnibox"),
    ("alt+code:d", "focus_omnibox"),
];

/// Chord → [`BrowserCommand`]. Resolved in Rust on every key press; a script
/// only ever supplies the bindings once, at init or reload (`ADR-0027`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keymap {
    bindings: HashMap<KeyChord, BrowserCommand>,
}

impl Keymap {
    /// Builds a keymap from unvalidated bindings, and returns why each
    /// rejected binding was left out. A malformed chord or an unknown action
    /// rejects that binding alone; a later binding of the same chord replaces
    /// an earlier one. When no binding is valid the keymap is
    /// [`Keymap::default`], so a bad script never leaves the browser without
    /// shortcuts (`ADR-0004`).
    #[must_use]
    pub fn from_bindings(
        bindings: impl IntoIterator<Item = KeymapBinding>,
    ) -> (Self, Vec<KeymapError>) {
        let (keymap, rejected) = Self::parse_all(bindings);
        if keymap.bindings.is_empty() {
            return (Self::default(), rejected);
        }
        (keymap, rejected)
    }

    /// The command bound to `event`'s chord: the logical key is tried first,
    /// then the physical key (non-Latin layouts).
    #[must_use]
    pub fn resolve(&self, event: &KeyEvent) -> Option<BrowserCommand> {
        KeyChord::candidates(event)
            .iter()
            .find_map(|chord| self.bindings.get(chord).copied())
    }

    fn parse_all(bindings: impl IntoIterator<Item = KeymapBinding>) -> (Self, Vec<KeymapError>) {
        let mut keymap = Self {
            bindings: HashMap::default(),
        };
        let mut rejected = Vec::new();
        for binding in bindings {
            match Self::parse(&binding) {
                Ok((chord, command)) => {
                    keymap.bindings.insert(chord, command);
                }
                Err(error) => rejected.push(error),
            }
        }
        (keymap, rejected)
    }

    fn parse(binding: &KeymapBinding) -> Result<(KeyChord, BrowserCommand), KeymapError> {
        Ok((
            binding.chord().as_str().parse()?,
            binding.action().as_str().parse()?,
        ))
    }
}

impl Default for Keymap {
    fn default() -> Self {
        let defaults = DEFAULT_BINDINGS
            .iter()
            .map(|(chord, action)| KeymapBinding::new(chord, action));
        Self::parse_all(defaults).0
    }
}
