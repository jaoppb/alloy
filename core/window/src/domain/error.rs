//! [`WindowError`] — the **one** typed error for this port (`ADR-0011` item 4).
//!
//! `thiserror`, not a hand-written `Display`: the manual carve-out of
//! `ADR-0015` applies only to `core/engine`; this crate follows `core/dom`,
//! `core/css`, `core/graphics` and `core/network` (correction at the top of
//! the v0.5 plan). Every variant carries the [`WindowOperation`] that was
//! attempted, and every variant that can be tied to an existing window
//! carries its [`WindowId`] — together, the location metadata of
//! `ADR-0011:93-95` for this port.

use core::fmt;

use crate::domain::attributes::WindowId;

/// A failure raised while creating, pumping events for, or presenting to a
/// window.
#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowError {
    /// `operation` was attempted before
    /// [`WindowSystem::create_window`](crate::application::ports::WindowSystem::create_window)
    /// ever succeeded — there is no window yet to operate on.
    #[error("{operation} was attempted before any window was created")]
    NoWindowYet { operation: WindowOperation },

    /// A window could not be created at all — no [`WindowId`] exists to name.
    #[error("the window could not be created — {reason}")]
    CreationFailed { reason: String },

    /// An operation on an already-existing window failed at the OS or
    /// backend level.
    #[error("{operation} failed for {window} — {reason}")]
    OperationFailed {
        window: WindowId,
        operation: WindowOperation,
        reason: String,
    },

    /// `pump_events` was called after the loop already reported
    /// [`PumpStatus::Exit`](crate::application::ports::PumpStatus::Exit). No
    /// further call is meaningful.
    #[error("the window system's event loop has already exited")]
    EventLoopExited,
}

impl WindowError {
    /// `operation` was attempted with no window created yet.
    #[must_use]
    pub const fn no_window_yet(operation: WindowOperation) -> Self {
        Self::NoWindowYet { operation }
    }

    /// A window that could not be created.
    #[must_use]
    pub fn creation_failed(reason: impl Into<String>) -> Self {
        Self::CreationFailed {
            reason: reason.into(),
        }
    }

    /// An operation on `window` that failed at the OS or backend level.
    #[must_use]
    pub fn operation_failed(
        window: WindowId,
        operation: WindowOperation,
        reason: impl Into<String>,
    ) -> Self {
        Self::OperationFailed {
            window,
            operation,
            reason: reason.into(),
        }
    }
}

/// A method on [`WindowSystem`](crate::application::ports::WindowSystem) or
/// [`Presenter`](crate::application::ports::Presenter).
///
/// Named so an error can say what was attempted without carrying a free-form
/// string — mirrors `graphics::FrameOperation` and `network::ProtocolPhase`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WindowOperation {
    CreateWindow,
    PumpEvents,
    Present,
}

impl WindowOperation {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CreateWindow => "create_window",
            Self::PumpEvents => "pump_events",
            Self::Present => "present",
        }
    }
}

impl fmt::Display for WindowOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
