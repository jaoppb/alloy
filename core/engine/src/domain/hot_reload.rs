use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Strongly typed debounce duration for script file watching (PRD-004:42).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebounceDuration(Duration);

impl DebounceDuration {
    /// Constructs a debounce duration with custom timing.
    #[must_use]
    pub const fn new(duration: Duration) -> Self {
        Self(duration)
    }

    /// Default debounce duration of 50 milliseconds according to PRD-004 §3.2.
    #[must_use]
    pub const fn default_50ms() -> Self {
        Self(Duration::from_millis(50))
    }

    /// Accesses the inner `Duration`.
    #[must_use]
    pub const fn as_duration(self) -> Duration {
        self.0
    }
}

impl Default for DebounceDuration {
    fn default() -> Self {
        Self::default_50ms()
    }
}

/// The result of an attempted hot-reload operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotReloadStatus {
    /// Script compiled successfully and active AST was swapped atomically.
    Success {
        /// Incremented monotonic version number of the active script.
        version: u64,
    },
    /// Compilation or syntax error encountered; previous active AST was retained.
    CompilationError {
        /// Diagnostic error description.
        error: String,
        /// The active version retained in the slot.
        previous_version: u64,
    },
    /// Script content was identical to active version; reload skipped.
    Unchanged,
}

/// Notification event emitted when a watched script file is updated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadEvent {
    pub path: PathBuf,
    pub timestamp: Instant,
}
