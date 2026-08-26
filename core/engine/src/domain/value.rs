use crate::domain::error::EngineError;
use std::collections::HashMap;

/// Canonical dynamic value representation for data crossing the host/script boundary.
///
/// Decouples domain crates from concrete interpreter values (e.g. `rhai::Dynamic`, `boa::JsValue`).
#[derive(Debug, Clone, PartialEq)]
pub enum EngineValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<EngineValue>),
    Object(HashMap<String, EngineValue>),
}

impl EngineValue {
    /// Extracts boolean value or returns a `TypeMismatch` error.
    pub fn as_bool(&self) -> Result<bool, EngineError> {
        match self {
            Self::Bool(b) => Ok(*b),
            other => Err(EngineError::TypeMismatch {
                expected: "bool",
                found: other.type_name(),
            }),
        }
    }

    /// Extracts `i64` integer or returns a `TypeMismatch` error.
    pub fn as_i64(&self) -> Result<i64, EngineError> {
        match self {
            Self::Int(i) => Ok(*i),
            other => Err(EngineError::TypeMismatch {
                expected: "i64",
                found: other.type_name(),
            }),
        }
    }

    /// Extracts `f64` float or converts an `Int` to `f64`, or returns a `TypeMismatch` error.
    pub fn as_f64(&self) -> Result<f64, EngineError> {
        match self {
            Self::Float(f) => Ok(*f),
            #[allow(clippy::cast_precision_loss)]
            Self::Int(i) => Ok(*i as f64),
            other => Err(EngineError::TypeMismatch {
                expected: "f64",
                found: other.type_name(),
            }),
        }
    }

    /// Extracts string reference or returns a `TypeMismatch` error.
    pub fn as_str(&self) -> Result<&str, EngineError> {
        match self {
            Self::String(s) => Ok(s.as_str()),
            other => Err(EngineError::TypeMismatch {
                expected: "String",
                found: other.type_name(),
            }),
        }
    }

    /// Returns the static type name of this value for diagnostics.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Bool(_) => "Bool",
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::String(_) => "String",
            Self::Array(_) => "Array",
            Self::Object(_) => "Object",
        }
    }
}
