//! Middle layer: the ports (`RuntimeEngine`, `ExecutionContext`) and the traits
//! that support them. Depends only on [`crate::domain`] (ADR-0010 §1).

pub mod conversion;
pub mod dyn_bridge;
pub mod engine_type;
pub mod function;
pub mod ports;

pub use conversion::{FromEngineValue, IntoEngineValue};
pub use dyn_bridge::{
    DynCompiledScript, DynExecutionContext, DynRuntimeEngine, eval_typed, native_fn,
};
pub use ports::{ExecutionContext, NativeFn, RuntimeEngine};
