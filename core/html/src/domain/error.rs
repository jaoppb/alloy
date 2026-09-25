//! Domain error types for HTML parsing and tree construction.

use core::fmt;

/// Errors arising during HTML tokenization, validation, or DOM construction.
#[non_exhaustive]
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HtmlError {
    /// A syntax error in HTML input.
    #[error("Parse syntax error: {0}")]
    ParseError(String),

    /// Unexpected end of file encountered in tokenizer.
    #[error("Unexpected end of input in state {state}")]
    UnexpectedEof {
        /// The tokenizer state name when EOF occurred.
        state: &'static str,
    },

    /// An invalid tag name was encountered.
    #[error("Invalid tag name: {0}")]
    InvalidTag(String),

    /// An invalid attribute was encountered.
    #[error("Invalid attribute: {0}")]
    InvalidAttribute(String),

    /// Underlying DOM tree error.
    #[error("DOM construction error: {0}")]
    DomError(#[from] dom::DomError),
}

impl HtmlError {
    /// Create a parse error with a descriptive message.
    #[must_use]
    pub fn parse(message: impl fmt::Display) -> Self {
        Self::ParseError(message.to_string())
    }
}
