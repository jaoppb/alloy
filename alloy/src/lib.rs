//! Alloy browser library.
//!
//! Provides the headless render pipeline (`render_html_to_png`, `run_render`),
//! display list generation (`paint_box_tree`), the native-window event loop
//! (`run_browser`, v0.5 Phase I4), error types, and logging.

#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod application;
pub mod error;
pub mod logging;

pub use application::paint::paint_box_tree;
pub use application::pipeline::{
    DEFAULT_FONT_SIZE, LinkTarget, RenderOptions, default_runtime_font_provider, render_dom,
    render_dom_with_font_provider, render_dom_with_links, render_html_to_png,
    render_html_with_font_provider, run_render,
};
pub use application::{
    LoopStats, initial_window_attributes, navigate, run_browser, run_browser_until,
    run_browser_until_first_frame,
};
pub use error::AlloyError;
