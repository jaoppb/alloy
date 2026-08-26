use crate::domain::error::EngineError;
use std::fmt;

/// Invariant-protecting newtype for function, method, and variable names.
///
/// Ensures identifiers are non-empty and stripped of leading/trailing whitespace,
/// adhering to Object Calisthenics (no naked primitives in domain interfaces).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier(String);

impl Identifier {
    /// Creates a new valid identifier.
    ///
    /// # Errors
    /// Returns `EngineError::InvalidIdentifier` if the input is empty or contains only whitespace.
    pub fn new(name: impl Into<String>) -> Result<Self, EngineError> {
        let trimmed = name.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(EngineError::InvalidIdentifier(
                "Identifier cannot be empty or contain only whitespace".to_string(),
            ));
        }
        Ok(Self(trimmed))
    }

    /// Returns the string slice representation of this identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
