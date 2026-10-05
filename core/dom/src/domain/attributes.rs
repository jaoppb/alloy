//! [`AttributeMap`], the first-class collection of an element's attributes (v0.2 report §2.2;
//! `ADR-0010:132` rule 4), keyed by the `html` vocabulary (`ADR-0024`).

use std::collections::BTreeMap;

use html::{AttributeName, AttributeValue};

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
