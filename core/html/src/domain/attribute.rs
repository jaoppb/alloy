//! Value objects and first-class collections representing element attributes.

use crate::domain::error::InvalidAttributeName;
use crate::domain::location::SourceLocation;
use core::fmt;

/// A validated attribute name: non-empty, with no control or whitespace characters and none of
/// `" ' / = >`; lowercased on construction (ADR-0024).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttributeName(String);

impl AttributeName {
    /// Create a validated attribute name; `Err(InvalidAttributeName)` when `raw` breaks the rule.
    pub fn new(raw: &str) -> Result<Self, InvalidAttributeName> {
        let valid = !raw.is_empty() && !raw.chars().any(is_forbidden);
        if !valid {
            return Err(InvalidAttributeName::new(raw));
        }
        Ok(Self(raw.to_ascii_lowercase()))
    }

    /// The `style` attribute — one element's own inline declaration block
    /// (CSS Cascade L4 §6.4.3).
    #[must_use]
    pub fn style() -> Self {
        Self("style".to_owned())
    }

    /// The `class` attribute — a whitespace-separated set of names rather than one string
    /// (HTML §3.2.6.7).
    #[must_use]
    pub fn class() -> Self {
        Self("class".to_owned())
    }

    /// The `id` attribute a `#name` selector component is compared against.
    #[must_use]
    pub fn id() -> Self {
        Self("id".to_owned())
    }

    /// Access attribute name as string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AttributeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl PartialEq<str> for AttributeName {
    fn eq(&self, other: &str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

const fn is_forbidden(character: char) -> bool {
    character.is_ascii_control()
        || character.is_whitespace()
        || matches!(character, '"' | '\'' | '/' | '=' | '>')
}

/// An attribute value representation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AttributeValue(String);

impl AttributeValue {
    /// Create a new attribute value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Access value as string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into inner string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for AttributeValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A paired attribute name and attribute value, with where its name started in the source.
///
/// Equality ignores the location: two entries are the same attribute wherever they were written.
#[derive(Clone, Debug)]
pub struct AttributeEntry {
    name: AttributeName,
    value: AttributeValue,
    location: SourceLocation,
}

impl PartialEq for AttributeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}

impl Eq for AttributeEntry {}

impl AttributeEntry {
    /// Create an attribute entry from name, value and the location of the name's first character.
    #[must_use]
    pub const fn new(name: AttributeName, value: AttributeValue, location: SourceLocation) -> Self {
        Self {
            name,
            value,
            location,
        }
    }

    /// Where the attribute name started in the source.
    #[must_use]
    pub const fn location(&self) -> SourceLocation {
        self.location
    }

    /// The attribute name.
    #[must_use]
    pub const fn name(&self) -> &AttributeName {
        &self.name
    }

    /// The attribute value.
    #[must_use]
    pub const fn value(&self) -> &AttributeValue {
        &self.value
    }
}

/// An attribute that [`AttributeList::insert`] refused because the name was already present.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateAttribute(AttributeEntry);

impl DuplicateAttribute {
    /// The rejected (later) entry.
    #[must_use]
    pub const fn entry(&self) -> &AttributeEntry {
        &self.0
    }
}

/// A first-class collection of element attributes, unique by name, in source order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttributeList {
    entries: Vec<AttributeEntry>,
}

impl AttributeList {
    /// Creates an empty attribute list.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Inserts an entry; the first attribute of a name wins (WHATWG `duplicate-attribute`).
    ///
    /// Uniqueness is the list's invariant so no sink can drift back to last-wins.
    pub fn insert(&mut self, entry: AttributeEntry) -> Result<(), DuplicateAttribute> {
        if self.entries.iter().any(|kept| kept.name() == entry.name()) {
            return Err(DuplicateAttribute(entry));
        }
        self.entries.push(entry);
        Ok(())
    }

    /// Number of attributes in the list.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Checks if the collection has no attributes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Slice of the attribute entries.
    #[must_use]
    pub fn as_slice(&self) -> &[AttributeEntry] {
        &self.entries
    }

    /// Find an attribute value by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&AttributeValue> {
        self.entries
            .iter()
            .find(|entry| entry.name() == name)
            .map(AttributeEntry::value)
    }

    /// Find an attribute value string by name.
    #[must_use]
    pub fn get_value_str(&self, name: &str) -> Option<&str> {
        self.get(name).map(AttributeValue::as_str)
    }

    /// Iterator over entries.
    pub fn iter(&self) -> core::slice::Iter<'_, AttributeEntry> {
        self.entries.iter()
    }
}

impl<'a> IntoIterator for &'a AttributeList {
    type Item = &'a AttributeEntry;
    type IntoIter = core::slice::Iter<'a, AttributeEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_creation_and_lookup() {
        let location = SourceLocation::initial();
        let name = AttributeName::new("CLASS").expect("valid name");
        let value = AttributeValue::new("btn primary");
        let entry = AttributeEntry::new(name, value, location);

        let mut list = AttributeList::new();
        list.insert(entry).expect("first entry is unique");

        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
        assert_eq!(list.get_value_str("class"), Some("btn primary"));
        assert_eq!(list.get_value_str("nonexistent"), None);
    }

    #[test]
    fn a_duplicate_name_is_rejected_and_the_first_wins() {
        let at = SourceLocation::initial();
        let entry = |value: &str| {
            AttributeEntry::new(
                AttributeName::new("id").unwrap(),
                AttributeValue::new(value),
                at,
            )
        };
        let mut list = AttributeList::new();
        list.insert(entry("first")).unwrap();
        let rejected = list.insert(entry("second")).unwrap_err();

        assert_eq!(list.len(), 1);
        assert_eq!(list.get_value_str("id"), Some("first"));
        assert_eq!(rejected.entry().value().as_str(), "second");
    }

    #[test]
    fn entry_equality_ignores_location() {
        let name = || AttributeName::new("id").unwrap();
        let one = AttributeEntry::new(name(), AttributeValue::new("a"), SourceLocation::initial());
        let other = AttributeEntry::new(
            name(),
            AttributeValue::new("a"),
            SourceLocation::new(9, 9, 9),
        );
        assert_eq!(one, other);
        assert_eq!(other.location().line(), 9);
    }

    #[test]
    fn an_empty_name_is_rejected() {
        assert_eq!(AttributeName::new(""), Err(InvalidAttributeName::new("")));
    }

    #[test]
    fn forbidden_characters_are_rejected() {
        for raw in [
            "a b", "a\tb", "a\nb", "a\u{0}b", "a\"b", "a'b", "a/b", "a=b", "a>b",
        ] {
            assert_eq!(
                AttributeName::new(raw),
                Err(InvalidAttributeName::new(raw)),
                "{raw:?}"
            );
        }
    }

    #[test]
    fn a_valid_name_is_lowercased_and_the_shortcuts_match() {
        assert_eq!(AttributeName::new("DATA-Id").unwrap().as_str(), "data-id");
        assert_eq!(AttributeName::new("STYLE"), Ok(AttributeName::style()));
        assert_eq!(AttributeName::new("class"), Ok(AttributeName::class()));
        assert_eq!(AttributeName::new("Id"), Ok(AttributeName::id()));
    }
}
