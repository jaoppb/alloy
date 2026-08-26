use crate::domain::error::EngineError;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Canonical dynamic value representation for data crossing the host/script boundary.
///
/// Decouples domain crates from concrete interpreter values (e.g. `rhai::Dynamic`, `boa::JsValue`).
#[derive(Clone)]
pub enum EngineValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<EngineValue>),
    Object(HashMap<String, EngineValue>),
    /// Opaque native host instance handle (ADR-0012, N-01).
    Handle(Arc<dyn Any + Send + Sync>),
}

impl PartialEq for EngineValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => a == b,
            (Self::Handle(a), Self::Handle(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl fmt::Debug for EngineValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "Null"),
            Self::Bool(b) => write!(f, "Bool({b:?})"),
            Self::Int(i) => write!(f, "Int({i:?})"),
            Self::Float(fl) => write!(f, "Float({fl:?})"),
            Self::String(s) => write!(f, "String({s:?})"),
            Self::Array(a) => write!(f, "Array({a:?})"),
            Self::Object(o) => write!(f, "Object({o:?})"),
            Self::Handle(_) => write!(f, "Handle(..)"),
        }
    }
}

impl EngineValue {
    /// Creates an opaque handle wrapping a concrete native Rust instance.
    pub fn handle<T: Send + Sync + 'static>(val: T) -> Self {
        Self::Handle(Arc::new(val))
    }

    /// Attempts to downcast a handle reference to concrete type `T`.
    pub fn downcast_handle<T: 'static>(&self) -> Option<&T> {
        match self {
            Self::Handle(arc) => arc.downcast_ref::<T>(),
            _ => None,
        }
    }

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
            Self::Handle(_) => "Handle",
        }
    }
}
