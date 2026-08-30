//! The ports themselves: [`RuntimeEngine`] and [`ExecutionContext`], transcribed
//! from PRD-002:35-59 with two deliberate, documented deviations required by
//! ADR-0011:
//!
//! 1. **No associated `type Error`.** PRD-002 gives each trait its own
//!    `type Error`; ADR-0011 item 4 mandates *exactly one* error enum per port.
//!    Every method here returns [`EngineError`] directly. This also lets
//!    consumers stay generic over the engine without threading an error type
//!    (C-05).
//! 2. **`EngineType` instead of `rhai::CustomType`.** See
//!    [`crate::application::engine_type`].
//!
//! The PRD-002 generic methods (`eval::<T>`, `register_fn`, `set_variable::<V>`,
//! `call_function::<T>`, `register_type::<T>`) are kept verbatim in spirit but
//! are **provided** methods carrying `where Self: Sized`, layered over a small
//! object-safe core that speaks only [`EngineValue`] / [`EngineError`].
//! Consequences:
//!
//! - `dyn ExecutionContext` **is** legal today — the context port is already
//!   object-safe.
//! - `dyn RuntimeEngine` is not (associated types by value; generic sugar). The
//!   `dyn`-dispatch companion is roadmap v0.2 / ADR-0013. Until then: monomorphise
//!   with `fn run<E: RuntimeEngine>(…)`.

use std::sync::Arc;

use crate::application::conversion::{FromEngineValue, IntoEngineValue};
use crate::application::engine_type::{EngineType, TypeRegistration};
use crate::application::function::{Arity, EngineFunction};
use crate::domain::capability::CapabilitySet;
use crate::domain::error::EngineError;
use crate::domain::function_name::FunctionName;
use crate::domain::value::EngineValue;

/// A type-erased native function body: marshalled arguments in, one value or an
/// error out. This is the shape F6 will wrap with a capability guard.
pub type NativeFn = Arc<dyn Fn(&[EngineValue]) -> Result<EngineValue, EngineError> + Send + Sync>;

/// An isolated script scope with a fixed capability grant (PRD-002:45-59,
/// PRD-003:78). One per subsystem, per isolate — never shared.
///
/// Implementors provide the seven object-safe methods; the generic conveniences
/// are supplied here.
pub trait ExecutionContext {
    // ---- required, object-safe core --------------------------------------

    /// The grant this context was created with. Never widens.
    fn capabilities(&self) -> CapabilitySet;

    /// Make a Rust type usable from scripts in this context.
    fn register_type_erased(&mut self, registration: TypeRegistration) -> Result<(), EngineError>;

    /// Bind a native function under `name`. `arity` lets an adapter reserve a
    /// binding of the right shape (e.g. Rhai's `register_raw_fn` needs a fixed
    /// parameter count); the `handler` re-checks it and errors on a mismatch.
    fn register_native_fn(
        &mut self,
        name: &FunctionName,
        arity: Arity,
        handler: NativeFn,
    ) -> Result<(), EngineError>;

    /// Set a scope variable to an already-marshalled value.
    fn set_value(&mut self, name: &str, value: EngineValue) -> Result<(), EngineError>;

    /// Read a scope variable, if present.
    fn get_value(&self, name: &str) -> Option<EngineValue>;

    /// Invoke a **registered native binding** by name (the handler installed by
    /// [`register_native_fn`](Self::register_native_fn) / `register_fn`).
    ///
    /// v0.1 does **not** cover invoking a function *defined by a compiled
    /// script* — the `on_init` / `on_event` / `on_process` / `on_reload` hook
    /// lifecycle of PRD-001 §5.2, which hot-reload (PRD-004) needs. That
    /// requires threading the compiled AST through this call and is tracked for
    /// v0.2 with a PRD-002 amendment. An unknown `name` returns
    /// [`EngineError::Binding`].
    fn call_function_value(
        &mut self,
        name: &FunctionName,
        arguments: &[EngineValue],
    ) -> Result<EngineValue, EngineError>;

    /// Drop all script-local state, keeping registrations. Used by hot-reload
    /// (ADR-0005) and after a trapped fault (PRD-003:70).
    fn reset_scope(&mut self) -> Result<(), EngineError>;

