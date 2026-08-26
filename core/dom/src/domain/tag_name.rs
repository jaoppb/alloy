use crate::domain::error::DomError;
use std::fmt;

/// Strongly typed element tag name (e.g. `div`, `span`, `p`).
/// Normalized to lowercase and guaranteed to be non-empty.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TagName(String);

impl TagName {
    /// Creates and validates a new `TagName`.
    ///
    /// # Errors
    /// Returns `DomError::InvalidTagName` if `name` is empty or only whitespace.
    pub fn new(name: impl Into<String>) -> Result<Self, DomError> {
        let raw = name.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomError::InvalidTagName(raw));
        }

        Ok(Self(trimmed.to_ascii_lowercase()))
    }

    /// Accesses the tag name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TagName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
