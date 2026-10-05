//! Fatal domain errors for HTML parsing and tree construction.
//!
//! Recoverable malformations are not errors: they are [`crate::ParseDiagnostic`]s delivered through
//! the port (ADR-0023). `HtmlError` aborts a parse, so it is reserved for sink/adapter failure and
//! value-object validation.

use crate::domain::location::SourceLocation;
use core::fmt;

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

    /// An invalid attribute was encountered.
    #[error("Invalid attribute '{name}' at {location}")]
    InvalidAttribute {
        /// The invalid attribute name.
        name: String,
        /// Source location where invalid attribute was found.
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

    /// Create an invalid attribute error.
    #[must_use]
    pub fn invalid_attribute(name: impl Into<String>, location: SourceLocation) -> Self {
        Self::InvalidAttribute {
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
    fn invalid_tag_and_attribute_constructors() {
        let location = SourceLocation::initial();
        let tag_error = HtmlError::invalid_tag("123", location);
        assert!(tag_error.to_string().contains("123"));

        let attr_error = HtmlError::invalid_attribute("", location);
        assert!(attr_error.to_string().contains("Invalid attribute"));
    }

    #[test]
    fn tree_construction_error_formats() {
        let error = HtmlError::tree_construction("capacity exceeded");
        assert_eq!(
            error.to_string(),
            "Tree construction error: capacity exceeded"
        );
    }

    #[cfg(feature = "dom")]
    #[test]
    fn dom_error_converts_into_tree_construction() {
        let dom_error = dom::DomError::InvalidAttributeName("bad".to_string());
        let html_error: HtmlError = dom_error.into();
        assert!(html_error.to_string().contains("Tree construction error"));
    }
}
