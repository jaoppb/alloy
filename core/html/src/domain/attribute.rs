//! Value objects and first-class collections representing element attributes.

use crate::domain::error::HtmlError;
use crate::domain::location::SourceLocation;
use core::fmt;

/// A normalized, validated attribute name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttributeName(String);

impl AttributeName {
    /// Create a validated attribute name.
    pub fn new(raw_name: impl Into<String>, location: SourceLocation) -> Result<Self, HtmlError> {
        let normalized = raw_name.into().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err(HtmlError::invalid_attribute(
                "attribute name cannot be empty",
                location,
            ));
        }
        Ok(Self(normalized))
    }

    /// Create an attribute name without location check (for trusted sources).
    #[must_use]
    pub fn new_unchecked(name: impl Into<String>) -> Self {
        Self(name.into().to_ascii_lowercase())
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

/// A paired attribute name and attribute value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeEntry {
    name: AttributeName,
    value: AttributeValue,
}

impl AttributeEntry {
    /// Create an attribute entry from name and value.
    #[must_use]
    pub const fn new(name: AttributeName, value: AttributeValue) -> Self {
        Self { name, value }
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

/// A first-class collection of element attributes.
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

    /// Pushes a new entry into the list.
    pub fn push(&mut self, entry: AttributeEntry) {
        self.entries.push(entry);
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
        let name = AttributeName::new("CLASS", location).expect("valid name");
        let value = AttributeValue::new("btn primary");
        let entry = AttributeEntry::new(name, value);

        let mut list = AttributeList::new();
        list.push(entry);

        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
        assert_eq!(list.get_value_str("class"), Some("btn primary"));
        assert_eq!(list.get_value_str("nonexistent"), None);
    }

    #[test]
    fn empty_attribute_name_errors() {
        let location = SourceLocation::initial();
        let error = AttributeName::new("", location).unwrap_err();
        assert!(matches!(error, HtmlError::InvalidAttribute { .. }));
    }
}
