//! Application layer for the Alloy browser.

pub mod paint;
pub mod pipeline;

pub use paint::paint_box_tree;
pub use pipeline::{RenderOptions, render_html_to_png, run_render};
