use dom::DomError;
use std::fmt;

/// Domain error enum representing failures in CSS parsing, selector matching, or style computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssError {
    /// Unexpected end of file while parsing CSS.
    UnexpectedEof,
    /// Malformed or unsupported selector syntax.
    InvalidSelector(String),
    /// Invalid CSS property name or unsupported format.
    InvalidProperty(String),
    /// Malformed color literal.
    InvalidColor(String),
    /// Underlying error from DOM tree navigation.
    DomError(DomError),
}

impl fmt::Display for CssError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "Unexpected end of file in CSS stream"),
            Self::InvalidSelector(s) => write!(f, "Invalid CSS selector: '{s}'"),
            Self::InvalidProperty(p) => write!(f, "Invalid CSS property: '{p}'"),
            Self::InvalidColor(c) => write!(f, "Invalid CSS color: '{c}'"),
            Self::DomError(e) => write!(f, "DOM error during CSS matching: {e}"),
        }
    }
}

impl std::error::Error for CssError {}

impl From<DomError> for CssError {
    fn from(err: DomError) -> Self {
        Self::DomError(err)
    }
}
