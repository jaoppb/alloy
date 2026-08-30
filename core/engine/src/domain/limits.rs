//! Execution ceilings (PRD-002 §4.3, ADR-0004). Engine-agnostic *mechanism*
//! config: the numbers live in a struct, never hard-coded in an adapter's eval
//! path (v0.1 report decision 2.6). `core/runtime/rhai` (F2) maps these onto
//! `Engine::set_max_operations` / `set_max_call_levels` / `set_max_expr_depths`
//! and a wall-clock guard; a breach becomes
//! [`EngineError::ExecutionLimitExceeded`][crate::EngineError] — the mechanism of C-04.

use std::fmt;
use std::time::Duration;

/// Which ceiling a script hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExecutionLimit {
    /// Instruction / operation counter (the `while true {}` guard).
    Operations,
    /// Nested function-call depth.
    CallDepth,
    /// Expression nesting depth (parser recursion).
    ExpressionDepth,
    /// Wall-clock budget for a single evaluation.
    Duration,
}

impl fmt::Display for ExecutionLimit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Operations => "operation count",
            Self::CallDepth => "call depth",
            Self::ExpressionDepth => "expression depth",
            Self::Duration => "time budget",
        };
        formatter.write_str(text)
    }
}

/// The full set of ceilings applied to every evaluation in a context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionLimits {
    max_operations: u64,
    max_call_depth: u16,
    max_expression_depth: u16,
    max_duration: Duration,
}

impl ExecutionLimits {
    /// Conservative defaults suitable for a script whose author is trusted but
    /// fallible (the muscle-script threat model of PRD-003 §2): generous enough
    /// for real UI/pipeline logic, tight enough that a runaway loop trips in
    /// well under a second.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            max_operations: 10_000_000,
            max_call_depth: 64,
            max_expression_depth: 128,
            max_duration: Duration::from_secs(1),
        }
    }

    /// Replace the operation ceiling (builder style, ADR-0010 rule 5 allows
    /// chains).
    #[must_use]
    pub const fn with_max_operations(mut self, max_operations: u64) -> Self {
        self.max_operations = max_operations;
        self
    }

    #[must_use]
    pub const fn with_max_call_depth(mut self, max_call_depth: u16) -> Self {
        self.max_call_depth = max_call_depth;
        self
    }

    #[must_use]
    pub const fn with_max_expression_depth(mut self, max_expression_depth: u16) -> Self {
        self.max_expression_depth = max_expression_depth;
        self
    }

    #[must_use]
    pub const fn with_max_duration(mut self, max_duration: Duration) -> Self {
        self.max_duration = max_duration;
        self
    }

    #[must_use]
    pub const fn max_operations(&self) -> u64 {
        self.max_operations
    }

    #[must_use]
    pub const fn max_call_depth(&self) -> u16 {
        self.max_call_depth
    }

    #[must_use]
    pub const fn max_expression_depth(&self) -> u16 {
        self.max_expression_depth
    }

    #[must_use]
    pub const fn max_duration(&self) -> Duration {
        self.max_duration
    }
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self::strict()
    }
}
