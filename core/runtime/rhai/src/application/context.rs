use crate::domain::limits::ExecutionLimits;
use crate::domain::marshaling::{
    RhaiNativeHandle, RhaiSingleton, dynamic_to_engine_value, engine_value_to_dynamic,
};
use engine::{
    CapabilitySet, EngineError, EngineValue, ExecutionContext, HostObject, Identifier, NativeFn,
};
use rhai::{Dynamic, EvalAltResult};
use std::collections::HashMap;
use std::sync::Arc;

/// An isolated Rhai execution context holding its own `rhai::Scope`, its own `rhai::Engine`, and capability permissions.
pub struct RhaiContext {
    capabilities: CapabilitySet,
    scope: rhai::Scope<'static>,
    engine: rhai::Engine,
    functions: HashMap<String, NativeFn>,
    host_objects: HashMap<String, HostObject>,
}

impl RhaiContext {
    /// Creates a new Rhai context with default limits and given capability set.
    #[must_use]
    pub fn new(capabilities: CapabilitySet) -> Self {
        Self::with_limits(capabilities, ExecutionLimits::new())
    }

    /// Creates a new Rhai context with custom execution limits applied to its engine isolate.
    #[must_use]
    pub fn with_limits(capabilities: CapabilitySet, limits: ExecutionLimits) -> Self {
        let mut engine = rhai::Engine::new();
        limits.apply_to(&mut engine);
        engine.register_type_with_name::<RhaiNativeHandle>("NativeHandle");
        engine.register_type_with_name::<RhaiSingleton>("HostSingleton");

        Self {
            capabilities,
            scope: rhai::Scope::new(),
            engine,
            functions: HashMap::new(),
            host_objects: HashMap::new(),
        }
    }

    /// Accesses the underlying Rhai scope immutably.
    #[must_use]
    pub const fn scope(&self) -> &rhai::Scope<'static> {
        &self.scope
    }

    /// Accesses the underlying Rhai scope mutably.
    pub fn scope_mut(&mut self) -> &mut rhai::Scope<'static> {
        &mut self.scope
    }

    /// Accesses the isolated Rhai engine immutably.
    #[must_use]
    pub const fn engine(&self) -> &rhai::Engine {
        &self.engine
    }

    /// Accesses the isolated Rhai engine mutably.
    pub fn engine_mut(&mut self) -> &mut rhai::Engine {
        &mut self.engine
    }

    /// Returns a reference to the registered native host functions.
    #[must_use]
    pub const fn functions(&self) -> &HashMap<String, NativeFn> {
        &self.functions
    }

    /// Returns a reference to registered host objects.
    #[must_use]
    pub const fn host_objects(&self) -> &HashMap<String, HostObject> {
        &self.host_objects
    }

    /// Evaluates script source directly in this context's isolated engine and scope.
    pub fn eval_dynamic(&mut self, script: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        self.engine.eval_with_scope(&mut self.scope, script)
    }

    /// Evaluates compiled AST in this context's isolated engine and scope.
    pub fn eval_ast<T: Clone + Send + Sync + 'static>(
        &mut self,
        ast: &rhai::AST,
    ) -> Result<T, Box<EvalAltResult>> {
        self.engine.eval_ast_with_scope(&mut self.scope, ast)
    }
}

impl ExecutionContext for RhaiContext {
    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    fn register_host_object(&mut self, object: HostObject) -> Result<(), EngineError> {
        let obj_name = object.name().as_str().to_string();

        if object.is_singleton() {
            self.scope
                .set_value(object.name().as_str(), RhaiSingleton::new(obj_name.clone()));
        }

        crate::application::host_dispatcher::HostDispatcher::register_host_object(
            &mut self.engine,
            &object,
            &self.capabilities,
        )?;

        self.host_objects.insert(obj_name, object);
        Ok(())
    }

    fn register_fn(&mut self, name: Identifier, f: NativeFn) -> Result<(), EngineError> {
        self.functions
            .insert(name.as_str().to_string(), Arc::clone(&f));
        Ok(())
    }

    fn set_variable(&mut self, name: Identifier, value: EngineValue) -> Result<(), EngineError> {
        let dyn_val = engine_value_to_dynamic(value);
        self.scope.set_value(name.as_str(), dyn_val);
        Ok(())
    }

    fn get_variable(&self, name: &Identifier) -> Result<Option<EngineValue>, EngineError> {
        let dyn_opt = self.scope.get_value::<Dynamic>(name.as_str());
        match dyn_opt {
            Some(d) => dynamic_to_engine_value(&d).map(Some),
            None => Ok(None),
        }
    }

    fn call_function(
        &mut self,
        name: &Identifier,
        args: &[EngineValue],
    ) -> Result<EngineValue, EngineError> {
        if let Some(native_fn) = self.functions.get(name.as_str()) {
            let f = Arc::clone(native_fn);
            return f(self, args);
        }

        if let Some((ns, method)) = name.as_str().split_once('.') {
            if let Some(host_obj) = self.host_objects.get(ns) {
                if let Some(cap) = host_obj.required_capability() {
                    if !self.capabilities.contains(cap) {
                        return Err(EngineError::PermissionDenied(format!("{cap:?}")));
                    }
                }
                for (m_name, m_fn) in host_obj.methods() {
                    if m_name.as_str() == method {
                        return m_fn(None, args);
                    }
                }
            }
        }

        if let Some(host_obj) = self.host_objects.get(name.as_str()) {
            if let Some(method) = host_obj.get_method(name) {
                let m = Arc::clone(method);
                return m(None, args);
            }
        }

        Err(EngineError::FunctionNotFound(name.as_str().to_string()))
    }

    fn reset_scope(&mut self) -> Result<(), EngineError> {
        self.scope.clear();
        for (name, obj) in &self.host_objects {
            if obj.is_singleton() {
                self.scope
                    .set_value(name.as_str(), RhaiSingleton::new(name.clone()));
            }
        }
        Ok(())
    }
}
