//! Outermost layer (ADR-0010 §1): the concrete Rhai adapter. Everything that
//! touches a `rhai::*` type lives here.

mod context;
mod engine;
mod error_map;
mod marshal;
mod native;

pub use context::{RhaiCompiledScript, RhaiContext};
pub use engine::RhaiEngine;
