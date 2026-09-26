//! Value objects representing HTML5 tokens.

use crate::domain::error::HtmlError;

/// An attribute entry belonging to a tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeEntry {
    name: String,
    value: String,
}

impl AttributeEntry {
    /// Create a validated, lowercased attribute entry.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Result<Self, HtmlError> {
        let name_string = name.into().to_ascii_lowercase();
        if name_string.is_empty() {
            return Err(HtmlError::InvalidAttribute(
                "attribute name cannot be empty".into(),
            ));
        }
        Ok(Self {
            name: name_string,
            value: value.into(),
        })
    }

    /// The attribute name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The attribute value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// A first-class collection of element attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttributeList {
    entries: Vec<AttributeEntry>,
}

impl AttributeList {
    /// Create an empty attribute collection.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Push an entry to the collection.
    pub fn push(&mut self, entry: AttributeEntry) {
        self.entries.push(entry);
    }

    /// Number of attributes in the collection.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Checks if the collection is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterator over the attribute entries.
    pub fn iter(&self) -> core::slice::Iter<'_, AttributeEntry> {
        self.entries.iter()
    }
    /// Slice of the attribute entries.
    #[must_use]
    pub fn as_slice(&self) -> &[AttributeEntry] {
        &self.entries
    }

    /// Find an attribute value by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.name() == name)
            .map(AttributeEntry::value)
    }
}

impl<'a> IntoIterator for &'a AttributeList {
    type Item = &'a AttributeEntry;
    type IntoIter = core::slice::Iter<'a, AttributeEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A DOCTYPE token representation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoctypeToken {
    name: Option<String>,
    public_id: Option<String>,
    system_id: Option<String>,
    force_quirks: bool,
}

impl DoctypeToken {
    /// Create a new DOCTYPE token.
    #[must_use]
    pub fn new(
        name: Option<String>,
        public_id: Option<String>,
        system_id: Option<String>,
        force_quirks: bool,
    ) -> Self {
        Self {
            name: name.map(|s| s.to_ascii_lowercase()),
            public_id,
            system_id,
            force_quirks,
        }
    }

    /// The DOCTYPE root name (e.g. `"html"`).
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

use crate::domain::tag::TagName;

/// A `StartTag` or `EndTag` token payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagToken {
    tag: TagName,
    attributes: AttributeList,
    self_closing: bool,
}

impl TagToken {
    /// Create a new tag token with a strongly-typed [`TagName`].
    #[must_use]
    pub const fn new(tag: TagName, attributes: AttributeList, self_closing: bool) -> Self {
        Self {
            tag,
            attributes,
            self_closing,
        }
    }

    /// Parse and validate a tag token from a raw tag name.
    pub fn parse(
        name: &str,
        attributes: AttributeList,
        self_closing: bool,
    ) -> Result<Self, HtmlError> {
        let tag = TagName::new(name)?;
        Ok(Self {
            tag,
            attributes,
            self_closing,
        })
    }

    /// The strongly-typed tag name.
    #[must_use]
    pub const fn tag(&self) -> &TagName {
        &self.tag
    }

    /// The tag name as a string slice.
    #[must_use]
    pub const fn name(&self) -> &str {
        self.tag.as_str()
    }

    /// The collection of attributes.
    #[must_use]
    pub const fn attributes(&self) -> &AttributeList {
        &self.attributes
    }

    /// Mutable reference to attributes for building tokens.
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
    Character(String),
    /// Comment data.
    Comment(String),
    /// End of stream.
    EndOfFile,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attribute(name: &str, value: &str) -> AttributeEntry {
        AttributeEntry::new(name, value).expect("valid attribute")
    }

    #[test]
    fn an_attribute_name_is_lowercased_and_never_empty() {
        let entry = attribute("HREF", "/x");
        assert_eq!((entry.name(), entry.value()), ("href", "/x"));
        assert_eq!(
            AttributeEntry::new("", "v"),
            Err(HtmlError::InvalidAttribute(
                "attribute name cannot be empty".into()
            ))
        );
    }

    #[test]
    fn the_attribute_list_keeps_insertion_order_and_looks_up_by_name() {
        let mut list = AttributeList::new();
        assert!(list.is_empty());
        list.push(attribute("id", "a"));
        list.push(attribute("class", "b"));

        assert_eq!(list.len(), 2);
        assert_eq!(list.get("class"), Some("b"));
        assert_eq!(list.get("missing"), None);
        let names: Vec<&str> = list.iter().map(AttributeEntry::name).collect();
        assert_eq!(names, ["id", "class"]);
        assert_eq!(list.as_slice().len(), 2);
        assert_eq!((&list).into_iter().count(), 2);
    }

    #[test]
    fn a_doctype_lowercases_its_name_and_exposes_every_field() {
        let doctype = DoctypeToken::new(
            Some("HTML".into()),
            Some("pub".into()),
            Some("sys".into()),
            true,
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
        let mut token = TagToken::parse("BR", AttributeList::new(), false).expect("valid tag");
        assert_eq!(token.tag(), &TagName::Br);
        assert_eq!(token.name(), "br");
        assert!(!token.is_self_closing());

        token.set_self_closing(true);
        token.attributes_mut().push(attribute("id", "x"));
        assert!(token.is_self_closing());
        assert_eq!(token.attributes().get("id"), Some("x"));

        assert_eq!(
            TagToken::parse("1x", AttributeList::new(), false),
            Err(HtmlError::InvalidTag("1x".into()))
        );
    }

    #[test]
    fn a_typed_tag_token_is_built_without_re_validation() {
        let token = TagToken::new(TagName::P, AttributeList::new(), false);
        assert_eq!(token.name(), "p");
    }
}
