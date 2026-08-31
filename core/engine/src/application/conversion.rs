//! [`IntoEngineValue`] / [`FromEngineValue`] (PRD-002:75).
//!
//! These are the *only* sanctioned way a concrete Rust type reaches the script
//! boundary. Every impl here is a pure `match` — no raw pointer read, satisfying
//! PRD-002 invariant 1.

use std::collections::BTreeMap;

use crate::domain::error::EngineError;
use crate::domain::value::{EngineValue, ValueKind};

/// A Rust value that can be projected into an [`EngineValue`].
pub trait IntoEngineValue {
    fn into_engine_value(self) -> EngineValue;
}

/// A Rust type that can be reconstructed from an [`EngineValue`], or explain via
/// [`EngineError`] why it cannot.
pub trait FromEngineValue: Sized {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError>;
}

// ---- identity -------------------------------------------------------------

impl IntoEngineValue for EngineValue {
    fn into_engine_value(self) -> EngineValue {
        self
    }
}

impl FromEngineValue for EngineValue {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError> {
        Ok(value)
    }
}

// ---- unit ---------------------------------------------------------------

impl IntoEngineValue for () {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Unit
    }
}

impl FromEngineValue for () {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError> {
        match value {
            EngineValue::Unit => Ok(()),
            other => Err(EngineError::type_mismatch(
                ValueKind::Unit.name(),
                other.kind().name(),
            )),
        }
    }
}

// ---- bool ------------------------------------------------------------------

impl IntoEngineValue for bool {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Bool(self)
    }
}

impl FromEngineValue for bool {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError> {
        value.as_bool()
    }
}

// ---- integers -----------------------------------------------------------

impl IntoEngineValue for i64 {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Int(self)
    }
}

impl FromEngineValue for i64 {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError> {
        value.as_int()
    }
}

impl IntoEngineValue for i32 {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Int(i64::from(self))
    }
}

impl FromEngineValue for i32 {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError> {
        let wide = value.as_int()?;
        Self::try_from(wide)
            .map_err(|_| EngineError::conversion(format!("{wide} does not fit in i32")))
    }
}

// ---- floats -----------------------------------------------------------------

impl IntoEngineValue for f64 {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Float(self)
    }
}

impl FromEngineValue for f64 {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError> {
        value.as_float()
    }
}

// ---- strings --------------------------------------------------------------

impl IntoEngineValue for String {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Text(self)
    }
}

impl IntoEngineValue for &str {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Text(self.to_owned())
    }
}

impl FromEngineValue for String {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError> {
        match value {
            EngineValue::Text(text) => Ok(text),
            other => Err(EngineError::type_mismatch(
                ValueKind::Text.name(),
                other.kind().name(),
            )),
        }
    }
}

// ---- collections ------------------------------------------------------------

impl<T: IntoEngineValue> IntoEngineValue for Vec<T> {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Array(
            self.into_iter()
                .map(IntoEngineValue::into_engine_value)
                .collect(),
        )
    }
}

impl<T: FromEngineValue> FromEngineValue for Vec<T> {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError> {
        match value {
            EngineValue::Array(items) => items.into_iter().map(T::from_engine_value).collect(),
            other => Err(EngineError::type_mismatch(
                ValueKind::Array.name(),
                other.kind().name(),
            )),
        }
    }
}

impl<T: IntoEngineValue> IntoEngineValue for BTreeMap<String, T> {
    fn into_engine_value(self) -> EngineValue {
        let entries = self
            .into_iter()
            .map(|(key, value)| (key, value.into_engine_value()))
            .collect();
        EngineValue::Map(entries)
    }
}

impl<T: FromEngineValue> FromEngineValue for BTreeMap<String, T> {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError> {
        match value {
            EngineValue::Map(entries) => entries
                .into_iter()
                .map(|(key, value)| T::from_engine_value(value).map(|converted| (key, converted)))
                .collect(),
            other => Err(EngineError::type_mismatch(
                ValueKind::Map.name(),
                other.kind().name(),
            )),
        }
    }
}

// ---- Option (nullable boundary values) ----------------------------------

impl<T: IntoEngineValue> IntoEngineValue for Option<T> {
    fn into_engine_value(self) -> EngineValue {
        self.map_or(EngineValue::Unit, IntoEngineValue::into_engine_value)
    }
}

impl<T: FromEngineValue> FromEngineValue for Option<T> {
    fn from_engine_value(value: EngineValue) -> Result<Self, EngineError> {
        match value {
            EngineValue::Unit => Ok(None),
            other => T::from_engine_value(other).map(Some),
        }
    }
}
