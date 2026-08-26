use thiserror::Error;

/// Strongly typed error enum for script execution, capability violations, and type mappings.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    /// Execution exceeded instruction counter or memory limit (PRD-002:78-79).
    #[error("Execution limit exceeded: {0}")]
    ExecutionLimitExceeded(String),
    /// Script attempted an action not authorized by its CapabilitySet (PRD-003:30-49).
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    /// Mismatch between expected and actual EngineValue types.
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    /// Script syntax error with line/column or message context.
    #[error("Syntax error: {0}")]
    SyntaxError(String),
    /// Runtime failure during script execution.
    #[error("Runtime error: {0}")]
    RuntimeError(String),
    /// Attempted invocation of an unregistered function.
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    /// Attempted lookup of an unassigned variable.
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
    /// Identifier failed validation (e.g. empty or invalid characters).
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
    /// A script execution panicked but was trapped by the fault isolation boundary (PRD-003:64-70, C-09).
    #[error("Script panic trapped: {0}")]
    PanicTrapped(String),
}
