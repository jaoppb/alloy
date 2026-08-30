//! Adapting a port [`NativeFn`] (dynamic `&[EngineValue] -> EngineValue`) onto a
//! `rhai` raw function of a fixed arity.

use std::any::TypeId;

use engine::{Arity, EngineError, NativeFn};
use rhai::{Dynamic, EvalAltResult, NativeCallContext};

use crate::infrastructure::marshal;

/// Register `handler` on `engine` under `name` with `arity` fixed parameter
/// slots. Marshalling and the port error are converted at the boundary.
#[allow(clippy::unnecessary_wraps)] // fallible-shaped for the name validation WP-5 adds
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
                .map_err(|error| to_eval_error(&error))?;
            let produced = handler(&engine_values).map_err(|error| to_eval_error(&error))?;
            marshal::engine_value_to_dynamic(produced).map_err(|error| to_eval_error(&error))
        },
    );
    Ok(())
}

#[allow(clippy::unnecessary_box_returns)] // `Box<EvalAltResult>` is rhai's required error type
fn to_eval_error(error: &EngineError) -> Box<EvalAltResult> {
    error.to_string().into()
}
