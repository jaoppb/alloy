use std::collections::HashMap;
use std::fmt;

/// Strongly typed attribute name (e.g. `class`, `id`, `href`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttributeName(String);

impl AttributeName {
    /// Creates a new `AttributeName`.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into().trim().to_ascii_lowercase())
    }

    /// Accesses the name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AttributeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Strongly typed attribute value (e.g. `container`, `https://example.com`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttributeValue(String);

impl AttributeValue {
    /// Creates a new `AttributeValue`.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Accesses the value as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// First-class collection wrapping element attributes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttributeMap {
    entries: HashMap<AttributeName, AttributeValue>,
}

impl AttributeMap {
    /// Creates an empty attribute map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Inserts an attribute into the map.
    pub fn insert(&mut self, name: AttributeName, value: AttributeValue) {
        self.entries.insert(name, value);
    }

    /// Gets an attribute value by name.
    #[must_use]
    pub fn get(&self, name: &AttributeName) -> Option<&AttributeValue> {
        self.entries.get(name)
    }

    /// Checks whether an attribute exists in the map.
    #[must_use]
    pub fn contains(&self, name: &AttributeName) -> bool {
        self.entries.contains_key(name)
    }

    /// Removes an attribute from the map.
    pub fn remove(&mut self, name: &AttributeName) -> Option<AttributeValue> {
        self.entries.remove(name)
    }

    /// Returns the number of attributes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Checks whether the attribute map is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterates over attribute key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&AttributeName, &AttributeValue)> {
        self.entries.iter()
    }
}
