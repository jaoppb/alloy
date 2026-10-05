//! Value objects representing HTML5 tokens.

use crate::domain::attribute::AttributeList;
use crate::domain::diagnostic::ParseDiagnostic;
use crate::domain::error::HtmlError;
use crate::domain::location::SourceLocation;
use crate::domain::tag::TagName;
use crate::domain::text::Text;

/// A DOCTYPE token representation.
///
/// Equality ignores [`DoctypeToken::location`].
#[derive(Clone, Debug, Default)]
pub struct DoctypeToken {
    name: Option<String>,
    public_id: Option<String>,
    system_id: Option<String>,
    force_quirks: bool,
    location: SourceLocation,
}

impl PartialEq for DoctypeToken {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.public_id == other.public_id
            && self.system_id == other.system_id
            && self.force_quirks == other.force_quirks
    }
}

impl Eq for DoctypeToken {}

impl DoctypeToken {
    /// Create a new DOCTYPE token that started (`<`) at `location`.
    #[must_use]
    pub fn new(
        name: Option<String>,
        public_id: Option<String>,
        system_id: Option<String>,
        force_quirks: bool,
        location: SourceLocation,
    ) -> Self {
        Self {
            name: name.map(|raw| raw.to_ascii_lowercase()),
            public_id,
            system_id,
            force_quirks,
            location,
        }
    }

    /// Where the declaration started in the source.
    #[must_use]
    pub const fn location(&self) -> SourceLocation {
        self.location
    }

    /// The DOCTYPE root name, lowercased (e.g. `"html"`).
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The PUBLIC identifier if present.
    #[must_use]
    pub fn public_id(&self) -> Option<&str> {
        self.public_id.as_deref()
    }

    /// The SYSTEM identifier if present.
    #[must_use]
    pub fn system_id(&self) -> Option<&str> {
        self.system_id.as_deref()
    }

    /// Whether quirks mode is forced.
    #[must_use]
    pub const fn force_quirks(&self) -> bool {
        self.force_quirks
    }
}

/// A `StartTag` or `EndTag` token payload.
///
/// Equality ignores [`TagToken::location`].
#[derive(Clone, Debug)]
pub struct TagToken {
    name: TagName,
    attributes: AttributeList,
    self_closing: bool,
    location: SourceLocation,
}

impl PartialEq for TagToken {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.attributes == other.attributes
            && self.self_closing == other.self_closing
    }
}

impl Eq for TagToken {}

impl TagToken {
    /// Create a new tag token with a validated tag name that started (`<`) at `location`.
    #[must_use]
    pub const fn new(
        name: TagName,
        attributes: AttributeList,
        self_closing: bool,
        location: SourceLocation,
    ) -> Self {
        Self {
            name,
            attributes,
            self_closing,
            location,
        }
    }

    /// Where the tag started in the source.
    #[must_use]
    pub const fn location(&self) -> SourceLocation {
        self.location
    }

    /// Parse and validate a tag token from a raw tag name.
    pub fn parse(
        name: &str,
        location: SourceLocation,
        attributes: AttributeList,
        self_closing: bool,
    ) -> Result<Self, HtmlError> {
        let tag = TagName::new(name, location)?;
        Ok(Self::new(tag, attributes, self_closing, location))
    }

    /// The tag name VO.
    #[must_use]
    pub const fn tag_name(&self) -> &TagName {
        &self.name
    }

    /// The strongly-typed tag name.
    #[must_use]
    pub const fn tag(&self) -> &TagName {
        &self.name
    }

    /// The tag name as string slice.
    #[must_use]
    pub const fn name(&self) -> &str {
        self.name.as_str()
    }

    /// The collection of attributes.
    #[must_use]
    pub const fn attributes(&self) -> &AttributeList {
        &self.attributes
    }

    /// Mutable reference to attributes.
    pub const fn attributes_mut(&mut self) -> &mut AttributeList {
        &mut self.attributes
    }

    /// Whether the tag had a self-closing slash (`/>`).
    #[must_use]
    pub const fn is_self_closing(&self) -> bool {
        self.self_closing
    }

