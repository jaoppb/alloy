//! [`Identifier`] — a validated CSS identifier.
//!
//! One value object serving five positions in the grammar: a type selector's
//! tag, a class name, an element id, an attribute name and a declaration's
//! property. Naming them all with the same newtype is what keeps the selector
//! and declaration types free of naked `String`s (`ADR-0010:128`), the way
//! `core/dom/src/domain/tag_name.rs` does for a DOM tag.
//!
//! Validation follows CSS Syntax Level 3 §4.3.11 *loosely but honestly*: the
//! escapes have already been resolved by the tokenizer, so what arrives here is
//! the identifier's **value**. An empty value, or one carrying a character that
//! could never appear unescaped and was not produced by an escape, is refused —
//! the caller turns that refusal into a `CssError` with a span.

use core::fmt;

/// A validated CSS identifier: a tag, class, id, attribute or property name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier {
    text: String,
}

impl Identifier {
    /// The identifier exactly as written, or `None` when `text` is empty or
    /// carries a character an identifier cannot hold.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        Self::from_owned(text.to_owned())
    }

    /// The identifier ASCII-lowercased — the form HTML tag names, attribute
    /// names, property names and at-rule keywords are matched in.
    #[must_use]
    pub fn lowercased(text: &str) -> Option<Self> {
        Self::from_owned(text.to_ascii_lowercase())
    }

    fn from_owned(text: String) -> Option<Self> {
        if text.is_empty() || text.chars().any(is_forbidden) {
            return None;
        }
        Some(Self { text })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// Whether `character` can never be part of an identifier's resolved value.
///
/// Control characters and the structural punctuation of the grammar; everything
/// else — including non-ASCII, which CSS Syntax L3 §4.2 treats as an identifier
/// code point — is accepted.
const fn is_forbidden(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            ' ' | '\t'
                | '"'
                | '\''
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | ','
                | ';'
                | ':'
                | '>'
                | '+'
                | '~'
                | '*'
                | '/'
                | '@'
                | '#'
                | '.'
                | '|'
                | '!'
                | '='
                | '%'
                | '&'
                | '?'
                | '<'
                | '$'
                | '^'
                | '`'
        )
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}
