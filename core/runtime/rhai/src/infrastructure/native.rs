//! Adapting a port [`NativeFn`] (dynamic `&[EngineValue] -> EngineValue`) onto a
//! `rhai` raw function of a fixed arity.

use std::any::TypeId;

use engine::{Arity, EngineError, NativeFn};
use rhai::{Dynamic, EvalAltResult, NativeCallContext};

use crate::infrastructure::marshal;

/// Register `handler` on `engine` under `name` with `arity` fixed parameter
/// slots. Marshalling and the port error are converted at the boundary.
pub fn register(
    engine: &mut rhai::Engine,
    name: &str,
    arity: Arity,
    handler: NativeFn,
) -> Result<(), EngineError> {
    let parameter_types = vec![TypeId::of::<Dynamic>(); arity.count()];
    engine.register_raw_fn(
        name,
        &parameter_types,
        move |_call: NativeCallContext, arguments: &mut [&mut Dynamic]| {
            let engine_values = arguments
                .iter()
                .map(|slot| marshal::dynamic_to_engine_value((*slot).clone()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(to_eval_error)?;
            let produced = handler(&engine_values).map_err(to_eval_error)?;
            marshal::engine_value_to_dynamic(produced).map_err(to_eval_error)
        },
    );
    Ok(())
}

/// Carry an [`EngineError`] out of a native binding **without flattening it to a
/// string**: `rhai::EvalAltResult::ErrorSystem` boxes it, and
/// [`crate::infrastructure::error_map::map_eval_error`] downcasts it back on the
/// way out. This is what lets a `PermissionDenied` / `Dom` raised inside a
/// binding surface to the host as that exact variant (C-07, I1).
pub(crate) fn to_eval_error(error: EngineError) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorSystem(String::new(), Box::new(error)))
}
