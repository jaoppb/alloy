use std::fmt;

/// Strongly typed error enum for script execution, capability violations, and type mappings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// Execution exceeded instruction counter or memory limit (PRD-002:78-79).
    ExecutionLimitExceeded(String),
    /// Script attempted an action not authorized by its CapabilitySet (PRD-003:30-49).
    PermissionDenied(String),
    /// Mismatch between expected and actual EngineValue types.
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    /// Script syntax error with line/column or message context.
    SyntaxError(String),
    /// Runtime failure during script execution.
    RuntimeError(String),
    /// Attempted invocation of an unregistered function.
    FunctionNotFound(String),
    /// Attempted lookup of an unassigned variable.
    VariableNotFound(String),
    /// Identifier failed validation (e.g. empty or invalid characters).
    InvalidIdentifier(String),
    /// A script execution panicked but was trapped by the fault isolation boundary (PRD-003:64-70, C-09).
    PanicTrapped(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExecutionLimitExceeded(msg) => write!(f, "Execution limit exceeded: {msg}"),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {msg}"),
            Self::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {expected}, found {found}")
            }
            Self::SyntaxError(msg) => write!(f, "Syntax error: {msg}"),
            Self::RuntimeError(msg) => write!(f, "Runtime error: {msg}"),
            Self::FunctionNotFound(name) => write!(f, "Function not found: {name}"),
            Self::VariableNotFound(name) => write!(f, "Variable not found: {name}"),
            Self::InvalidIdentifier(msg) => write!(f, "Invalid identifier: {msg}"),
            Self::PanicTrapped(msg) => write!(f, "Script panic trapped: {msg}"),
        }
    }
}

impl std::error::Error for EngineError {}