    // ---- provided PRD-002 sugar (monomorphised; `dyn` uses the core above) --

    /// PRD-002:48. Register `T` by its [`EngineType::registration`] descriptor.
    fn register_type<T>(&mut self) -> Result<(), EngineError>
    where
        Self: Sized,
        T: EngineType,
    {
        self.register_type_erased(T::registration())
    }

    /// PRD-002:49-51. Bind an ordinary Rust closure, adapting it through
    /// [`EngineFunction`]. `name` is validated into a [`FunctionName`].
    fn register_fn<Function, Args, Ret>(
        &mut self,
        name: &str,
        function: Function,
    ) -> Result<(), EngineError>
    where
        Self: Sized,
        Function: EngineFunction<Args, Ret>,
    {
        let name = FunctionName::parse(name)?;
        let arity = function.arity();
        let handler: NativeFn =
            Arc::new(move |arguments: &[EngineValue]| function.invoke(arguments));
        self.register_native_fn(&name, arity, handler)
    }

    /// PRD-002:52. Set a scope variable from any [`IntoEngineValue`].
    fn set_variable<Value>(&mut self, name: &str, value: Value) -> Result<(), EngineError>
    where
        Self: Sized,
        Value: IntoEngineValue,
    {
        self.set_value(name, value.into_engine_value())
    }

    /// PRD-002:53-57. Invoke a registered native binding by name and convert its
    /// result (see [`call_function_value`](Self::call_function_value) for the
    /// v0.1 scope limit).
    fn call_function<Ret>(
        &mut self,
        name: &str,
        arguments: &[EngineValue],
    ) -> Result<Ret, EngineError>
    where
        Self: Sized,
        Ret: FromEngineValue,
    {
        let name = FunctionName::parse(name)?;
        let value = self.call_function_value(&name, arguments)?;
        Ret::from_engine_value(value)
    }
}

/// A script backend (PRD-002:35-43). `Send + Sync` so one engine can serve many
/// subsystem contexts across threads.
pub trait RuntimeEngine: Send + Sync {
    /// The scope type this engine hands out.
    type Context: ExecutionContext;

    /// A parsed, reusable program. `'static` so hot-reload can hold it behind an
    /// `Arc` and swap atomically (ADR-0005).
    type CompiledScript: Send + Sync + 'static;

    /// PRD-002:40. Build an isolated context with exactly `capabilities`.
    fn create_context(&self, capabilities: CapabilitySet) -> Result<Self::Context, EngineError>;

    /// PRD-002:41. Parse `script_source`, mapping syntax errors to
    /// [`EngineError::Compilation`] with a [`crate::SourceLocation`] when known.
    fn compile(&self, script_source: &str) -> Result<Self::CompiledScript, EngineError>;

    /// Compile-and-run `script_source`, returning the raw boundary value. The
    /// object-safe core of PRD-002:42.
    fn eval_value(
        &self,
        context: &mut Self::Context,
        script_source: &str,
    ) -> Result<EngineValue, EngineError>;

    /// Run an already-[`compile`](Self::compile)d program. The path hot-reload
    /// (F11) uses so a running subsystem never re-parses.
    fn eval_compiled_value(
        &self,
        context: &mut Self::Context,
        compiled: &Self::CompiledScript,
    ) -> Result<EngineValue, EngineError>;

    /// PRD-002:42 verbatim (bar the error-type deviation): run `script_source`
    /// and convert the result to `T`.
    fn eval<T>(&self, context: &mut Self::Context, script_source: &str) -> Result<T, EngineError>
    where
        T: FromEngineValue,
    {
        let value = self.eval_value(context, script_source)?;
        T::from_engine_value(value)
    }

    /// [`eval`](Self::eval) against a pre-compiled program.
    fn eval_compiled<T>(
        &self,
        context: &mut Self::Context,
        compiled: &Self::CompiledScript,
    ) -> Result<T, EngineError>
    where
        T: FromEngineValue,
    {
        let value = self.eval_compiled_value(context, compiled)?;
        T::from_engine_value(value)
    }
}
