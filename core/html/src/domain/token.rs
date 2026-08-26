use dom::{AttributeMap, DomError, TagName};
use std::fmt;

/// Tokens emitted by the HTML tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlToken {
    /// DOCTYPE declaration (e.g. `<!DOCTYPE html>`).
    Doctype(String),
    /// Start element tag `<tag ...>`, with attributes and self-closing flag.
    StartTag {
        /// Element tag name.
        name: TagName,
        /// Element attributes.
        attributes: AttributeMap,
        /// Indicates if the tag ended with `/>`.
        self_closing: bool,
    },
    /// End element tag `</tag>`.
    EndTag(TagName),
    /// Character / text data.
    Character(String),
    /// HTML comment `<!-- ... -->`.
    Comment(String),
    /// End of file / stream.
    Eof,
}

/// Errors occurring during HTML tokenization and tree construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlError {
    /// Stream ended prematurely while parsing a token.
    UnexpectedEof,
    /// Malformed tag structure (e.g. invalid tag name or missing closing bracket).
    MalformedTag(String),
    /// Invalid tag name.
    InvalidTagName(String),
    /// Underlying error from DOM arena mutations.
    DomError(DomError),
}

impl fmt::Display for HtmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "Unexpected end of file in HTML stream"),
            Self::MalformedTag(msg) => write!(f, "Malformed HTML tag: {msg}"),
            Self::InvalidTagName(msg) => write!(f, "Invalid HTML tag name: {msg}"),
            Self::DomError(err) => write!(f, "DOM tree error during HTML parsing: {err}"),
        }
    }
}

impl std::error::Error for HtmlError {}

impl From<DomError> for HtmlError {
    fn from(err: DomError) -> Self {
        Self::DomError(err)
    }
}
