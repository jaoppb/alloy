//! [`AttributeName`] / [`AttributeValue`] value objects and [`AttributeMap`], the
//! first-class collection of an element's attributes (v0.2 report §2.2; `ADR-0010:132` rule 4).

use core::fmt;
use std::collections::BTreeMap;

use crate::domain::error::DomError;

/// A validated attribute name: non-empty, ASCII, no control or whitespace
/// characters and none of `" ' / = >`; lowercased on construction.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

const fn is_forbidden(character: char) -> bool {
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

/// An element's attributes backed by a [`BTreeMap`] for fast lookups and
/// deterministic, alphabetically sorted serialization output (v0.2 report §2.2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttributeMap {
    entries: BTreeMap<AttributeName, AttributeValue>,
}

impl AttributeMap {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Insert `name`, or overwrite its value when already present.
    pub fn set(&mut self, name: AttributeName, value: AttributeValue) {
        self.entries.insert(name, value);
    }

    #[must_use]
    pub fn get(&self, name: &AttributeName) -> Option<&AttributeValue> {
        self.entries.get(name)
    }

    /// Remove `name` if present; returns whether it was.
    pub fn remove(&mut self, name: &AttributeName) -> bool {
        self.entries.remove(name).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AttributeName, &AttributeValue)> + '_ {
        self.entries.iter()
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
