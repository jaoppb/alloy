use crate::domain::marshaling::{dynamic_to_engine_value, engine_value_to_dynamic};
use engine::{CapabilitySet, EngineError, EngineValue, ExecutionContext, Identifier, NativeFn};
use std::collections::HashMap;

/// An isolated Rhai execution context holding its own `rhai::Scope` and capability permissions.
pub struct RhaiContext {
    capabilities: CapabilitySet,
    scope: rhai::Scope<'static>,
    functions: HashMap<String, NativeFn>,
}

impl RhaiContext {
    /// Creates a new Rhai context with the given capability set.
    #[must_use]
    pub fn new(capabilities: CapabilitySet) -> Self {
        Self {
            capabilities,
            scope: rhai::Scope::new(),
            functions: HashMap::new(),
        }
    }

    /// Accesses the underlying Rhai scope immutably.
    #[must_use]
    pub fn scope(&self) -> &rhai::Scope<'static> {
        &self.scope
    }

    /// Accesses the underlying Rhai scope mutably.
    pub fn scope_mut(&mut self) -> &mut rhai::Scope<'static> {
        &mut self.scope
    }

    /// Returns a reference to the registered native host functions.
    #[must_use]
    pub fn functions(&self) -> &HashMap<String, NativeFn> {
        &self.functions
    }
}

impl ExecutionContext for RhaiContext {
    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    fn register_fn(&mut self, name: Identifier, f: NativeFn) -> Result<(), EngineError> {
        self.functions.insert(name.as_str().to_string(), f);
        Ok(())
    }

    fn set_variable(&mut self, name: Identifier, value: EngineValue) -> Result<(), EngineError> {
        let dyn_val = engine_value_to_dynamic(value);
        self.scope.set_value(name.as_str(), dyn_val);
        Ok(())
    }

    fn get_variable(&self, name: &Identifier) -> Result<Option<EngineValue>, EngineError> {
        let Some(val) = self.scope.get_value::<rhai::Dynamic>(name.as_str()) else {
            return Ok(None);
        };
        dynamic_to_engine_value(&val).map(Some)
    }

    fn call_function(
        &mut self,
        name: &Identifier,
        args: &[EngineValue],
    ) -> Result<EngineValue, EngineError> {
        let f = self
            .functions
            .get(name.as_str())
            .cloned()
            .ok_or_else(|| EngineError::FunctionNotFound(name.as_str().to_string()))?;

        f(self, args)
    }

    fn reset_scope(&mut self) -> Result<(), EngineError> {
        self.scope.clear();
        Ok(())
    }
}
