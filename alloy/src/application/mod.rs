//! Application layer for the Alloy browser.

pub mod browser_app;
pub mod browser_services;
pub mod compositor;
pub mod effects;
pub mod event_loop;
pub mod fetch_channel;
pub mod html_parse;
pub mod keymap;
pub mod loop_stats;
pub mod paint;
pub mod pending_title;
pub mod pipeline;
pub mod ports;
pub mod runtime_font;

pub use browser_app::BrowserApp;
pub use browser_services::BrowserServices;
pub use compositor::{Compositor, Damage};
pub use effects::EffectExecutor;

pub use fetch_channel::{FetchInbox, FetchOutbox, fetch_channel};
pub use keymap::resolve_keymap;
pub use loop_stats::LoopStats;
pub use paint::paint_box_tree;
pub use pipeline::{
    DEFAULT_FONT_SIZE, RenderOptions, default_runtime_font_provider, render_dom,
    render_dom_with_font_provider, render_dom_with_links, render_html_to_png,
    render_html_with_font_provider, run_render,
};
pub use ports::{FetchExecutor, IdlePause, ImageDecoder, KeymapSource};
pub use runtime_font::RuntimeFontProvider;
