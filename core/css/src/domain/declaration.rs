use crate::domain::property::{PropertyName, PropertyValue};

/// A single CSS declaration mapping a property name to a property value.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: PropertyName,
    pub value: PropertyValue,
}

impl Declaration {
    /// Creates a new CSS declaration.
    #[must_use]
    pub const fn new(name: PropertyName, value: PropertyValue) -> Self {
        Self { name, value }
    }
}

/// First-class collection wrapping CSS declarations for a rule block (ADR-0010).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeclarationList {
    declarations: Vec<Declaration>,
}

impl DeclarationList {
    /// Creates an empty declaration list.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }

    /// Adds a declaration to the list.
    pub fn push(&mut self, declaration: Declaration) {
        self.declarations.push(declaration);
    }

    /// Looks up a property value by name.
    #[must_use]
    pub fn get(&self, name: &PropertyName) -> Option<&PropertyValue> {
        self.declarations
            .iter()
            .rev()
            .find(|d| &d.name == name)
            .map(|d| &d.value)
    }

    /// Returns the number of declarations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.declarations.len()
    }

    /// Checks if the declaration list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }

    /// Iterates over declarations.
    pub fn iter(&self) -> impl Iterator<Item = &Declaration> {
        self.declarations.iter()
    }
}
