//! [`KeymapBinding`] — one unvalidated binding, and the texts it is made of.

use core::fmt;

/// A chord as written by a script or config (`"ctrl+l"`), not yet parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordText(String);

impl ChordText {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ChordText {
    fn from(text: &str) -> Self {
        Self(text.to_owned())
    }
}

impl fmt::Display for ChordText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// An action name as written by a script or config (`"focus_omnibox"`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionName(String);

impl ActionName {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ActionName {
    fn from(name: &str) -> Self {
        Self(name.to_owned())
    }
}

impl fmt::Display for ActionName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One binding as a script or config hands it over: unvalidated text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapBinding {
    chord: ChordText,
    action: ActionName,
}

impl KeymapBinding {
    #[must_use]
    pub fn new(chord: &str, action: &str) -> Self {
        Self {
            chord: ChordText::from(chord),
            action: ActionName::from(action),
        }
    }

    pub(super) const fn chord(&self) -> &ChordText {
        &self.chord
    }

    pub(super) const fn action(&self) -> &ActionName {
        &self.action
    }
}
