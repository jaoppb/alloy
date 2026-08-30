//! Outermost layer (ADR-0010 §1): the concrete Rhai adapter. Everything that
//! touches a `rhai::*` type lives here.

mod context;
mod dom_bindings;
mod engine;
mod error_map;
mod fallback;
mod marshal;
mod native;
mod sandbox;

pub use context::{RhaiCompiledScript, RhaiContext};
pub use dom_bindings::{NODE_HANDLE_BINDINGS, NodeHandle};
pub use engine::RhaiEngine;
pub use fallback::{PanicHookGuard, minimal_document, run_with_fallback};
pub use sandbox::{GuardedBinding, install_guarded_table};
