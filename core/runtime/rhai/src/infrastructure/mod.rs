//! Outermost layer (ADR-0010 §1): the concrete Rhai adapter. Everything that
//! touches a `rhai::*` type lives here. Domain-coupled bridges (DOM, and later
//! CSS/graphics/network/window) live in the sibling `rhai-bindings` crate, so
//! this crate names no domain type (v0.5 report §2.12).

mod context;
mod engine;
mod error_map;
mod fallback;
mod marshal;
mod native;
mod sandbox;

pub use context::{RhaiCompiledScript, RhaiContext};
pub use engine::RhaiEngine;
pub use fallback::{PanicHookGuard, run_with_fallback};
pub use native::to_eval_error;
pub use sandbox::{GuardedBinding, install_guarded_table};