    /// Set self-closing status.
    pub const fn set_self_closing(&mut self, self_closing: bool) {
        self.self_closing = self_closing;
    }
}

/// A discriminated HTML5 token emitted by the tokenizer.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// DOCTYPE declaration.
    Doctype(DoctypeToken),
    /// Opening tag.
    StartTag(TagToken),
    /// Closing tag.
    EndTag(TagToken),
    /// Sequence of character data.
    Character(Text),
    /// Comment data.
    Comment(Text),
    /// A recoverable parse error, delivered ahead of the token that triggered it (ADR-0023).
    ParseError(ParseDiagnostic),
    /// End of stream.
    EndOfFile,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::attribute::{AttributeEntry, AttributeName, AttributeValue};

    fn attribute(name: &str, value: &str) -> AttributeEntry {
        AttributeEntry::new(
            AttributeName::new_unchecked(name),
            AttributeValue::new(value),
            SourceLocation::initial(),
        )
    }

    #[test]
    fn the_attribute_list_keeps_insertion_order_and_looks_up_by_name() {
        let mut list = AttributeList::new();
        assert!(list.is_empty());
        list.insert(attribute("id", "a")).unwrap();
        list.insert(attribute("class", "b")).unwrap();

        assert_eq!(list.len(), 2);
        assert_eq!(list.get_value_str("class"), Some("b"));
        assert_eq!(list.get_value_str("missing"), None);
        let names: Vec<&str> = list.iter().map(|entry| entry.name().as_str()).collect();
        assert_eq!(names, ["id", "class"]);
    }

    #[test]
    fn a_doctype_lowercases_its_name_and_exposes_every_field() {
        let doctype = DoctypeToken::new(
            Some("HTML".into()),
            Some("pub".into()),
            Some("sys".into()),
            true,
            SourceLocation::initial(),
        );
        assert_eq!(doctype.name(), Some("html"));
        assert_eq!(doctype.public_id(), Some("pub"));
        assert_eq!(doctype.system_id(), Some("sys"));
        assert!(doctype.force_quirks());

        let empty = DoctypeToken::default();
        assert_eq!(empty.name(), None);
        assert!(!empty.force_quirks());
    }

    #[test]
    fn a_tag_token_validates_its_name_and_tracks_self_closing() {
        let location = SourceLocation::initial();
        let mut token =
            TagToken::parse("BR", location, AttributeList::new(), false).expect("valid tag");
        assert_eq!(token.tag(), &TagName::Br);
        assert_eq!(token.name(), "br");
        assert!(!token.is_self_closing());

        token.set_self_closing(true);
        token.attributes_mut().insert(attribute("id", "x")).unwrap();
        assert!(token.is_self_closing());
        assert_eq!(token.attributes().get_value_str("id"), Some("x"));

        assert_eq!(
            TagToken::parse("1x", location, AttributeList::new(), false),
            Err(HtmlError::invalid_tag("1x", location))
        );
    }

    #[test]
    fn token_equality_ignores_location() {
        let tag = |at| TagToken::new(TagName::P, AttributeList::new(), false, at);
        assert_eq!(
            tag(SourceLocation::initial()),
            tag(SourceLocation::new(4, 4, 40))
        );
        assert_eq!(tag(SourceLocation::new(4, 4, 40)).location().line(), 4);
        let doctype = |at| DoctypeToken::new(None, None, None, false, at);
        assert_eq!(
            doctype(SourceLocation::initial()),
            doctype(SourceLocation::new(2, 1, 7))
        );
    }

    #[test]
    fn a_typed_tag_token_is_built_without_re_validation() {
        let token = TagToken::new(
            TagName::P,
            AttributeList::new(),
            false,
            SourceLocation::initial(),
        );
        assert_eq!(token.name(), "p");
    }

    #[test]
    fn character_and_comment_tokens_carry_their_text() {
        assert!(matches!(
            Token::Character(Text::new("hello")),
            Token::Character(_)
        ));
        assert!(matches!(
            Token::Comment(Text::new("note")),
            Token::Comment(_)
        ));
        assert_eq!(Token::EndOfFile, Token::EndOfFile);
    }
}
