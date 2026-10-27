//! Why one keymap binding was rejected.

/// A binding that could not be loaded. The rest of the set is kept
/// (epic #110 decision 15).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum KeymapError {
    /// The action name is not a [`BrowserCommand`](super::BrowserCommand).
    #[error("unknown action `{0}`")]
    UnknownAction(String),
    /// The chord text is empty or names no key (`"ctrl+"`).
    #[error("chord `{0}` names no key")]
    MissingKey(String),
    /// A word before the final `+` is not a modifier.
    #[error("unknown modifier `{modifier}` in chord `{chord}`")]
    UnknownModifier { chord: String, modifier: String },
    /// The final word is neither one character nor a known key name.
    #[error("unknown key `{key}` in chord `{chord}`")]
    UnknownKey { chord: String, key: String },
}
