//! End-to-end headless render golden tests and determinism gate (ADR-0016).
//!
//! Asserts that:
//! 1. HTML parsed, styled, laid out, and rasterized to PNG matches a blessed
//!    golden image pixel-for-pixel (`UPDATE_GOLDEN=1` supported).
//! 2. 100 consecutive executions produce byte-for-byte identical PNG outputs.
//! 3. Invalid surface dimensions are cleanly refused with `AlloyError::InvalidDimensions`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use alloy::{AlloyError, RenderOptions, render_html_to_png, run_render};
use graphics::golden::assert_matches_golden;
use graphics::png::decode as decode_png;

const TEST_HTML: &str = include_str!("fixtures/test_page.html");

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

#[test]
fn render_pipeline_matches_golden_reference() {
    let options = RenderOptions::new(320, 240);
    let png_bytes = render_html_to_png(TEST_HTML, &options).expect("render succeeds");
    let frame = decode_png(&png_bytes).expect("output must be a valid PNG");
    assert_matches_golden(&frame, &golden_path("pipeline.png"));
}

#[test]
fn render_pipeline_is_strictly_deterministic_over_100_runs() {
    let options = RenderOptions::new(320, 240);
    let reference_bytes = render_html_to_png(TEST_HTML, &options).expect("first render succeeds");

    for iteration in 1..=100 {
        let current_bytes =
            render_html_to_png(TEST_HTML, &options).expect("subsequent render succeeds");
        assert_eq!(
            reference_bytes, current_bytes,
            "render output diverged on iteration {iteration}"
        );
    }
}

#[test]
fn run_render_convenience_matches_render_html_to_png() {
    let options = RenderOptions::new(320, 240);
    let direct_bytes = render_html_to_png(TEST_HTML, &options).expect("direct render succeeds");
    let convenience_bytes = run_render(TEST_HTML, 320, 240).expect("convenience render succeeds");
    assert_eq!(direct_bytes, convenience_bytes);
}

#[test]
fn invalid_surface_dimensions_are_refused() {
    let zero_width = RenderOptions::new(0, 600);
    let result_zero_width = render_html_to_png(TEST_HTML, &zero_width);
    assert!(matches!(
        result_zero_width,
        Err(AlloyError::InvalidDimensions)
    ));

    let zero_height = RenderOptions::new(800, 0);
    let result_zero_height = render_html_to_png(TEST_HTML, &zero_height);
    assert!(matches!(
        result_zero_height,
        Err(AlloyError::InvalidDimensions)
    ));
}

/// `height` sits outside the `@media` block so the body box is 60px tall in
/// both cases; only the `background-color` is media-gated, so the sampled
/// pixel isolates whether the query fired.
fn media_page(query: &str) -> String {
    format!(
        "<html><head><style>body {{ height: 60px }} \
         @media {query} {{ body {{ background-color: #00ff00 }} }}</style></head>\
         <body>hi</body></html>"
    )
}

const GREEN: graphics::Color = graphics::Color::rgb(0x00, 0xFF, 0x00);

#[test]
fn a_supported_media_query_applies_once_the_pipeline_knows_the_viewport() {
    let options = RenderOptions::new(200, 120);

    let fires = render_html_to_png(&media_page("(min-width: 1px)"), &options).expect("render");
    let fired_frame = decode_png(&fires).expect("valid PNG");
    assert_eq!(
        fired_frame.pixel(20, 30),
        Some(GREEN),
        "`@media (min-width: 1px)` must apply at a 200px-wide viewport"
    );

    let skipped = render_html_to_png(&media_page("(min-width: 5000px)"), &options).expect("render");
    let skipped_frame = decode_png(&skipped).expect("valid PNG");
    assert_ne!(
        skipped_frame.pixel(20, 30),
        Some(GREEN),
        "`@media (min-width: 5000px)` must not apply at 200px wide"
    );
}

#[test]
fn rendering_html_with_svg_and_unregistered_images_does_not_crash() {
    let html = "<html><body><svg width=\"24\" height=\"24\"></svg><img src=\"missing.png\"><img alt=\"no src\"></body></html>";
    let options = RenderOptions::new(100, 100);
    let bytes =
        render_html_to_png(html, &options).expect("render succeeds without ImageUnavailable crash");
    assert!(!bytes.is_empty());
}
