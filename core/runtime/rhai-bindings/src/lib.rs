//! # `rhai-bindings` — domain-crate bridges for the Rhai backend
//!
//! [`rhai_runtime`] implements the [`engine`] port and names no domain crate
//! (v0.5 report §2.12). This crate holds the bridges that *do* need a domain
//! type: the scriptable-DOM [`NodeHandle`] (roadmap I1) and the DOM-specific
//! instances of the C-09 fallback skeleton. Later v0.5 phases add the CSS,
//! graphics, network and window bindings and the embedded muscle-policy cycle
//! here alongside them.
//!
//! The one-way edge holds: a `rhai` type never reaches `core/dom`, and
//! `rhai-runtime` never names `dom`.

#![forbid(unsafe_code)]
// Fallible functions return `engine::EngineError` and document their failures in
// prose; a `# Errors` heading on each would only repeat that.
#![allow(clippy::missing_errors_doc)]

mod css_bindings;
mod dom_bindings;
mod dom_fallback;
mod net_bindings;
mod window_bindings;

pub use css_bindings::{
    ScriptCascadeResolver, SnapshotHandle, StyledTreeHandle, register_css_bindings,
};
pub use dom_bindings::{NODE_HANDLE_BINDINGS, NodeHandle};
pub use dom_fallback::{bind_dom, minimal_document, run_dom_with_fallback};
pub use net_bindings::{NETWORK_BINDINGS, ScriptRequestPolicy, register_net_bindings};
pub use window_bindings::{WINDOW_BINDINGS, register_window_bindings, run_ui_event_with_fallback};

/// Embedded default UI policy script (`scripts/default_ui.rhai`).
pub const DEFAULT_UI_SCRIPT: &str = include_str!("../../../../scripts/default_ui.rhai");

/// Embedded default network policy script (`scripts/default_network.rhai`).
pub const DEFAULT_NETWORK_SCRIPT: &str = include_str!("../../../../scripts/default_network.rhai");

/// Embedded cascade policy script (`scripts/cascade.rhai`).
pub const DEFAULT_CASCADE_SCRIPT: &str = include_str!("../../../../scripts/cascade.rhai");
