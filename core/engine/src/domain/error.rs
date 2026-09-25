//! [`EngineError`] — the **one** typed error for this port (ADR-0011 item 4).
//!
//! Every adapter maps its native failures into these variants; no adapter error
//! type (`rhai::EvalAltResult`, `rhai::ParseError`, …) ever crosses the seam.
//! Hand-written `Display` / [`std::error::Error`] keep this port crate free of a
//! derive-macro dependency — the deliberate `thiserror` carve-out of ADR-0015.

use std::fmt;

use crate::domain::capability::Capability;
use crate::domain::limits::ExecutionLimit;
use crate::domain::source::SourceLocation;

/// Which domain crate a [`EngineError::Subsystem`] failure came from.
///
/// One name per subsystem that binds into a muscle script, so the error enum
/// does not grow a bespoke variant per subsystem (v0.5 Phase EE; see
/// `PORT_SCHEMA_VERSION` and PRD-002 §4.5). Naming a subsystem here is not a
/// claim it has production bindings yet — `Css` / `Graphics` / `Network` /
/// `Window` are named ahead of Phase M wiring them, the same way `Capability`
/// has always carried more flags than v0.1 used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubsystemName {
    /// `core/dom`, bound by `core/runtime/rhai-bindings::dom_bindings`.
    Dom,
    /// `core/css`, bound by the scriptable cascade adapter (Phase M).
    Css,
    /// `core/graphics`, bound by the display-list bindings (Phase M).
    Graphics,
    /// `core/network`, bound by the scriptable `RequestPolicy` (Phase M).
    Network,
    /// `core/window`, bound by the UI/window bindings (Phase M).
    Window,
}

impl fmt::Display for SubsystemName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Dom => "dom",
            Self::Css => "css",
            Self::Graphics => "graphics",
            Self::Network => "network",
            Self::Window => "window",
        };
        formatter.write_str(text)
    }
}

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

    /// A DOM operation invoked from a script failed for a reason other than a
    /// missing capability — an invariant violation, a stale node id, a busy
    /// tree. Carries the `operation` that was attempted and a `reason` string
    /// mapped from the domain crate's own error (v0.2 F6/I1; keeps `Binding`
    /// free to mean "bad native-binding name / arity").
    #[deprecated(
        since = "0.5.0",
        note = "use EngineError::Subsystem { subsystem: SubsystemName::Dom, .. } instead (v0.5 Phase EE, PRD-002 §4.5)"
    )]
    Dom { operation: String, reason: String },

    /// A subsystem operation invoked from a script failed for a reason other
    /// than a missing capability — an invariant violation, a stale handle, a
    /// busy resource. Generalizes [`Self::Dom`] to every subsystem a muscle
    /// script binds into, so a new subsystem never needs its own variant (v0.5
    /// Phase EE, PRD-002 §4.5). Carries the `subsystem` the failure came from,
    /// the `operation` that was attempted, and a `reason` mapped from the
    /// domain crate's own error.
    Subsystem {
        subsystem: SubsystemName,
        operation: String,
        reason: String,
    },
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

    /// Delegates to [`Self::subsystem`] with [`SubsystemName::Dom`] — kept so
    /// every existing caller (the DOM bindings) gets the generalized variant
    /// with no source change (v0.5 Phase EE).
    #[must_use]
    pub fn dom(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::subsystem(SubsystemName::Dom, operation, reason)
    }

    #[must_use]
    pub fn subsystem(
        subsystem: SubsystemName,
        operation: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::Subsystem {
            subsystem,
            operation: operation.into(),
            reason: reason.into(),
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
            #[allow(deprecated)]
            // the deprecated variant is still constructible; exhaustive match must still name it
            Self::Dom { operation, reason } => {
                write!(formatter, "dom operation `{operation}` failed: {reason}")
            }
            Self::Subsystem {
                subsystem,
                operation,
                reason,
            } => {
                write!(
                    formatter,
                    "{subsystem} operation `{operation}` failed: {reason}"
                )
            }
        }
    }
}

fn write_location(
    formatter: &mut fmt::Formatter<'_>,
    location: Option<&SourceLocation>,
) -> fmt::Result {
    location.map_or(Ok(()), |position| write!(formatter, " at {position}"))
}

impl std::error::Error for EngineError {}
