use crate::application::context::RhaiContext;
use crate::domain::limits::ExecutionLimits;
use crate::domain::marshaling::{dynamic_to_engine_value, rhai_error_to_engine_error};
use engine::{CapabilitySet, EngineError, FromEngineValue, RuntimeEngine};

/// Concrete Rhai scripting engine implementing `RuntimeEngine` (PRD-002:62-70, C-02).
pub struct RhaiEngine {
    engine: rhai::Engine,
    limits: ExecutionLimits,
}

impl Default for RhaiEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RhaiEngine {
    /// Creates a new Rhai engine with default execution limits.
    #[must_use]
    pub fn new() -> Self {
        let limits = ExecutionLimits::new();
        let mut engine = rhai::Engine::new();
        limits.apply_to(&mut engine);
        Self { engine, limits }
    }

    /// Configures the engine with custom execution limits.
    #[must_use]
    pub fn with_limits(mut self, limits: ExecutionLimits) -> Self {
        limits.apply_to(&mut self.engine);
        self.limits = limits;
        self
    }

    /// Accesses the underlying execution limits.
    #[must_use]
    pub const fn limits(&self) -> &ExecutionLimits {
        &self.limits
    }

    /// Accesses the underlying Rhai engine immutably.
    #[must_use]
    pub const fn raw_engine(&self) -> &rhai::Engine {
        &self.engine
    }

    /// Accesses the underlying Rhai engine mutably.
    pub fn raw_engine_mut(&mut self) -> &mut rhai::Engine {
        &mut self.engine
    }
}

impl RuntimeEngine for RhaiEngine {
    type Context = RhaiContext;
    type CompiledScript = rhai::AST;
    type Error = EngineError;

    fn create_context(&self, capabilities: CapabilitySet) -> Result<Self::Context, Self::Error> {
        Ok(RhaiContext::new(capabilities))
    }

    fn compile(&self, script_source: &str) -> Result<Self::CompiledScript, Self::Error> {
        self.engine
            .compile(script_source)
            .map_err(|err| EngineError::SyntaxError(err.to_string()))
    }

    fn eval<T: FromEngineValue>(
        &self,
        context: &mut Self::Context,
        script: &str,
    ) -> Result<T, Self::Error> {
        let dyn_result = self
            .engine
            .eval_with_scope::<rhai::Dynamic>(context.scope_mut(), script)
            .map_err(|err| rhai_error_to_engine_error(*err))?;

        let engine_val = dynamic_to_engine_value(&dyn_result)?;
        T::from_engine_value(&engine_val)
    }
}
