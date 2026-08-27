use engine::{EngineError, EngineValue};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Wrapper around an opaque host instance handle inside Rhai scripts (ADR-0012, N-01, C-57).
#[derive(Clone)]
pub struct RhaiNativeHandle(Arc<dyn Any + Send + Sync>);

impl RhaiNativeHandle {
    /// Wraps an opaque type into a `RhaiNativeHandle`.
    #[must_use]
    pub const fn new(handle: Arc<dyn Any + Send + Sync>) -> Self {
        Self(handle)
    }

    /// Accesses the underlying inner handle.
    #[must_use]
    pub fn inner(&self) -> &Arc<dyn Any + Send + Sync> {
        &self.0
    }

    /// Unwraps into the underlying inner handle.
    #[must_use]
    pub fn into_inner(self) -> Arc<dyn Any + Send + Sync> {
        self.0
    }
}

impl std::fmt::Debug for RhaiNativeHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeHandle(..)")
    }
}

/// Host singleton instance representation in Rhai scope (ADR-0012, N-01, C-57).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhaiSingleton(String);

impl RhaiSingleton {
    /// Creates a new `RhaiSingleton` identifier.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Returns the singleton name slice.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }

    /// Consumes into the underlying String name.
    #[must_use]
    pub fn into_name(self) -> String {
        self.0
    }
}

/// Converts a canonical `EngineValue` into a `rhai::Dynamic`.
pub fn engine_value_to_dynamic(val: EngineValue) -> rhai::Dynamic {
    match val {
        EngineValue::Null => rhai::Dynamic::UNIT,
        EngineValue::Bool(b) => rhai::Dynamic::from(b),
        EngineValue::Int(i) => rhai::Dynamic::from(i),
        EngineValue::Float(f) => rhai::Dynamic::from(f),
        EngineValue::String(s) => rhai::Dynamic::from(s),
        EngineValue::Array(arr) => {
            let rhai_arr: rhai::Array = arr.into_iter().map(engine_value_to_dynamic).collect();
            rhai::Dynamic::from(rhai_arr)
        }
        EngineValue::Object(map) => {
            let rhai_map: rhai::Map = map
                .into_iter()
                .map(|(k, v)| (k.into(), engine_value_to_dynamic(v)))
                .collect();
            rhai::Dynamic::from(rhai_map)
        }
        EngineValue::Handle(arc) => rhai::Dynamic::from(RhaiNativeHandle::new(arc)),
    }
}

/// Converts a `rhai::Dynamic` into a canonical `EngineValue`.
///
/// # Errors
/// Returns `EngineError::TypeMismatch` if the dynamic type is unsupported.
pub fn dynamic_to_engine_value(dyn_val: &rhai::Dynamic) -> Result<EngineValue, EngineError> {
    if dyn_val.is_unit() {
        return Ok(EngineValue::Null);
    }

    if dyn_val.is::<RhaiNativeHandle>() {
        let handle = dyn_val.clone_cast::<RhaiNativeHandle>();
        return Ok(EngineValue::Handle(handle.into_inner()));
    }

    if dyn_val.is_bool() {
        return Ok(EngineValue::Bool(dyn_val.as_bool().unwrap_or(false)));
    }

    if dyn_val.is_int() {
        return Ok(EngineValue::Int(dyn_val.as_int().unwrap_or(0)));
    }

    if dyn_val.is_float() {
        return Ok(EngineValue::Float(dyn_val.as_float().unwrap_or(0.0)));
    }

    if dyn_val.is_string() {
        return Ok(EngineValue::String(dyn_val.clone_cast::<String>()));
    }

    if dyn_val.is_array() {
        let arr = dyn_val.clone_cast::<rhai::Array>();
        let mut converted = Vec::with_capacity(arr.len());
        for item in &arr {
            converted.push(dynamic_to_engine_value(item)?);
        }
        return Ok(EngineValue::Array(converted));
    }

    if dyn_val.is_map() {
        let map = dyn_val.clone_cast::<rhai::Map>();
        let mut converted = HashMap::with_capacity(map.len());
        for (k, v) in &map {
            converted.insert(k.to_string(), dynamic_to_engine_value(v)?);
        }
        return Ok(EngineValue::Object(converted));
    }

    // Fallback: convert custom or other types to string representation
    Ok(EngineValue::String(dyn_val.to_string()))
}

/// Translates a Rhai evaluation error into a domain `EngineError`.
#[must_use]
pub fn rhai_error_to_engine_error(err: rhai::EvalAltResult) -> EngineError {
    match err {
        rhai::EvalAltResult::ErrorTooManyOperations(_) => EngineError::ExecutionLimitExceeded(
            "Rhai execution limit exceeded: too many operations".to_string(),
        ),
        rhai::EvalAltResult::ErrorParsing(parse_err, pos) => EngineError::SyntaxError(format!(
            "{parse_err} at line {}, position {}",
            pos.line().unwrap_or(0),
            pos.position().unwrap_or(0)
        )),
        rhai::EvalAltResult::ErrorMismatchDataType(expected, found, _) => {
            // Leak strings as &'static str or format message
            EngineError::RuntimeError(format!("Type mismatch: expected {expected}, found {found}"))
        }
        rhai::EvalAltResult::ErrorFunctionNotFound(fn_name, _) => {
            EngineError::FunctionNotFound(fn_name)
        }
        rhai::EvalAltResult::ErrorVariableNotFound(var_name, _) => {
            EngineError::VariableNotFound(var_name)
        }
        rhai::EvalAltResult::ErrorRuntime(val, _) => {
            let msg = val.to_string();
            if let Some(cap) = msg.strip_prefix("PermissionDenied: ") {
                EngineError::PermissionDenied(cap.to_string())
            } else {
                EngineError::RuntimeError(msg)
            }
        }
        other => {
            let msg = other.to_string();
            if let Some(cap) = msg.strip_prefix("PermissionDenied: ") {
                EngineError::PermissionDenied(cap.to_string())
            } else {
                EngineError::RuntimeError(msg)
            }
        }
    }
}
