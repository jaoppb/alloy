//! [`TagName`] — a validated element tag: non-empty, first character an ASCII
//! letter, the rest ASCII alphanumeric or `-`, lowercased on construction
//! (v0.2 report §2.2).

use core::fmt;

use crate::domain::error::DomError;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TagName(String);

impl TagName {
    /// Validate and normalise `raw`. `Err(DomError::InvalidTagName)` when it is
    /// empty, starts with a non-letter, or contains a character other than an
    /// ASCII alphanumeric or `-`.
    pub fn new(raw: &str) -> Result<Self, DomError> {
        let valid = starts_with_letter(raw) && raw.chars().skip(1).all(is_tag_character);
        if !valid {
            return Err(DomError::invalid_tag_name(raw));
        }
        Ok(Self(raw.to_ascii_lowercase()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn starts_with_letter(raw: &str) -> bool {
    raw.chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic())
}

fn is_tag_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-'
}

impl fmt::Display for TagName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
