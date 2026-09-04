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
//! `EngineValue` / [`EngineError`]. The `dyn`-dispatch companion required by
//! ADR-0011 item 2 lives in [`application::dyn_bridge`] ([`DynRuntimeEngine`] /
//! [`DynExecutionContext`] / [`DynCompiledScript`] + [`eval_typed`], ADR-0013,
//! v0.2 F6): `Box<dyn DynRuntimeEngine>` is a usable engine handle. Blanket
//! impls give every `RuntimeEngine` the companion for free.
//!
//! ## Contract record
//!
//! This crate is the `RuntimeEngine` port under the ADR-0011 Replaceable Port
//! Contract. `docs/architecture/runtime-engine-port-contract.md` records the
//! state of all seven contract items (variation model, threat model, lifecycle
//! and concurrency, conformance, freeze point). PRD-002 §2.1–2.2 hold the
//! variation and threat models.

#![forbid(unsafe_code)]
// Every fallible method returns the one `EngineError` (ADR-0011 item 4) and
// documents its failure modes in prose; a `# Errors` heading on each would only
// repeat that.
#![allow(clippy::missing_errors_doc)]

pub mod application;
pub mod conformance;
pub mod domain;

/// Schema version of this port's boundary surface — the `EngineValue` /
/// `EngineError` / `TypeRegistration` shapes and the trait method contracts
/// taken together (ADR-0011 items 3 and 7).
///
/// Bump this on **any** change that an out-of-tree adapter or consumer could
/// observe as breaking (a new `EngineValue` variant, a new `EngineError`
/// note to PRD-002. `1` was frozen at roadmap point F1; `2` is the review
/// response & v0.2 I1 — `FunctionName` / `VariableName` on binding and scope
/// methods, `SourceLocation` as an enum over `Line` / `Column`, and additive
/// `EngineError::Dom { operation, reason }` (see PRD-002 §4.2). `3` is v0.5
/// Phase EE — `EngineError::Subsystem { subsystem: SubsystemName, operation,
/// reason }` generalizes `Dom` so `Css` / `Graphics` / `Network` / `Window`
/// don't each need their own variant; `Dom` is `#[deprecated]`, not removed
/// (full removal is a v0.7 schema-4 change, see PRD-002 §4.5).
pub const PORT_SCHEMA_VERSION: u32 = 3;

pub use application::{
    DynCompiledScript, DynExecutionContext, DynRuntimeEngine, ExecutionContext, FromEngineValue,
    IntoEngineValue, NativeFn, RuntimeEngine,
    engine_type::{EngineType, TypeRegistration},
    eval_typed,
    function::{Arity, EngineFunction},
    native_fn,
};
pub use domain::{
    capability::{Capability, CapabilitySet, profiles},
    error::{EngineError, SubsystemName},
    function_name::FunctionName,
    limits::{ExecutionLimit, ExecutionLimits},
    source::{Column, Line, SourceLocation},
    value::{EngineValue, ValueKind},
    variable_name::VariableName,
};
