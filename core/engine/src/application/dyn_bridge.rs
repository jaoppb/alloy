//! ADR-0013 — the object-safe **companion** to the `RuntimeEngine` port.
//!
//! `RuntimeEngine` is not object-safe: `create_context` / `compile` return
//! associated types by value and the PRD-002 sugar is generic. `ADR-0011`
//! item 2 nonetheless requires "an object-safe companion form" for every port.
//! This module is it — three `dyn`-safe traits and a free `eval_typed`:
//!
//! - [`DynExecutionContext`] — exactly the object-safe core of
//!   [`ExecutionContext`] (`capabilities`, `register_type_erased`,
//!   `register_native_fn`, `set_value`, `get_value`, `call_function_value`,
//!   `reset_scope`) plus `as_any_mut` for the downcast below.
//! - [`DynCompiledScript`] — an erased compiled program.
//! - [`DynRuntimeEngine`] — `create_context_dyn` / `compile_dyn` /
//!   `eval_value_dyn` / `eval_compiled_value_dyn`, all returning
//!   [`EngineError`].
//!
//! Everything here is **purely additive**: blanket impls give every existing
//! `RuntimeEngine` / `ExecutionContext` the companion for free, and no v0.1
//! signature changes. `MockEngine` and `RhaiEngine` both pass
//! [`crate::conformance::run_dyn_suite`].

use std::any::Any;
use std::sync::Arc;

use crate::application::conversion::FromEngineValue;
use crate::application::engine_type::TypeRegistration;
use crate::application::function::Arity;
use crate::application::ports::{ExecutionContext, NativeFn, RuntimeEngine};
use crate::domain::capability::CapabilitySet;
use crate::domain::error::EngineError;
use crate::domain::value::EngineValue;

