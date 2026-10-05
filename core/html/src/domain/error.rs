//! Fatal domain errors for HTML parsing and tree construction.
//!
//! Recoverable malformations are not errors: they are [`crate::ParseDiagnostic`]s delivered through
//! the port (ADR-0023). `HtmlError` aborts a parse, so it is reserved for sink/adapter failure and
//! value-object validation.

use crate::domain::location::SourceLocation;
use core::fmt;

/// A string that is not a valid tag name (see [`crate::TagName::new`]).
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
#[error("not a valid tag name: {0:?}")]
pub struct InvalidTagName(String);

impl InvalidTagName {
    /// Records the rejected `raw` name.
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// The rejected name, as written.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.0
    }
}

/// A string that is not a valid attribute name (see [`crate::AttributeName::new`]).
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
#[error("not a valid attribute name: {0:?}")]
pub struct InvalidAttributeName(String);

impl InvalidAttributeName {
    /// Records the rejected `raw` name.
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// The rejected name, as written.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.0
    }
}

/// Fatal errors arising from value-object validation or a tree sink.
#[non_exhaustive]
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum HtmlError {
    /// An invalid tag name was encountered.
    #[error("Invalid tag name '{name}' at {location}")]
    InvalidTag {
        /// The invalid tag name.
        name: String,
        /// Source location where invalid tag was found.
        location: SourceLocation,
    },

    /// Tree construction or adapter failure.
    #[error("Tree construction error: {message}")]
    TreeConstruction {
        /// Description of the tree sink error.
        message: String,
    },
}

impl HtmlError {
    /// Create an invalid tag error.
    #[must_use]
    pub fn invalid_tag(name: impl Into<String>, location: SourceLocation) -> Self {
        Self::InvalidTag {
            name: name.into(),
            location,
        }
    }

    /// Create a tree construction error.
    #[must_use]
    pub fn tree_construction(message: impl fmt::Display) -> Self {
        Self::TreeConstruction {
            message: message.to_string(),
        }
    }
}

#[cfg(feature = "dom")]
impl From<dom::DomError> for HtmlError {
    fn from(error: dom::DomError) -> Self {
        Self::TreeConstruction {
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_tag_constructor_and_value_object_errors() {
        let tag_error = HtmlError::invalid_tag("123", SourceLocation::initial());
        assert!(tag_error.to_string().contains("123"));

        assert_eq!(InvalidTagName::new("1x").raw(), "1x");
        assert!(
            InvalidAttributeName::new("a\"b")
                .to_string()
                .contains("a\\\"b")
        );
    }

    #[test]
    fn tree_construction_error_formats() {
        let error = HtmlError::tree_construction("capacity exceeded");
        assert_eq!(
            error.to_string(),
            "Tree construction error: capacity exceeded"
        );
    }
}
