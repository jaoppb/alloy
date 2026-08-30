//! [`EngineError`] — the **one** typed error for this port (ADR-0011 item 4).
//!
//! Every adapter maps its native failures into these variants; no adapter error
//! type (`rhai::EvalAltResult`, `rhai::ParseError`, …) ever crosses the seam.
//! Hand-written `Display` / [`std::error::Error`] keep the domain layer free of
//! a derive-macro dependency.

use std::fmt;

use crate::domain::capability::Capability;
use crate::domain::limits::ExecutionLimit;
use crate::domain::source::SourceLocation;

/// A failure raised while compiling or running a muscle script, or while moving
/// a value across the boundary.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EngineError {
    /// The source did not compile. `location` is present when the backend
    /// reports a position (PRD-002:81).
    Compilation {
        message: String,
        location: Option<SourceLocation>,
    },

    /// A resource ceiling was hit — the mechanism behind C-04. Which ceiling is
    /// named by [`ExecutionLimit`].
    ExecutionLimitExceeded { limit: ExecutionLimit },

    /// The script attempted an operation its context was not granted
    /// (PRD-003:77, C-07). Carries the capability that was missing.
    PermissionDenied { capability: Capability },

    /// A value crossing the boundary had the wrong shape. `expected` / `found`
    /// are [`crate::domain::value::ValueKind::name`] strings.
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },

    /// A conversion via [`crate::application::FromEngineValue`] failed for a
    /// reason other than a plain shape mismatch (e.g. integer out of range).
    Conversion { message: String },

    /// Registering or invoking a native binding failed (bad name, arity
    /// mismatch, unknown function).
    Binding { message: String },

    /// The script raised a runtime error that is not a limit breach —
    /// unbound variable, failed assertion, thrown value (PRD-002 invariant 4).
    ScriptRuntime {
        message: String,
        location: Option<SourceLocation>,
    },

    /// A native binding or the script itself panicked. Trapped by the adapter so
    /// the host process survives (PRD-003:79, mechanism of C-09).
    ScriptPanic { message: String },
}

impl EngineError {
    #[must_use]
    pub fn compilation(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::Compilation {
            message: message.into(),
            location,
        }
    }

    #[must_use]
    pub const fn execution_limit_exceeded(limit: ExecutionLimit) -> Self {
        Self::ExecutionLimitExceeded { limit }
    }

    #[must_use]
    pub const fn permission_denied(capability: Capability) -> Self {
        Self::PermissionDenied { capability }
    }

    #[must_use]
    pub const fn type_mismatch(expected: &'static str, found: &'static str) -> Self {
        Self::TypeMismatch { expected, found }
    }

    #[must_use]
    pub fn conversion(message: impl Into<String>) -> Self {
        Self::Conversion {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn binding(message: impl Into<String>) -> Self {
        Self::Binding {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn script_runtime(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::ScriptRuntime {
            message: message.into(),
            location,
        }
    }

    #[must_use]
    pub fn script_panic(message: impl Into<String>) -> Self {
        Self::ScriptPanic {
            message: message.into(),
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compilation { message, location } => {
                write!(formatter, "compilation failed")?;
                write_location(formatter, location.as_ref())?;
                write!(formatter, ": {message}")
            }
            Self::ExecutionLimitExceeded { limit } => {
                write!(formatter, "execution limit exceeded: {limit}")
            }
            Self::PermissionDenied { capability } => {
                write!(
                    formatter,
                    "permission denied: missing capability {capability:?}"
                )
            }
            Self::TypeMismatch { expected, found } => {
                write!(
                    formatter,
                    "type mismatch: expected {expected}, found {found}"
                )
            }
            Self::Conversion { message } => write!(formatter, "conversion failed: {message}"),
            Self::Binding { message } => write!(formatter, "native binding error: {message}"),
            Self::ScriptRuntime { message, location } => {
                write!(formatter, "script error")?;
                write_location(formatter, location.as_ref())?;
                write!(formatter, ": {message}")
            }
            Self::ScriptPanic { message } => write!(formatter, "script panic (trapped): {message}"),
        }
    }
}

fn write_location(
    formatter: &mut fmt::Formatter<'_>,
    location: Option<&SourceLocation>,
) -> fmt::Result {
    match location {
        Some(position) => write!(formatter, " at {position}"),
        None => Ok(()),
    }
}

impl std::error::Error for EngineError {}
