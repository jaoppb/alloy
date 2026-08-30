//! [`engine::EngineValue`] ⇄ [`rhai::Dynamic`]. Pure `match` / typed accessors —
//! no `unsafe`, no raw pointer reads (PRD-002 invariant 1). The workspace pins
//! `rhai` so `rhai::INT == i64` and `rhai::FLOAT == f64`, matching
//! `EngineValue::Int` / `EngineValue::Float` exactly.

use std::collections::BTreeMap;

use engine::{EngineError, EngineValue};
use rhai::Dynamic;

/// Project an [`EngineValue`] into a [`rhai::Dynamic`] for the script side.
///
/// Fallible only because `EngineValue` is `#[non_exhaustive]` (ADR-0011 item 3):
/// a shape added to the port after this adapter was built lands in the `_` arm
/// as [`EngineError::Conversion`] rather than a silent wrong value.
pub fn engine_value_to_dynamic(value: EngineValue) -> Result<Dynamic, EngineError> {
    match value {
        EngineValue::Unit => Ok(Dynamic::UNIT),
        EngineValue::Bool(inner) => Ok(Dynamic::from(inner)),
        EngineValue::Int(inner) => Ok(Dynamic::from(inner)),
        EngineValue::Float(inner) => Ok(Dynamic::from(inner)),
        EngineValue::Text(inner) => Ok(Dynamic::from(inner)),
        EngineValue::Array(items) => {
            let array: rhai::Array = items
                .into_iter()
                .map(engine_value_to_dynamic)
                .collect::<Result<_, _>>()?;
            Ok(Dynamic::from(array))
        }
        EngineValue::Map(entries) => {
            let mut map = rhai::Map::new();
            for (key, item) in entries {
                map.insert(key.into(), engine_value_to_dynamic(item)?);
            }
            Ok(Dynamic::from(map))
        }
        other => Err(EngineError::conversion(format!(
            "EngineValue::{other:?} is newer than this rhai adapter (PORT_SCHEMA_VERSION {})",
            engine::PORT_SCHEMA_VERSION
        ))),
    }
}

/// Reconstruct an [`EngineValue`] from a [`rhai::Dynamic`], or explain via
/// [`EngineError`] why the shape is not representable at the boundary.
pub fn dynamic_to_engine_value(value: Dynamic) -> Result<EngineValue, EngineError> {
    if value.is_unit() {
        return Ok(EngineValue::Unit);
    }
    if let Ok(inner) = value.as_bool() {
        return Ok(EngineValue::Bool(inner));
    }
    if let Ok(inner) = value.as_int() {
        return Ok(EngineValue::Int(inner));
    }
    if let Ok(inner) = value.as_float() {
        return Ok(EngineValue::Float(inner));
    }
    if value.is_string() {
        return value
            .into_string()
            .map(EngineValue::Text)
            .map_err(|found| EngineError::conversion(format!("expected a string, found {found}")));
    }
    if value.is_array() {
        let array = value
            .try_cast::<rhai::Array>()
            .ok_or_else(|| EngineError::conversion("value reported as array but did not cast"))?;
        let items = array
            .into_iter()
            .map(dynamic_to_engine_value)
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(EngineValue::Array(items));
    }
    if value.is_map() {
        let map = value
            .try_cast::<rhai::Map>()
            .ok_or_else(|| EngineError::conversion("value reported as map but did not cast"))?;
        let mut entries = BTreeMap::new();
        for (key, item) in map {
            entries.insert(key.to_string(), dynamic_to_engine_value(item)?);
        }
        return Ok(EngineValue::Map(entries));
    }
    Err(EngineError::type_mismatch(
        "engine-representable value",
        value.type_name(),
    ))
}
