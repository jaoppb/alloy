//! # `rhai-runtime` — the Rhai adapter for the `engine` port
//!
//! One of the two crates that name a `rhai` type (the other is the sibling
//! `rhai-bindings`, which holds the domain-coupled bridges). This crate owns the
//! `engine`-port implementation and names **no** domain crate (v0.5 report
//! §2.12). It implements [`engine::RuntimeEngine`] / [`engine::ExecutionContext`]
//! on top of [`rhai::Engine`] / [`rhai::Scope`], translating in both directions:
//!
//! - [`engine::EngineValue`] ⇄ [`rhai::Dynamic`] (`infrastructure::marshal`) — a
//!   pure `match`, no raw pointer reads (PRD-002 invariant 1).
//! - `rhai::ParseError` / `rhai::EvalAltResult` → [`engine::EngineError`]
//!   (`infrastructure::error_map`) — one typed error out, source location kept
//!   (PRD-002:81, ADR-0011 item 4).
//!
//! Execution ceilings ([`engine::ExecutionLimits`]) map onto
//! `set_max_operations` / `set_max_call_levels` / `set_max_expr_depths` plus a
//! wall-clock `on_progress` guard; a breach becomes
//! [`engine::EngineError::ExecutionLimitExceeded`] (mechanism of C-04). Script
//! and native-function panics are trapped with [`std::panic::catch_unwind`] and
//! surface as [`engine::EngineError::ScriptPanic`] — the host process never
//! aborts (PRD-003:79, mechanism of C-09).

#![forbid(unsafe_code)]
// Fallible methods return `engine::EngineError` and document their failures in
// prose; a `# Errors` heading on each would only repeat that.
#![allow(clippy::missing_errors_doc)]

mod infrastructure;

pub use infrastructure::{
    GuardedBinding, PanicHookGuard, RhaiCompiledScript, RhaiContext, RhaiEngine,
    install_guarded_table, run_with_fallback, to_eval_error,
};
