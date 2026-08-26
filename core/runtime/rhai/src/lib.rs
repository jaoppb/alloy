#![forbid(unsafe_code)]

//! # Rhai Runtime Backend (`core/runtime/rhai`)
//!
//! Concrete Rhai scripting engine implementing `RuntimeEngine` and `ExecutionContext` from `core/engine`.
//! Provides memory and CPU instruction execution limits, preventing denial of service and infinite loops (PRD-002, C-02, C-04).

pub mod application;
pub mod domain;

pub use application::context::RhaiContext;
pub use application::engine::RhaiEngine;
pub use domain::limits::ExecutionLimits;
pub use domain::marshaling::{
    dynamic_to_engine_value, engine_value_to_dynamic, rhai_error_to_engine_error,
};
