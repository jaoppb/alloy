//! CSS Custom Properties and Variables (`PRD-007`, CSS Custom Properties for Cascading Variables Module Level 1).
//!
//! Provides the domain primitives for storing and manipulating custom properties (`--*`)
//! and variable values.

use core::fmt;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A validated CSS custom property name (e.g. `--main-color`).
///
/// Per CSS Custom Properties Level 1 §2, custom property names are case-sensitive
/// and must begin with two dashes (`--`).
///
/// The text is shared, not owned: every element that declares a custom
/// property starts from a copy of its parent's whole map, and sharing makes
/// that copy a pointer per entry rather than a string allocation per entry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableName {
    text: Arc<str>,
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
            text: Arc::from(text),
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

    /// Whether a property name, as written, names a custom property (CSS
    /// Variables L1 §2: any name starting with two dashes). Such a name
    /// bypasses the `crate::SUPPORTED_PROPERTIES` registry — the set of custom
    /// properties is open by definition.
    #[must_use]
    pub fn names_custom_property(text: &str) -> bool {
        text.starts_with("--")
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
/// with leading and trailing whitespace trimmed. Shared like
/// [`VariableName`]'s text: a value may be as long as the substitution's
/// 64 KiB expansion cap, and an inherited map must not copy it once per
/// element.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct VariableValue {
    text: Arc<str>,
}

impl VariableValue {
    /// Creates a new variable value from a string slice with trimmed whitespace.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            text: Arc::from(text.trim()),
        }
    }

    /// The string representation of this variable value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Whether this variable value contains no characters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
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
#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
pub enum VariableError {
    /// The variable name is invalid (e.g. missing `--` prefix or contains illegal characters).
    #[error("invalid variable name `{0}`")]
    InvalidName(String),
    /// A referenced variable was not found and no fallback was provided.
    #[error("undefined variable `{0}` without fallback")]
    UndefinedVariable(String),
    /// A cyclic reference was detected during variable resolution.
    #[error("cycle detected in variable references: {}", .0.join(" -> "))]
    CycleDetected(Vec<String>),
    /// Syntax error in a `var(...)` function call.
    #[error("malformed `var()` function: {0}")]
    MalformedVarFunction(String),
    /// An empty `var()` call or missing variable identifier.
    #[error("empty variable reference in `var()`")]
    EmptyVariableReference,
    /// Substitution produced more text than the expansion cap allows. CSS
    /// Variables L1 §3 lets a user agent treat such a value as invalid at
    /// computed-value time; without the cap `--a: var(--b) var(--b)` nested
    /// thirty deep would grow to `2^30` copies (a "billion laughs").
    #[error("`var()` substitution exceeded the {limit_bytes}-byte expansion limit")]
    ExpansionLimit {
        /// The cap that was crossed, in bytes of substituted text.
        limit_bytes: usize,
    },
}
