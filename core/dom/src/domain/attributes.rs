//! [`AttributeName`] / [`AttributeValue`] value objects and [`AttributeMap`], the
//! insertion-ordered first-class collection of an element's attributes
//! (v0.2 report §2.2; `ADR-0010:132` rule 4).

use core::fmt;

use crate::domain::error::DomError;

/// A validated attribute name: non-empty, ASCII, no control or whitespace
/// characters and none of `" ' / = >`; lowercased on construction.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttributeName(String);

impl AttributeName {
    pub fn new(raw: &str) -> Result<Self, DomError> {
        let valid = !raw.is_empty() && !raw.chars().any(is_forbidden);
        if !valid {
            return Err(DomError::invalid_attribute_name(raw));
        }
        Ok(Self(raw.to_ascii_lowercase()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_forbidden(character: char) -> bool {
    character.is_ascii_control()
        || character.is_whitespace()
        || matches!(character, '"' | '\'' | '/' | '=' | '>')
}

impl fmt::Display for AttributeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// An attribute value. Any string is legal; the serializer escapes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AttributeValue(String);

impl AttributeValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An element's attributes in insertion order. A `set` on a name already present
/// overwrites in place and never reorders (v0.2 report §2.2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttributeMap {
    entries: Vec<(AttributeName, AttributeValue)>,
}

impl AttributeMap {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Insert `name`, or overwrite its value in place when already present.
    pub fn set(&mut self, name: AttributeName, value: AttributeValue) {
        match self
            .entries
            .iter_mut()
            .find(|(existing, _)| *existing == name)
        {
            Some(entry) => entry.1 = value,
            None => self.entries.push((name, value)),
        }
    }

    #[must_use]
    pub fn get(&self, name: &AttributeName) -> Option<&AttributeValue> {
        self.entries
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|entry| &entry.1)
    }

    /// Remove `name` if present; returns whether it was.
    pub fn remove(&mut self, name: &AttributeName) -> bool {
        let original_length = self.entries.len();
        self.entries.retain(|(existing, _)| existing != name);
        self.entries.len() != original_length
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AttributeName, &AttributeValue)> + '_ {
        self.entries.iter().map(|entry| (&entry.0, &entry.1))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
