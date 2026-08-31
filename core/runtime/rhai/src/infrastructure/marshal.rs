//! [`engine::EngineValue`] ⇄ [`rhai::Dynamic`], spelled as `TryFrom` on the
//! [`RhaiValue`] newtype (review comment on `marshal.rs`). Pure `match` / typed
//! accessors — no `unsafe`, no raw pointer reads (PRD-002 invariant 1). The
//! workspace pins `rhai` so `rhai::INT == i64` and `rhai::FLOAT == f64`,
//! matching `EngineValue::Int` / `EngineValue::Float` exactly.

use std::collections::BTreeMap;

use engine::{EngineError, EngineValue};
use rhai::Dynamic;

/// A `rhai::Dynamic` sitting on the port boundary. The newtype is what lets the
/// marshalling be `TryFrom` impls: `Dynamic` and `EngineValue` are both foreign,
/// but a local type on one side of each `impl` satisfies the orphan rule.
pub struct RhaiValue(pub Dynamic);

impl TryFrom<EngineValue> for RhaiValue {
    type Error = EngineError;

    /// Fallible only because `EngineValue` is `#[non_exhaustive]` (ADR-0011 item
    /// 3): a shape added to the port after this adapter was built lands in the
    /// `_` arm as [`EngineError::Conversion`] rather than a silent wrong value.
    fn try_from(value: EngineValue) -> Result<Self, EngineError> {
        let dynamic = match value {
            EngineValue::Unit => Dynamic::UNIT,
            EngineValue::Bool(inner) => Dynamic::from(inner),
            EngineValue::Int(inner) => Dynamic::from(inner),
            EngineValue::Float(inner) => Dynamic::from(inner),
            EngineValue::Text(inner) => Dynamic::from(inner),
            EngineValue::Array(items) => {
                let array: rhai::Array = items
                    .into_iter()
                    .map(|item| Self::try_from(item).map(|wrapped| wrapped.0))
                    .collect::<Result<_, _>>()?;
                Dynamic::from(array)
            }
            EngineValue::Map(entries) => {
                let mut map = rhai::Map::new();
                for (key, item) in entries {
                    map.insert(key.into(), Self::try_from(item)?.0);
                }
                Dynamic::from(map)
            }
            other => {
                return Err(EngineError::conversion(format!(
                    "EngineValue::{other:?} is newer than this rhai adapter (PORT_SCHEMA_VERSION {})",
                    engine::PORT_SCHEMA_VERSION
                )));
            }
        };
        Ok(Self(dynamic))
    }
}

impl TryFrom<RhaiValue> for EngineValue {
    type Error = EngineError;

    fn try_from(RhaiValue(value): RhaiValue) -> Result<Self, EngineError> {
        if value.is_unit() {
            return Ok(Self::Unit);
        }
        if let Ok(inner) = value.as_bool() {
            return Ok(Self::Bool(inner));
        }
        if let Ok(inner) = value.as_int() {
            return Ok(Self::Int(inner));
        }
        if let Ok(inner) = value.as_float() {
            return Ok(Self::Float(inner));
        }
        if value.is_string() {
            let Ok(text) = value.into_string() else {
                return Err(EngineError::conversion(
                    "rhai reported a string it could not yield",
                ));
            };
            return Ok(Self::Text(text));
        }
        if value.is_array() {
            let Some(array) = value.try_cast::<rhai::Array>() else {
                return Err(EngineError::conversion(
                    "rhai reported an array it could not yield",
                ));
            };
            let items = array
                .into_iter()
                .map(|item| Self::try_from(RhaiValue(item)))
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(Self::Array(items));
        }
        if value.is_map() {
            let Some(map) = value.try_cast::<rhai::Map>() else {
                return Err(EngineError::conversion(
                    "rhai reported a map it could not yield",
                ));
            };
            let mut entries = BTreeMap::new();
            for (key, item) in map {
                entries.insert(key.to_string(), Self::try_from(RhaiValue(item))?);
            }
            return Ok(Self::Map(entries));
        }
        Err(EngineError::type_mismatch(
            "engine-representable value",
            value.type_name(),
        ))
    }
}
