use crate::domain::error::EngineError;
use crate::domain::value::EngineValue;

/// Trait for converting native Rust domain types into canonical dynamic `EngineValue`.
pub trait IntoEngineValue: Send + Sync {
    /// Converts this value into an `EngineValue`.
    fn into_engine_value(self) -> EngineValue;
}

/// Trait for converting from a canonical dynamic `EngineValue` into a native Rust domain type.
pub trait FromEngineValue: Sized + Send + Sync {
    /// Converts from `&EngineValue` into `Self`.
    ///
    /// # Errors
    /// Returns `EngineError::TypeMismatch` if the dynamic value is incompatible.
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError>;
}

// Identity
impl IntoEngineValue for EngineValue {
    fn into_engine_value(self) -> EngineValue {
        self
    }
}

impl FromEngineValue for EngineValue {
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        Ok(value.clone())
    }
}

// Unit / Null
impl IntoEngineValue for () {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Null
    }
}

impl FromEngineValue for () {
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        match value {
            EngineValue::Null => Ok(()),
            other => Err(EngineError::TypeMismatch {
                expected: "Null",
                found: other.type_name(),
            }),
        }
    }
}

// Boolean
impl IntoEngineValue for bool {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Bool(self)
    }
}

impl FromEngineValue for bool {
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        value.as_bool()
    }
}

// Integers
impl IntoEngineValue for i64 {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Int(self)
    }
}

impl FromEngineValue for i64 {
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        value.as_i64()
    }
}

impl IntoEngineValue for i32 {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Int(i64::from(self))
    }
}

impl FromEngineValue for i32 {
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        let val = value.as_i64()?;
        i32::try_from(val).map_err(|_| EngineError::TypeMismatch {
            expected: "i32",
            found: "i64 out of range",
        })
    }
}

impl IntoEngineValue for u32 {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Int(i64::from(self))
    }
}

impl FromEngineValue for u32 {
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        let val = value.as_i64()?;
        u32::try_from(val).map_err(|_| EngineError::TypeMismatch {
            expected: "u32",
            found: "i64 out of range",
        })
    }
}

impl IntoEngineValue for u64 {
    fn into_engine_value(self) -> EngineValue {
        #[allow(clippy::cast_possible_wrap)]
        EngineValue::Int(self as i64)
    }
}

impl FromEngineValue for u64 {
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        let val = value.as_i64()?;
        u64::try_from(val).map_err(|_| EngineError::TypeMismatch {
            expected: "u64",
            found: "negative i64",
        })
    }
}

// Floats
impl IntoEngineValue for f64 {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Float(self)
    }
}

impl FromEngineValue for f64 {
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        value.as_f64()
    }
}

impl IntoEngineValue for f32 {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::Float(f64::from(self))
    }
}

impl FromEngineValue for f32 {
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        let val = value.as_f64()?;
        #[allow(clippy::cast_possible_truncation)]
        Ok(val as f32)
    }
}

// Strings
impl IntoEngineValue for String {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::String(self)
    }
}

impl FromEngineValue for String {
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        value.as_str().map(ToString::to_string)
    }
}

impl IntoEngineValue for &str {
    fn into_engine_value(self) -> EngineValue {
        EngineValue::String(self.to_string())
    }
}

// Collections
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
    fn from_engine_value(value: &EngineValue) -> Result<Self, EngineError> {
        match value {
            EngineValue::Array(arr) => arr.iter().map(FromEngineValue::from_engine_value).collect(),
            other => Err(EngineError::TypeMismatch {
                expected: "Array",
                found: other.type_name(),
            }),
        }
    }
}
