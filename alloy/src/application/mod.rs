//! Application layer for the Alloy browser.

pub mod browser_services;
pub mod event_loop;
pub mod image_store;
pub mod navigation;
pub mod paint;
pub mod pipeline;
pub mod runtime_font;
pub mod subresource;

pub use browser_services::BrowserServices;
pub use event_loop::{
    LoopStats, initial_window_attributes, run_browser, run_browser_until,
    run_browser_until_first_frame,
};
pub use image_store::ImageStore;
pub use navigation::navigate;
pub use paint::paint_box_tree;
pub use pipeline::{
    DEFAULT_FONT_SIZE, LinkTarget, RenderOptions, default_runtime_font_provider, render_dom,
    render_dom_with_font_provider, render_dom_with_links, render_html_to_png,
    render_html_with_font_provider, run_render,
};
pub use runtime_font::RuntimeFontProvider;
pub use subresource::{
    ImageRequest, MarkupDiscoverer, StylesheetRequest, SubresourceDiscoverer, SubresourceRequest,
    Subresources,
};
