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

mod dom_bindings;
mod dom_fallback;

pub use dom_bindings::{NODE_HANDLE_BINDINGS, NodeHandle};
pub use dom_fallback::{bind_dom, minimal_document, run_dom_with_fallback};