/// The object-safe face of [`ExecutionContext`]. Every method speaks only
/// boundary types; `dyn DynExecutionContext` is usable directly.
pub trait DynExecutionContext {
    fn capabilities(&self) -> CapabilitySet;
    fn register_type_erased(&mut self, registration: TypeRegistration) -> Result<(), EngineError>;
    fn register_native_fn(
        &mut self,
        name: &str,
        arity: Arity,
        handler: NativeFn,
    ) -> Result<(), EngineError>;
    fn set_value(&mut self, name: &str, value: EngineValue) -> Result<(), EngineError>;
    fn get_value(&self, name: &str) -> Option<EngineValue>;
    fn call_function_value(
        &mut self,
        name: &str,
        arguments: &[EngineValue],
    ) -> Result<EngineValue, EngineError>;
    fn reset_scope(&mut self) -> Result<(), EngineError>;
    /// For [`DynRuntimeEngine`] to recover the concrete context an engine
    /// handed out. Not part of the port surface.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<Context> DynExecutionContext for Context
where
    Context: ExecutionContext + 'static,
{
    fn capabilities(&self) -> CapabilitySet {
        ExecutionContext::capabilities(self)
    }

    fn register_type_erased(&mut self, registration: TypeRegistration) -> Result<(), EngineError> {
        ExecutionContext::register_type_erased(self, registration)
    }

    fn register_native_fn(
        &mut self,
        name: &str,
        arity: Arity,
        handler: NativeFn,
    ) -> Result<(), EngineError> {
        ExecutionContext::register_native_fn(self, name, arity, handler)
    }

    fn set_value(&mut self, name: &str, value: EngineValue) -> Result<(), EngineError> {
        ExecutionContext::set_value(self, name, value)
    }

    fn get_value(&self, name: &str) -> Option<EngineValue> {
        ExecutionContext::get_value(self, name)
    }

    fn call_function_value(
        &mut self,
        name: &str,
        arguments: &[EngineValue],
    ) -> Result<EngineValue, EngineError> {
        ExecutionContext::call_function_value(self, name, arguments)
    }

    fn reset_scope(&mut self) -> Result<(), EngineError> {
        ExecutionContext::reset_scope(self)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// An erased compiled program produced by [`DynRuntimeEngine::compile_dyn`].
pub trait DynCompiledScript: Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl<Compiled> DynCompiledScript for Compiled
where
    Compiled: Send + Sync + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The object-safe face of [`RuntimeEngine`]. `Box<dyn DynRuntimeEngine>` is a
/// usable engine handle; the generic `eval::<T>` becomes the free
/// [`eval_typed`].
pub trait DynRuntimeEngine: Send + Sync {
    fn create_context_dyn(
        &self,
        capabilities: CapabilitySet,
    ) -> Result<Box<dyn DynExecutionContext>, EngineError>;

    fn compile_dyn(&self, script_source: &str) -> Result<Box<dyn DynCompiledScript>, EngineError>;

    fn eval_value_dyn(
        &self,
        context: &mut dyn DynExecutionContext,
        script_source: &str,
    ) -> Result<EngineValue, EngineError>;

    fn eval_compiled_value_dyn(
        &self,
        context: &mut dyn DynExecutionContext,
        compiled: &dyn DynCompiledScript,
    ) -> Result<EngineValue, EngineError>;
}

impl<Engine> DynRuntimeEngine for Engine
where
    Engine: RuntimeEngine,
    Engine::Context: 'static,
    Engine::CompiledScript: 'static,
{
    fn create_context_dyn(
        &self,
        capabilities: CapabilitySet,
    ) -> Result<Box<dyn DynExecutionContext>, EngineError> {
        let context = RuntimeEngine::create_context(self, capabilities)?;
        Ok(Box::new(context))
    }

    fn compile_dyn(&self, script_source: &str) -> Result<Box<dyn DynCompiledScript>, EngineError> {
        let compiled = RuntimeEngine::compile(self, script_source)?;
        Ok(Box::new(compiled))
    }

    fn eval_value_dyn(
        &self,
        context: &mut dyn DynExecutionContext,
        script_source: &str,
    ) -> Result<EngineValue, EngineError> {
        let concrete = downcast_context::<Engine>(context)?;
        RuntimeEngine::eval_value(self, concrete, script_source)
    }

    fn eval_compiled_value_dyn(
        &self,
        context: &mut dyn DynExecutionContext,
        compiled: &dyn DynCompiledScript,
    ) -> Result<EngineValue, EngineError> {
        let concrete_compiled = compiled
            .as_any()
            .downcast_ref::<Engine::CompiledScript>()
            .ok_or_else(|| {
                EngineError::binding("compiled script does not belong to this engine")
            })?;
        let concrete_context = downcast_context::<Engine>(context)?;
        RuntimeEngine::eval_compiled_value(self, concrete_context, concrete_compiled)
    }
}

fn downcast_context<Engine>(
    context: &mut dyn DynExecutionContext,
) -> Result<&mut Engine::Context, EngineError>
where
    Engine: RuntimeEngine,
    Engine::Context: 'static,
{
    context
        .as_any_mut()
        .downcast_mut::<Engine::Context>()
        .ok_or_else(|| EngineError::binding("execution context does not belong to this engine"))
}

/// PRD-002:42 as a free function over the `dyn` companion: run `script_source`
/// and convert the result to `T`.
pub fn eval_typed<T: FromEngineValue>(
    engine: &dyn DynRuntimeEngine,
    context: &mut dyn DynExecutionContext,
    script_source: &str,
) -> Result<T, EngineError> {
    let value = engine.eval_value_dyn(context, script_source)?;
    T::from_engine_value(value)
}

/// A convenience for callers that only need a boxed handler: wrap a plain
/// closure as a [`NativeFn`].
#[must_use]
pub fn native_fn<F>(handler: F) -> NativeFn
where
    F: Fn(&[EngineValue]) -> Result<EngineValue, EngineError> + Send + Sync + 'static,
{
    Arc::new(handler)
}
