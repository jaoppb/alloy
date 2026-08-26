use crate::application::conversion::FromEngineValue;
use crate::domain::capability::CapabilitySet;
use crate::domain::error::EngineError;
use crate::domain::host_object::HostObject;
use crate::domain::identifier::Identifier;
use crate::domain::value::EngineValue;
use std::sync::Arc;

/// Type alias for native host functions registered into an `ExecutionContext`.
pub type NativeFn = Arc<
    dyn Fn(&mut dyn ExecutionContext, &[EngineValue]) -> Result<EngineValue, EngineError>
        + Send
        + Sync,
>;

/// Trait defining the contract for an active, sandboxed script isolate.
pub trait ExecutionContext: Send + Sync {
    /// Returns the capability set governing this execution context isolate (PRD-003).
    fn capabilities(&self) -> &CapabilitySet;

    /// Registers a declarative host object or namespace into this isolate (ADR-0012, N-01).
    ///
    /// # Errors
    /// Returns `EngineError` if registration fails.
    fn register_host_object(&mut self, object: HostObject) -> Result<(), EngineError>;

    /// Registers a native Rust function callback into this isolate.
    ///
    /// # Errors
    /// Returns `EngineError` if the function cannot be registered.
    fn register_fn(&mut self, name: Identifier, f: NativeFn) -> Result<(), EngineError>;

    /// Sets a script-accessible variable in the current isolate scope.
    ///
    /// # Errors
    /// Returns `EngineError` if the variable cannot be assigned.
    fn set_variable(&mut self, name: Identifier, value: EngineValue) -> Result<(), EngineError>;

    /// Gets a variable value from the current isolate scope.
    ///
    /// # Errors
    /// Returns `EngineError` if variable lookup fails.
    fn get_variable(&self, name: &Identifier) -> Result<Option<EngineValue>, EngineError>;

    /// Invokes a registered script or native function by name.
    ///
    /// # Errors
    /// Returns `EngineError::FunctionNotFound` if no function matches, or any runtime error thrown by the function.
    fn call_function(
        &mut self,
        name: &Identifier,
        args: &[EngineValue],
    ) -> Result<EngineValue, EngineError>;

    /// Resets the script variable scope while retaining registered native functions (PRD-001 §5.2).
    ///
    /// # Errors
    /// Returns `EngineError` if scope cleanup fails.
    fn reset_scope(&mut self) -> Result<(), EngineError>;
}

/// Abstract runtime engine trait decoupling domain crates from concrete script engines (PRD-002, C-01).
pub trait RuntimeEngine: Send + Sync {
    /// The isolated execution context type managed by this runtime.
    type Context: ExecutionContext;

    /// The compiled script representation (e.g. AST or bytecode).
    type CompiledScript: Send + Sync;

    /// The structured error type returned by this runtime.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Creates an isolated execution context granted a specific set of capabilities (PRD-002:40, PRD-003).
    ///
    /// # Errors
    /// Returns `Self::Error` if context initialization fails.
    fn create_context(&self, capabilities: CapabilitySet) -> Result<Self::Context, Self::Error>;

    /// Pre-compiles script source code into a reusable compiled script artifact (PRD-002:41, PRD-004).
    ///
    /// # Errors
    /// Returns `Self::Error` on syntax or compilation failure.
    fn compile(&self, script_source: &str) -> Result<Self::CompiledScript, Self::Error>;

    /// Evaluates a script source within an execution context, marshaling the result to `T` (PRD-002:42).
    ///
    /// # Errors
    /// Returns `Self::Error` on execution limit, permission denial, runtime error, or type mismatch.
    fn eval<T: FromEngineValue>(
        &self,
        context: &mut Self::Context,
        script: &str,
    ) -> Result<T, Self::Error>;
}

/// Port defining filesystem watching capabilities decoupled from concrete I/O libraries (C-29).
pub trait FileWatchPort: Send + Sync {
    /// Starts watching a path, executing callback on change.
    ///
    /// # Errors
    /// Returns `EngineError` if watching fails to initialize.
    fn watch(
        &mut self,
        path: &std::path::Path,
        callback: Box<dyn Fn(std::path::PathBuf) + Send + Sync + 'static>,
    ) -> Result<(), EngineError>;
}
