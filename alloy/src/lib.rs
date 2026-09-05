//! Alloy browser library.
//!
//! Provides the headless render pipeline (`render_html_to_png`, `run_render`),
//! display list generation (`paint_box_tree`), error types, and logging.

#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod application;
pub mod error;
pub mod logging;

pub use application::paint::paint_box_tree;
pub use application::pipeline::{RenderOptions, render_html_to_png, run_render};
pub use error::AlloyError;
