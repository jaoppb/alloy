use dom::DomError;
use thiserror::Error;

/// Domain error enum representing failures in CSS parsing, selector matching, or style computation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CssError {
    /// Unexpected end of file while parsing CSS.
    #[error("Unexpected end of file in CSS stream")]
    UnexpectedEof,
    /// Malformed or unsupported selector syntax.
    #[error("Invalid CSS selector: '{0}'")]
    InvalidSelector(String),
    /// Invalid CSS property name or unsupported format.
    #[error("Invalid CSS property: '{0}'")]
    InvalidProperty(String),
    /// Malformed color literal.
    #[error("Invalid CSS color: '{0}'")]
    InvalidColor(String),
    /// Underlying error from DOM tree navigation.
    #[error("DOM error during CSS matching: {0}")]
    DomError(#[from] DomError),
}
