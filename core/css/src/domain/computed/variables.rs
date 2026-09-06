//! CSS Custom Properties and Variables (`PRD-007`, CSS Custom Properties for Cascading Variables Module Level 1).
//!
//! Provides the domain primitives for storing and manipulating custom properties (`--*`)
//! and variable values.

use core::fmt;
use std::collections::BTreeMap;

/// A validated CSS custom property name (e.g. `--main-color`).
///
/// Per CSS Custom Properties Level 1 §2, custom property names are case-sensitive
/// and must begin with two dashes (`--`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableName {
    text: String,
}

impl VariableName {
    /// Creates a validated variable name, returning `None` if invalid.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let stripped = text.strip_prefix("--")?;
        if stripped.is_empty() || stripped.chars().any(is_forbidden_variable_char) {
            return None;
        }
        Some(Self {
            text: text.to_owned(),
        })
    }

    /// Parses a custom property name, returning a typed [`VariableError`].
    ///
    /// # Errors
    ///
    /// Returns [`VariableError::EmptyVariableReference`] if `text` is empty,
    /// or [`VariableError::InvalidName`] if the name does not start with `--` or contains illegal characters.
    pub fn parse(text: &str) -> Result<Self, VariableError> {
        if text.is_empty() {
            return Err(VariableError::EmptyVariableReference);
        }
        Self::new(text).ok_or_else(|| VariableError::InvalidName(text.to_owned()))
    }

    /// Returns the variable name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// Checks whether a character cannot appear in a variable name identifier.
const fn is_forbidden_variable_char(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            ' ' | '\t'
                | '\n'
                | '\r'
                | '"'
                | '\''
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | ','
                | ';'
                | ':'
                | '>'
                | '+'
                | '~'
                | '*'
                | '/'
                | '@'
                | '#'
                | '.'
                | '|'
                | '!'
                | '='
                | '%'
                | '&'
                | '?'
                | '<'
                | '$'
                | '^'
                | '`'
        )
}

impl fmt::Display for VariableName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// The value of a custom property.
///
/// Custom property values preserve their token representation or raw text,
/// with leading and trailing whitespace trimmed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct VariableValue {
    text: String,
}

impl VariableValue {
    /// Creates a new variable value from a string slice with trimmed whitespace.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            text: text.trim().to_owned(),
        }
    }

    /// The string representation of this variable value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Whether this variable value contains no characters.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Whether this value contains a `var(` reference.
    #[must_use]
    pub fn contains_var(&self) -> bool {
        self.text.contains("var(")
    }
}

impl fmt::Display for VariableValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// A first-class collection of custom properties (`--*`).
///
/// Maps [`VariableName`] to [`VariableValue`]. Uses [`BTreeMap`] for deterministic
/// traversal ordering during style computation and serialization.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomPropertiesMap {
    entries: BTreeMap<VariableName, VariableValue>,
}

impl CustomPropertiesMap {
    /// Creates an empty collection of custom properties.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Retrieves a variable value by its name.
    #[must_use]
    pub fn get(&self, name: &VariableName) -> Option<&VariableValue> {
        self.entries.get(name)
    }

    /// Sets or replaces the value for a custom property.
    pub fn set(&mut self, name: VariableName, value: VariableValue) {
        self.entries.insert(name, value);
    }

    /// Removes a custom property from the collection.
    pub fn remove(&mut self, name: &VariableName) -> Option<VariableValue> {
        self.entries.remove(name)
    }

    /// Checks if a custom property name is present.
    #[must_use]
    pub fn contains(&self, name: &VariableName) -> bool {
        self.entries.contains_key(name)
    }

    /// Number of custom properties in the collection.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterates over all variable name and value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&VariableName, &VariableValue)> {
        self.entries.iter()
    }

    /// Creates an inherited custom properties map from parent and child declarations.
    ///
    /// The resulting map contains all variables from `parent`, overridden by any
    /// local declarations in `child_local`.
    #[must_use]
    pub fn inherited(parent: &Self, child_local: &Self) -> Self {
        let mut merged = parent.clone();
        for (name, value) in child_local.iter() {
            merged.set(name.clone(), value.clone());
        }
        merged
    }

    /// Returns a new map containing inherited variables from `parent` overridden by `self`.
    #[must_use]
    pub fn inherit_from(&self, parent: &Self) -> Self {
        Self::inherited(parent, self)
    }
}

/// Typed errors for custom property and variable processing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariableError {
    /// The variable name is invalid (e.g. missing `--` prefix or contains illegal characters).
    InvalidName(String),
    /// A referenced variable was not found and no fallback was provided.
    UndefinedVariable(String),
    /// A cyclic reference was detected during variable resolution.
    CycleDetected(Vec<String>),
    /// Syntax error in a `var(...)` function call.
    MalformedVarFunction(String),
    /// An empty `var()` call or missing variable identifier.
    EmptyVariableReference,
}

impl fmt::Display for VariableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(formatter, "invalid variable name `{name}`"),
            Self::UndefinedVariable(name) => {
                write!(formatter, "undefined variable `{name}` without fallback")
            }
            Self::CycleDetected(cycle) => {
                write!(
                    formatter,
                    "cycle detected in variable references: {}",
                    cycle.join(" -> ")
                )
            }
            Self::MalformedVarFunction(reason) => {
                write!(formatter, "malformed `var()` function: {reason}")
            }
            Self::EmptyVariableReference => {
                formatter.write_str("empty variable reference in `var()`")
            }
        }
    }
}

impl std::error::Error for VariableError {}
