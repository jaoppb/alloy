use crate::application::conversion::FromEngineValue;
use crate::application::ports::{ExecutionContext, NativeFn, RuntimeEngine};
use crate::domain::capability::CapabilitySet;
use crate::domain::error::EngineError;
use crate::domain::identifier::Identifier;
use crate::domain::value::EngineValue;
use std::collections::HashMap;

/// An in-memory, mock execution context for unit and integration tests.
#[derive(Default)]
pub struct MockContext {
    capabilities: CapabilitySet,
    variables: HashMap<String, EngineValue>,
    functions: HashMap<String, NativeFn>,
}

impl MockContext {
    /// Creates a new mock context with explicit capabilities.
    #[must_use]
    pub fn new(capabilities: CapabilitySet) -> Self {
        Self {
            capabilities,
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }
}

impl ExecutionContext for MockContext {
    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    fn register_fn(&mut self, name: Identifier, f: NativeFn) -> Result<(), EngineError> {
        self.functions.insert(name.as_str().to_string(), f);
        Ok(())
    }

    fn set_variable(&mut self, name: Identifier, value: EngineValue) -> Result<(), EngineError> {
        self.variables.insert(name.as_str().to_string(), value);
        Ok(())
    }

    fn get_variable(&self, name: &Identifier) -> Result<Option<EngineValue>, EngineError> {
        Ok(self.variables.get(name.as_str()).cloned())
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
        self.variables.clear();
        Ok(())
    }
}

/// A lightweight, in-memory mock engine that implements `RuntimeEngine` (C-01, C-05).
#[derive(Debug, Default, Clone, Copy)]
pub struct MockEngine;

impl MockEngine {
    /// Creates a new mock engine instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl RuntimeEngine for MockEngine {
    type Context = MockContext;
    type CompiledScript = String;
    type Error = EngineError;

    fn create_context(&self, capabilities: CapabilitySet) -> Result<Self::Context, Self::Error> {
        Ok(MockContext::new(capabilities))
    }

    fn compile(&self, script_source: &str) -> Result<Self::CompiledScript, Self::Error> {
        if script_source.is_empty() || script_source.contains("SYNTAX_ERROR") {
            return Err(EngineError::SyntaxError(
                "Script syntax error detected".to_string(),
            ));
        }
        Ok(script_source.to_string())
    }

    fn eval<T: FromEngineValue>(
        &self,
        context: &mut Self::Context,
        script: &str,
    ) -> Result<T, Self::Error> {
        let trimmed = script.trim();

        // 1. Literal integer evaluation
        if let Ok(num) = trimmed.parse::<i64>() {
            let val = EngineValue::Int(num);
            return T::from_engine_value(&val);
        }

        // 2. Literal boolean evaluation
        if let Ok(b) = trimmed.parse::<bool>() {
            let val = EngineValue::Bool(b);
            return T::from_engine_value(&val);
        }

        // 3. Literal float evaluation
        if let Ok(f) = trimmed.parse::<f64>() {
            let val = EngineValue::Float(f);
            return T::from_engine_value(&val);
        }

        // 4. Variable lookup
        if let Some(val) = context.variables.get(trimmed) {
            return T::from_engine_value(val);
        }

        // 5. Function call expression: "fn_name()"
        if let Some(stripped) = trimmed.strip_suffix("()") {
            let id = Identifier::new(stripped)?;
            let res = context.call_function(&id, &[])?;
            return T::from_engine_value(&res);
        }

        // 6. Quoted string literal
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            let val = EngineValue::String(inner.to_string());
            return T::from_engine_value(&val);
        }

        Err(EngineError::RuntimeError(format!(
            "MockEngine cannot evaluate expression: {trimmed}"
        )))
    }
}
