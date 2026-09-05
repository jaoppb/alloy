//! Value objects representing HTML5 tokens.

use crate::domain::attribute::AttributeList;
use crate::domain::tag_name::TagName;
use crate::domain::text::Text;

/// A DOCTYPE token representation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoctypeToken {
    name: Option<TagName>,
    public_id: Option<String>,
    system_id: Option<String>,
    force_quirks: bool,
}

impl DoctypeToken {
    /// Create a new DOCTYPE token.
    #[must_use]
    pub const fn new(
        name: Option<TagName>,
        public_id: Option<String>,
        system_id: Option<String>,
        force_quirks: bool,
    ) -> Self {
        Self {
            name,
            public_id,
            system_id,
            force_quirks,
        }
    }

    /// The DOCTYPE root name (e.g. `"html"`).
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(TagName::as_str)
    }

    /// The tag name VO if present.
    #[must_use]
    pub const fn tag_name(&self) -> Option<&TagName> {
        self.name.as_ref()
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagToken {
    name: TagName,
    attributes: AttributeList,
    self_closing: bool,
}

impl TagToken {
    /// Create a new tag token with a validated tag name.
    #[must_use]
    pub const fn new(name: TagName, attributes: AttributeList, self_closing: bool) -> Self {
        Self {
            name,
            attributes,
            self_closing,
        }
    }

    /// The tag name VO.
    #[must_use]
    pub const fn tag_name(&self) -> &TagName {
        &self.name
    }

    /// The tag name as string slice.
    #[must_use]
    pub fn name(&self) -> &str {
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
    /// End of stream.
    EndOfFile,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctype_token_accessors() {
        let doctype = DoctypeToken::new(
            Some(TagName::new_unchecked("html")),
            Some("-//W3C//DTD HTML 4.01//EN".into()),
            None,
            false,
        );
        assert_eq!(doctype.name(), Some("html"));
        assert_eq!(doctype.public_id(), Some("-//W3C//DTD HTML 4.01//EN"));
        assert_eq!(doctype.system_id(), None);
        assert!(!doctype.force_quirks());
    }

    #[test]
    fn tag_token_accessors() {
        let mut tag = TagToken::new(TagName::new_unchecked("div"), AttributeList::new(), false);
        assert_eq!(tag.name(), "div");
        assert!(!tag.is_self_closing());
        tag.set_self_closing(true);
        assert!(tag.is_self_closing());
    }

    #[test]
    fn token_variants() {
        let text_token = Token::Character(Text::new("hello"));
        assert!(matches!(text_token, Token::Character(_)));

        let comment_token = Token::Comment(Text::new("note"));
        assert!(matches!(comment_token, Token::Comment(_)));

        let eof = Token::EndOfFile;
        assert_eq!(eof, Token::EndOfFile);
    }
}
