#![forbid(unsafe_code)]

//! # Alloy Core Engine Application (`alloy`)
//!
//! Orchestrates the browser pipeline, script loading, XDG multi-version isolation,
//! and provides the headless rendering CLI.

pub mod error;
pub mod pipeline;
pub mod xdg_scripts;

pub use error::AlloyCliError;
pub use pipeline::{DEFAULT_PIPELINE_SCRIPT, execute_render, extract_inline_style, render_frame};
pub use xdg_scripts::{VERSION_FINGERPRINT, XdgScriptManager};
