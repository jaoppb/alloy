//! Application layer for the Alloy browser.

pub mod event_loop;
pub mod navigation;
pub mod paint;
pub mod pipeline;
pub mod subresource;

pub use event_loop::{
    LoopStats, initial_window_attributes, run_browser, run_browser_until,
    run_browser_until_first_frame,
};
pub use navigation::navigate;
pub use paint::paint_box_tree;
pub use pipeline::{
    DEFAULT_FONT_SIZE, LinkTarget, RenderOptions, default_runtime_font_provider, render_dom,
    render_dom_with_font_provider, render_dom_with_links, render_html_to_png,
    render_html_with_font_provider, run_render,
};
