//! Adapting a port [`NativeFn`] (dynamic `&[EngineValue] -> EngineValue`) onto a
//! `rhai` raw function of a fixed arity.

use std::any::TypeId;

use engine::{Arity, EngineError, EngineValue, NativeFn};
use rhai::{Dynamic, EvalAltResult, NativeCallContext};

use crate::infrastructure::marshal::RhaiValue;

/// Register `handler` on `engine` under `name` with `arity` fixed parameter
/// slots. Marshalling and the port error are converted at the boundary.
pub fn register(engine: &mut rhai::Engine, name: &str, arity: Arity, handler: NativeFn) {
    let parameter_types = vec![TypeId::of::<Dynamic>(); arity.count()];
    engine.register_raw_fn(
        name,
        &parameter_types,
        move |_call: NativeCallContext, arguments: &mut [&mut Dynamic]| {
            let engine_values = arguments
                .iter()
                .map(|slot| EngineValue::try_from(RhaiValue((*slot).clone())))
                .collect::<Result<Vec<_>, _>>()
                .map_err(to_eval_error)?;
            let produced = handler(&engine_values).map_err(to_eval_error)?;
            RhaiValue::try_from(produced)
                .map(|wrapped| wrapped.0)
                .map_err(to_eval_error)
        },
    );
}

/// Carry an [`EngineError`] out of a native binding without flattening it to a
/// string.
///
/// `rhai::EvalAltResult::ErrorSystem` boxes the error, and
/// [`crate::infrastructure::error_map::map_eval_error`] downcasts it back on the
/// way out. This is what lets a `PermissionDenied` / `Dom` raised inside a
/// binding surface to the host as that exact variant (C-07, I1). `rhai-bindings`
/// re-uses it for the domain bridges (v0.5 report §2.12).
#[must_use]
#[allow(clippy::unnecessary_box_returns)] // `Box<EvalAltResult>` is rhai's required error type
pub fn to_eval_error(error: EngineError) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorSystem(String::new(), Box::new(error)))
}
