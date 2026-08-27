use dom::{AttributeMap, DomError, TagName};
use thiserror::Error;

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
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HtmlError {
    /// Stream ended prematurely while parsing a token.
    #[error("Unexpected end of file in HTML stream")]
    UnexpectedEof,
    /// Malformed tag structure (e.g. invalid tag name or missing closing bracket).
    #[error("Malformed HTML tag: {0}")]
    MalformedTag(String),
    /// Invalid tag name.
    #[error("Invalid HTML tag name: {0}")]
    InvalidTagName(String),
    /// Unrecognized HTML entity.
    #[error("Unrecognized HTML entity: {0}")]
    InvalidEntity(String),
    /// Underlying error from DOM arena mutations.
    #[error("DOM tree error during HTML parsing: {0}")]
    DomError(#[from] DomError),
}
