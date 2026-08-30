//! [`EngineValue`] — the **only** type allowed to cross the script boundary.
//!
//! It is an anti-corruption DTO (ADR-0010 §3): a closed set of engine-neutral
//! shapes with no interpreter type inside. Because it is a boundary DTO and not
//! a domain entity, its variants carry bare `bool` / `i64` / `f64` / `String`
//! rather than newtypes — the Object Calisthenics "wrap primitives" rule
//! (ADR-0010 rule 3) is scoped to entities and value objects, and DTO mapping is
//! its documented exception (ADR-0010:114-119).
//!
//! Conversions to and from concrete Rust types live in
//! [`crate::application::conversion`] and are pure `match` arms — never a raw
//! pointer read (PRD-002 invariant 1).
//!
//! [`EngineValue`] is `#[non_exhaustive]` (ADR-0011 item 3): a new shape can be
//! added without it being a breaking change, and an out-of-tree adapter is
//! forced to provide a `_` arm and decide how to fail on a shape it predates.
//! Any such addition bumps [`crate::PORT_SCHEMA_VERSION`].

use std::collections::BTreeMap;
use std::fmt;

use crate::domain::error::EngineError;

/// A value handed across the Rust ⇄ script seam in either direction.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EngineValue {
    /// The absence of a value (script `()` / a statement with no result).
    Unit,
    Bool(bool),
    /// 64-bit signed integer. Backends are pinned so their native integer is
    /// also 64-bit (workspace `rhai` features fix `rhai::INT = i64`).
    Int(i64),
    /// 64-bit float. Backends are pinned so their native float is also 64-bit.
    Float(f64),
    Text(String),
    Array(Vec<EngineValue>),
    Map(BTreeMap<String, EngineValue>),
}

impl EngineValue {
    /// The shape of this value, for diagnostics and type-mismatch messages.
    #[must_use]
    pub const fn kind(&self) -> ValueKind {
        match self {
            Self::Unit => ValueKind::Unit,
            Self::Bool(_) => ValueKind::Bool,
            Self::Int(_) => ValueKind::Int,
            Self::Float(_) => ValueKind::Float,
            Self::Text(_) => ValueKind::Text,
            Self::Array(_) => ValueKind::Array,
            Self::Map(_) => ValueKind::Map,
        }
    }

    #[must_use]
    pub const fn is_unit(&self) -> bool {
        matches!(self, Self::Unit)
    }

    /// Borrow as a `bool`, or [`EngineError::TypeMismatch`].
    pub fn as_bool(&self) -> Result<bool, EngineError> {
        match self {
            Self::Bool(value) => Ok(*value),
            other => Err(EngineError::type_mismatch(
                ValueKind::Bool.name(),
                other.kind().name(),
            )),
        }
    }

    /// Borrow as an `i64`, or [`EngineError::TypeMismatch`].
    pub fn as_int(&self) -> Result<i64, EngineError> {
        match self {
            Self::Int(value) => Ok(*value),
            other => Err(EngineError::type_mismatch(
                ValueKind::Int.name(),
                other.kind().name(),
            )),
        }
    }

    /// Borrow as an `f64`, accepting an integer by widening it, or
    /// [`EngineError::TypeMismatch`].
    pub fn as_float(&self) -> Result<f64, EngineError> {
        match self {
            Self::Float(value) => Ok(*value),
            Self::Int(value) => Ok(*value as f64),
            other => Err(EngineError::type_mismatch(
                ValueKind::Float.name(),
                other.kind().name(),
            )),
        }
    }

    /// Borrow as a string slice, or [`EngineError::TypeMismatch`].
    pub fn as_text(&self) -> Result<&str, EngineError> {
        match self {
            Self::Text(value) => Ok(value),
            other => Err(EngineError::type_mismatch(
                ValueKind::Text.name(),
                other.kind().name(),
            )),
        }
    }

    /// Borrow as a slice of values, or [`EngineError::TypeMismatch`].
    pub fn as_array(&self) -> Result<&[EngineValue], EngineError> {
        match self {
            Self::Array(values) => Ok(values),
            other => Err(EngineError::type_mismatch(
                ValueKind::Array.name(),
                other.kind().name(),
            )),
        }
    }

    /// Borrow as a string-keyed map, or [`EngineError::TypeMismatch`].
    pub fn as_map(&self) -> Result<&BTreeMap<String, EngineValue>, EngineError> {
        match self {
            Self::Map(entries) => Ok(entries),
            other => Err(EngineError::type_mismatch(
                ValueKind::Map.name(),
                other.kind().name(),
            )),
        }
    }
}

impl fmt::Display for EngineValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => formatter.write_str("()"),
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::Int(value) => write!(formatter, "{value}"),
            Self::Float(value) => write!(formatter, "{value}"),
            Self::Text(value) => write!(formatter, "{value}"),
            Self::Array(values) => write_sequence(formatter, values),
            Self::Map(entries) => write_map(formatter, entries),
        }
    }
}

fn write_sequence(formatter: &mut fmt::Formatter<'_>, values: &[EngineValue]) -> fmt::Result {
    formatter.write_str("[")?;
    for (index, value) in values.iter().enumerate() {
        write_separator(formatter, index)?;
        write!(formatter, "{value}")?;
    }
    formatter.write_str("]")
}

fn write_map(
    formatter: &mut fmt::Formatter<'_>,
    entries: &BTreeMap<String, EngineValue>,
) -> fmt::Result {
    formatter.write_str("{")?;
    for (index, (key, value)) in entries.iter().enumerate() {
        write_separator(formatter, index)?;
        write!(formatter, "{key}: {value}")?;
    }
    formatter.write_str("}")
}

fn write_separator(formatter: &mut fmt::Formatter<'_>, index: usize) -> fmt::Result {
    match index {
        0 => Ok(()),
        _ => formatter.write_str(", "),
    }
}

/// The discriminant of an [`EngineValue`], used in diagnostics. Mirrors
/// [`EngineValue`] one-for-one, so it is `#[non_exhaustive]` for the same reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueKind {
    Unit,
    Bool,
    Int,
    Float,
    Text,
    Array,
    Map,
}

impl ValueKind {
    /// A stable lowercase name (`"int"`, `"text"`, …) for error text.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Float => "float",
            Self::Text => "text",
            Self::Array => "array",
            Self::Map => "map",
        }
    }
}

impl fmt::Display for ValueKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
