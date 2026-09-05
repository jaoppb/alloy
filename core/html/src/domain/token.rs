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

/// A `StartTag` or `EndTag` token payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagToken {
    name: String,
    attributes: AttributeList,
    self_closing: bool,
}

impl TagToken {
    /// Create a new tag token with lowercased name.
    pub fn new(
        name: impl Into<String>,
        attributes: AttributeList,
        self_closing: bool,
    ) -> Result<Self, HtmlError> {
        let name_string = name.into().to_ascii_lowercase();
        if name_string.is_empty() {
            return Err(HtmlError::InvalidTag("tag name cannot be empty".into()));
        }
        Ok(Self {
            name: name_string,
            attributes,
            self_closing,
        })
    }

    /// The tag name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
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
