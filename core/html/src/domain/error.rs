//! Domain error types for HTML parsing and tree construction.

use crate::domain::location::SourceLocation;
use core::fmt;

/// Errors arising during HTML tokenization, validation, or DOM construction.
#[non_exhaustive]
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum HtmlError {
    /// A syntax error in HTML input at a specific source location.
    #[error("Parse syntax error at {location}: {message}")]
    ParseError {
        /// Diagnostic message describing the syntax error.
        message: String,
        /// Source location where error occurred.
        location: SourceLocation,
    },

    /// Unexpected end of file encountered in tokenizer.
    #[error("Unexpected end of input at {location} in state {state}")]
    UnexpectedEndOfInput {
        /// The tokenizer state name when EOF occurred.
        state: &'static str,
        /// Source location where unexpected EOF occurred.
        location: SourceLocation,
    },

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
    /// Create a parse error with a descriptive message and source location.
    #[must_use]
    pub fn parse(message: impl fmt::Display, location: SourceLocation) -> Self {
        Self::ParseError {
            message: message.to_string(),
            location,
        }
    }

    /// Create an unexpected EOF error.
    #[must_use]
    pub const fn unexpected_eof(state: &'static str, location: SourceLocation) -> Self {
        Self::UnexpectedEndOfInput { state, location }
    }

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
    fn parse_error_formats_with_location() {
        let location = SourceLocation::new(2, 5, 20);
        let error = HtmlError::parse("expected closing tag", location);
        assert!(error.to_string().contains("2:5:offset 20"));
        assert!(error.to_string().contains("expected closing tag"));
    }

    #[test]
    fn unexpected_eof_formats_with_state() {
        let location = SourceLocation::new(1, 10, 9);
        let error = HtmlError::unexpected_eof("TagOpen", location);
        assert!(error.to_string().contains("TagOpen"));
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_names_its_cause() {
        assert_eq!(
            HtmlError::parse("stray <").to_string(),
            "Parse syntax error: stray <"
        );
        assert_eq!(
            HtmlError::UnexpectedEof { state: "Data" }.to_string(),
            "Unexpected end of input in state Data"
        );
        assert_eq!(
            HtmlError::InvalidTag("1x".into()).to_string(),
            "Invalid tag name: 1x"
        );
        assert_eq!(
            HtmlError::InvalidAttribute("a b".into()).to_string(),
            "Invalid attribute: a b"
        );
    }
}
