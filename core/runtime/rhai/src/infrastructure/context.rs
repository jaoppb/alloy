//! [`RhaiContext`] — one isolated `rhai` scope with a fixed capability grant,
//! and [`RhaiCompiledScript`] — a parsed program behind an `Arc` (ADR-0005).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use dom::DomTree;
use engine::{
    Arity, Capability, CapabilitySet, EngineError, EngineType, EngineValue, ExecutionContext,
    FunctionName, NativeFn, TypeRegistration, VariableName,
};

use crate::infrastructure::dom_bindings::NodeHandle;
use crate::infrastructure::marshal::RhaiValue;
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
    /// The host-owned DOM tree bound by [`RhaiContext::bind_dom`], if any. The
    /// context holds an `Arc` clone; the host reads the mutated tree after
    /// `eval` returns (`ADR-0003`, contract §5.1).
    dom: Option<Arc<Mutex<DomTree>>>,
    /// `(name, required)` for every binding installed through
    /// [`RhaiContext::register_guarded_binding`]. The F6 conformance sweep walks
    /// this alongside `NODE_HANDLE_BINDINGS` to prove no DOM binding is
    /// unguarded (C-06).
    guarded_bindings: Vec<(FunctionName, Capability)>,
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
            dom: None,
            guarded_bindings: Vec::new(),
        }
    }

    /// **C-06 / C-07**: register `handler` under `name` behind a single-point
    /// capability guard. Every call from a script first checks `required`
    /// against this context's grant — captured by value at registration, so
    /// there is no per-call relookup — and returns
    /// [`EngineError::PermissionDenied`] on a miss before the handler runs.
    /// `register_fn` / `register_native_fn` remain for **pure** bindings.
    pub fn register_guarded_binding(
        &mut self,
        name: &FunctionName,
        arity: Arity,
        required: Capability,
        handler: NativeFn,
    ) -> Result<(), EngineError> {
        let capabilities = self.capabilities;
        let guarded: NativeFn = Arc::new(move |arguments: &[EngineValue]| {
            capabilities.require(required)?;
            handler(arguments)
        });
        self.register_native_fn(name, arity, guarded)?;
        self.guarded_bindings.push((name.clone(), required));
        Ok(())
    }

    /// `(name, required)` for every binding registered through
    /// [`register_guarded_binding`](Self::register_guarded_binding).
    #[must_use]
    pub fn guarded_binding_names(&self) -> &[(FunctionName, Capability)] {
        &self.guarded_bindings
    }

    /// Adapter extension beyond the port (roadmap I1): bind a host-owned
    /// [`DomTree`] into this context so scripts can read and mutate it through a
    /// global `document` handle (C-03). Registers [`NodeHandle`] as a script
    /// type and stamps the `document` handle with this context's capability set
    /// — `ADR-0004` fixes capabilities at context creation, so baking the set
    /// into the handle is sound. Every DOM binding then checks that set (C-06);
    /// a missing capability is [`EngineError::PermissionDenied`] (C-07).
    pub fn bind_dom(&mut self, tree: Arc<Mutex<DomTree>>) -> Result<(), EngineError> {
        self.register_custom_type::<NodeHandle>()?;
        let root = tree
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .document();
        let document = NodeHandle::new(Arc::clone(&tree), root, self.capabilities);
        let name = VariableName::parse("document")?;
        self.set_custom_value(&name, document);
        self.dom = Some(tree);
        Ok(())
    }

    /// The DOM tree bound by [`bind_dom`](Self::bind_dom), if any. The host reads
    /// the mutated tree through this after an evaluation returns.
    #[must_use]
    pub const fn dom(&self) -> Option<&Arc<Mutex<DomTree>>> {
        self.dom.as_ref()
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
    pub fn set_custom_value<T>(&mut self, name: &VariableName, value: T)
    where
        T: Clone + Send + Sync + 'static,
    {
        self.scope.set_value(name.as_str().to_owned(), value);
    }

    /// Adapter extension: read a concrete custom value back out of the scope.
    #[must_use]
    pub fn custom_value<T>(&self, name: &VariableName) -> Option<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.scope.get_value::<T>(name.as_str())
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
        native::register(&mut self.engine, name.as_str(), arity, handler.clone());
        self.native_functions.insert(name.clone(), handler);
        Ok(())
    }

    fn set_value(&mut self, name: &VariableName, value: EngineValue) -> Result<(), EngineError> {
        let RhaiValue(dynamic) = RhaiValue::try_from(value)?;
        self.scope.set_value(name.as_str().to_owned(), dynamic);
        Ok(())
    }

    fn get_value(&self, name: &VariableName) -> Option<EngineValue> {
        let dynamic = self.scope.get_value::<rhai::Dynamic>(name.as_str())?;
        EngineValue::try_from(RhaiValue(dynamic)).ok()
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
