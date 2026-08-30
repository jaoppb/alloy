//! [`RhaiContext`] — one isolated `rhai` scope with a fixed capability grant,
//! and [`RhaiCompiledScript`] — a parsed program behind an `Arc` (ADR-0005).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use engine::{
    Arity, CapabilitySet, EngineError, EngineType, EngineValue, ExecutionContext, FunctionName,
    NativeFn, TypeRegistration,
};

use crate::infrastructure::marshal;
use crate::infrastructure::native;

/// A compiled Rhai program. `Arc<rhai::AST>` is the exact shape hot-reload
/// (F11) swaps atomically.
#[derive(Clone)]
pub struct RhaiCompiledScript {
    pub(crate) ast: Arc<rhai::AST>,
}

impl RhaiCompiledScript {
    pub(crate) fn new(ast: rhai::AST) -> Self {
        Self { ast: Arc::new(ast) }
    }
}

/// An isolated execution scope. Each one owns its own `rhai::Engine` so native
/// functions and registered types never leak between subsystems (PRD-003:78,
/// C-08).
pub struct RhaiContext {
    pub(crate) engine: rhai::Engine,
    pub(crate) scope: rhai::Scope<'static>,
    capabilities: CapabilitySet,
    /// Set to `Some(instant)` for the duration of an evaluation; the engine's
    /// `on_progress` guard reads it to enforce the wall-clock ceiling.
    pub(crate) deadline: Arc<Mutex<Option<Instant>>>,
    /// Native bindings, kept so `call_function_value` can invoke one directly
    /// from Rust without going through the interpreter.
    native_functions: HashMap<FunctionName, NativeFn>,
    registered_types: Vec<TypeRegistration>,
}

impl RhaiContext {
    pub(crate) fn new(
        engine: rhai::Engine,
        capabilities: CapabilitySet,
        deadline: Arc<Mutex<Option<Instant>>>,
    ) -> Self {
        Self {
            engine,
            scope: rhai::Scope::new(),
            capabilities,
            deadline,
            native_functions: HashMap::new(),
            registered_types: Vec::new(),
        }
    }

    /// Adapter extension beyond the port: register a type that is both a port
    /// [`EngineType`] and a [`rhai::CustomType`], so scripts can hold and mutate
    /// values of it (mechanism of C-02). The generic capability-guarded binding
    /// registrar is v0.2 (roadmap I1).
    pub fn register_custom_type<T>(&mut self) -> Result<(), EngineError>
    where
        T: EngineType + rhai::CustomType,
    {
        self.engine.build_type::<T>();
        self.registered_types.push(T::registration());
        Ok(())
    }

    /// Adapter extension: push a concrete custom value into the scope (there is
    /// no [`EngineValue`] shape for a `rhai::CustomType`). The bound is what
    /// `rhai`'s sealed `Variant` marker is auto-implemented for.
    pub fn set_custom_value<T>(&mut self, name: &str, value: T)
    where
        T: Clone + Send + Sync + 'static,
    {
        self.scope.set_value(name.to_string(), value);
    }

    /// Adapter extension: read a concrete custom value back out of the scope.
    #[must_use]
    pub fn custom_value<T>(&self, name: &str) -> Option<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.scope.get_value::<T>(name)
    }

    /// The script-visible names of every type registered on this context.
    #[must_use]
    pub fn registered_type_names(&self) -> Vec<&'static str> {
        self.registered_types
            .iter()
            .map(TypeRegistration::script_name)
            .collect()
    }
}

impl ExecutionContext for RhaiContext {
    fn capabilities(&self) -> CapabilitySet {
        self.capabilities
    }

    fn register_type_erased(&mut self, registration: TypeRegistration) -> Result<(), EngineError> {
        // Name-only in v0.1: the concrete `rhai::CustomType` bridge needs the
        // static type and is offered by `register_custom_type`.
        self.registered_types.push(registration);
        Ok(())
    }

    fn register_native_fn(
        &mut self,
        name: &FunctionName,
        arity: Arity,
        handler: NativeFn,
    ) -> Result<(), EngineError> {
        native::register(&mut self.engine, name.as_str(), arity, handler.clone())?;
        self.native_functions.insert(name.clone(), handler);
        Ok(())
    }

    fn set_value(&mut self, name: &str, value: EngineValue) -> Result<(), EngineError> {
        let dynamic = marshal::engine_value_to_dynamic(value)?;
        self.scope.set_value(name.to_string(), dynamic);
        Ok(())
    }

    fn get_value(&self, name: &str) -> Option<EngineValue> {
        let dynamic = self.scope.get_value::<rhai::Dynamic>(name)?;
        marshal::dynamic_to_engine_value(dynamic).ok()
    }

    fn call_function_value(
        &mut self,
        name: &FunctionName,
        arguments: &[EngineValue],
    ) -> Result<EngineValue, EngineError> {
        let handler = self
            .native_functions
            .get(name)
            .cloned()
            .ok_or_else(|| EngineError::binding(format!("unknown function `{name}`")))?;
        handler(arguments)
    }

    fn reset_scope(&mut self) -> Result<(), EngineError> {
        self.scope.clear();
        Ok(())
    }
}
