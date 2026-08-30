//! # `engine` — the abstract runtime-engine port
//!
//! This crate is the **Skeleton-side contract** for browser *muscle* scripting
//! (ADR-0003). It defines *what* a script engine must do and *nothing* about
//! *how*: it names no interpreter, links no interpreter, and — proven in CI by
//! the `no-engine` job — has no interpreter anywhere in its dependency graph
//! (ADR-0002:49, ADR-0011 item 2).
//!
//! ## Layout (ADR-0010 §1)
//!
//! - [`domain`] — zero-I/O value objects: [`EngineValue`], [`EngineError`],
//!   [`Capability`] / [`CapabilitySet`], [`ExecutionLimits`], [`SourceLocation`].
//! - [`application`] — the ports themselves: [`RuntimeEngine`],
//!   [`ExecutionContext`], and the conversion traits [`IntoEngineValue`] /
//!   [`FromEngineValue`], plus [`EngineFunction`] and [`EngineType`].
//! - [`conformance`] — a backend-agnostic test suite every adapter must pass
//!   (ADR-0011 item 6). `core/runtime/rhai` and the in-repo `MockEngine`
//!   reference adapter both run it.
//!
//! ## Boundary rule
//!
//! Every value that crosses the seam is an [`EngineValue`]; every failure is an
//! [`EngineError`]. No adapter type (`rhai::Dynamic`, `rhai::EvalAltResult`, …)
//! ever appears in a signature here (ADR-0011 items 3–4).
//!
//! ## Object-safety
//!
//! The PRD-002 sugar methods (`eval::<T>`, `register_fn`, `set_variable::<V>`,
//! `call_function::<T>`, `register_type::<T>`) are **provided** methods layered
//! over a small set of object-safe required methods that speak only
//! `EngineValue` / [`EngineError`]. A `dyn`-dispatch companion port is scheduled
//! for v0.2 (roadmap I1 / ADR-0013); until then consumers monomorphise with
//! `fn run<E: RuntimeEngine>(…)`.
//!
//! ## Contract record
//!
//! This crate is the `RuntimeEngine` port under the ADR-0011 Replaceable Port
//! Contract. `docs/architecture/runtime-engine-port-contract.md` records the
//! state of all seven contract items (variation model, threat model, lifecycle
//! and concurrency, conformance, freeze point). PRD-002 §2.1–2.2 hold the
//! variation and threat models.

#![forbid(unsafe_code)]

pub mod application;
pub mod conformance;
pub mod domain;

/// Schema version of this port's boundary surface — the `EngineValue` /
/// `EngineError` / `TypeRegistration` shapes and the trait method contracts
/// taken together (ADR-0011 items 3 and 7).
///
/// Bump this on **any** change that an out-of-tree adapter or consumer could
/// observe as breaking (a new `EngineValue` variant, a new `EngineError`
/// variant with new meaning, a changed method signature), and add a migration
/// note to PRD-002.
///
/// - `1` — the surface frozen at roadmap point F1 (v0.1).
/// - `2` — v0.2 F6/I1: added `EngineError::Dom { operation, reason }`. Additive
///   for an out-of-tree adapter (it already needs a `_` arm on the
///   `#[non_exhaustive]` enum); a consumer that exhaustively matched
///   `EngineError` must add a `Dom` arm. See PRD-002 §5.1.
pub const PORT_SCHEMA_VERSION: u32 = 2;

pub use application::{
    ExecutionContext, FromEngineValue, IntoEngineValue, NativeFn, RuntimeEngine,
    engine_type::{EngineType, TypeRegistration},
    function::{Arity, EngineFunction},
};
pub use domain::{
    capability::{Capability, CapabilitySet, profiles},
    error::EngineError,
    limits::{ExecutionLimit, ExecutionLimits},
    source::SourceLocation,
    value::{EngineValue, ValueKind},
};
