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
                .map_err(|error| to_eval_error(&error))?;
            let produced = handler(&engine_values).map_err(|error| to_eval_error(&error))?;
            RhaiValue::try_from(produced)
                .map(|wrapped| wrapped.0)
                .map_err(|error| to_eval_error(&error))
        },
    );
}

#[allow(clippy::unnecessary_box_returns)] // `Box<EvalAltResult>` is rhai's required error type
fn to_eval_error(error: &EngineError) -> Box<EvalAltResult> {
    error.to_string().into()
}
