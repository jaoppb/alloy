//! Render integration test verifying that nested lists do not flatten (issue #78).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use alloy::{RenderOptions, render_html_to_png};

#[test]
fn nested_list_renders_differently_from_flattened_list() {
    let options = RenderOptions::new(400, 200);
    let nested_html = "<!doctype html><html><head><style>body{margin:0;background:#fff}</style></head><body><ul><li>aa<ul><li>bb</li></ul></li></ul></body></html>";
    let flattened_html = "<!doctype html><html><head><style>body{margin:0;background:#fff}</style></head><body><ul><li>aa<ul></ul></li><li>bb</li></ul></body></html>";

    let nested_png = render_html_to_png(nested_html, &options).expect("nested renders");
    let flattened_png = render_html_to_png(flattened_html, &options).expect("flattened renders");

    assert_ne!(
        nested_png, flattened_png,
        "nested list must not render byte-identical to flattened list"
    );
}
