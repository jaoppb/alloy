//! Why one keymap binding was rejected.

use super::keymap_binding::{ActionName, ChordText, ChordWord};

/// A binding that could not be loaded. The rest of the set is kept
/// (epic #110 decision 15).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum KeymapError {
    /// The action name is not a [`BrowserCommand`](super::BrowserCommand).
    #[error("unknown action `{0}`")]
    UnknownAction(ActionName),
    /// The chord text is empty or names no key (`"ctrl+"`).
    #[error("chord `{0}` names no key")]
    MissingKey(ChordText),
    /// A word before the key is not a modifier.
    #[error("unknown modifier `{modifier}` in chord `{chord}`")]
    UnknownModifier {
        chord: ChordText,
        modifier: ChordWord,
    },
    /// The key word is neither one character, a `code:` position nor a known
    /// key name.
    #[error("unknown key `{key}` in chord `{chord}`")]
    UnknownKey { chord: ChordText, key: ChordWord },
}
