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
use css::{
    BlockLayout, CascadeResolver, FontBackedMeasurer, LayoutBoxTree, LayoutEngine, UaCascade,
    ViewportConstraints, collect_style_sheets, snapshot,
};
use graphics::golden::assert_matches_golden;
use graphics::png::decode as decode_png;
use graphics::{Au, Color, SyntheticFontProvider};

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

#[test]
fn inline_svg_renders_byte_identical_to_inline_block_span() {
    let options = RenderOptions::new(100, 100);
    let svg_html = "<!DOCTYPE html><html><body><svg width=\"20\" height=\"20\"><text>hello</text></svg></body></html>";
    let span_html = "<!DOCTYPE html><html><body><span style=\"display:inline-block;width:20px;height:20px\"></span></body></html>";

    let svg_bytes = render_html_to_png(svg_html, &options).expect("svg render succeeds");
    let span_bytes = render_html_to_png(span_html, &options).expect("span render succeeds");

    assert_eq!(
        svg_bytes, span_bytes,
        "<svg width=\"20\" height=\"20\"><text>hello</text></svg> must render byte-identical to inline-block span"
    );
}
const RED: graphics::Color = graphics::Color::rgb(0xFF, 0x00, 0x00);

#[test]
fn border_shorthand_sets_style_and_color_and_paints_correctly() {
    let html = "<!doctype html><html><head><style>body{margin:0;background:#fff} #a{border:4px solid red;color:blue}</style></head>\
                <body><div id=\"a\">Hello world text here</div></body></html>";
    let options = RenderOptions::new(400, 200);
    let bytes = render_html_to_png(html, &options).expect("render succeeds");
    let frame = decode_png(&bytes).expect("valid PNG");

    // The top border row (y = 0..4, x = 0..400) must be red (255, 0, 0), not text color blue.
    assert_eq!(
        frame.pixel(10, 2),
        Some(RED),
        "top border pixels must be painted in red, not blue"
    );
}

#[test]
fn border_width_with_no_border_style_renders_same_as_no_border() {
    let with_border_width = "<!doctype html><html><head><style>body{margin:0;background:#fff} #a{border-width:4px;color:blue}</style></head>\
                             <body><div id=\"a\">Hello world text here</div></body></html>";
    let without_border = "<!doctype html><html><head><style>body{margin:0;background:#fff} #a{color:blue}</style></head>\
                          <body><div id=\"a\">Hello world text here</div></body></html>";
    let options = RenderOptions::new(400, 200);
    let bytes_with_width =
        render_html_to_png(with_border_width, &options).expect("render with width succeeds");
    let bytes_without =
        render_html_to_png(without_border, &options).expect("render without border succeeds");

    assert_eq!(
        bytes_with_width, bytes_without,
        "border-width alone with no border-style must render identically to no border"
    );
}

#[test]
fn visibility_hidden_renders_same_as_empty_box_while_preserving_sibling_layout() {
    let with_visibility_hidden = "<!doctype html><html><head><style>\
                                  body{margin:0;background:#fff}\
                                  #hidden{visibility:hidden;background:red;height:50px}\
                                  #sibling{background:blue;height:50px}\
                                  </style></head>\
                                  <body><div id=\"hidden\">Hidden text content</div>\
                                  <div id=\"sibling\">Following sibling</div></body></html>";

    let without_bg_and_text = "<!doctype html><html><head><style>\
                               body{margin:0;background:#fff}\
                               #hidden{height:50px}\
                               #sibling{background:blue;height:50px}\
                               </style></head>\
                               <body><div id=\"hidden\"></div>\
                               <div id=\"sibling\">Following sibling</div></body></html>";

    let options = RenderOptions::new(400, 200);
    let bytes_hidden =
        render_html_to_png(with_visibility_hidden, &options).expect("render hidden succeeds");
    let bytes_empty =
        render_html_to_png(without_bg_and_text, &options).expect("render empty succeeds");

    assert_eq!(
        bytes_hidden, bytes_empty,
        "visibility:hidden must render identically to a box with no background and no text"
    );

    let with_visibility_collapse = "<!doctype html><html><head><style>\
                                    body{margin:0;background:#fff}\
                                    #hidden{visibility:collapse;background:red;height:50px}\
                                    #sibling{background:blue;height:50px}\
                                    </style></head>\
                                    <body><div id=\"hidden\">Hidden text content</div>\
                                    <div id=\"sibling\">Following sibling</div></body></html>";
    let bytes_collapse =
        render_html_to_png(with_visibility_collapse, &options).expect("render collapse succeeds");
    assert_eq!(
        bytes_collapse, bytes_empty,
        "visibility:collapse outside tables must render identically to visibility:hidden"
    );
}

#[test]
fn visible_child_inside_hidden_parent_paints_and_matches_golden() {
    let html = "<!doctype html><html><head><style>\
                body{margin:0;background:#fff}\
                #parent{visibility:hidden;background:red;width:200px;height:100px}\
                #child{visibility:visible;background:#00ff00;width:100px;height:50px}\
                </style></head>\
                <body><div id=\"parent\"><div id=\"child\">Visible child</div></div></body></html>";

    let options = RenderOptions::new(320, 240);
    let bytes = render_html_to_png(html, &options).expect("render succeeds");
    let frame = decode_png(&bytes).expect("valid PNG");

    // Parent background (red) must NOT be drawn; body's white background shows through outside child
    assert_eq!(
        frame.pixel(150, 20),
        Some(graphics::Color::WHITE),
        "parent background must not be painted"
    );

    // Child background (green) MUST be drawn
    assert_eq!(
        frame.pixel(80, 40),
        Some(GREEN),
        "child background must be painted"
    );

    assert_matches_golden(&frame, &golden_path("visibility_child.png"));
}

fn layout_for_html(html: &str, width: i32, height: i32) -> LayoutBoxTree {
    let dom_tree = alloy::application::html_parse::parse_html(html).expect("html parse succeeds");
    let snapshot = snapshot(&dom_tree, dom_tree.document());
    let sheets = collect_style_sheets(&snapshot).expect("collect sheets succeeds");
    let constraints = ViewportConstraints::new(
        Au::from_whole_px(width).expect("valid width Au"),
        Au::from_whole_px(height).expect("valid height Au"),
    );
    let sheets = sheets.matching_viewport(&constraints);
    let styled_tree = UaCascade::new()
        .resolve(&snapshot, &sheets)
        .expect("styled tree resolution succeeds");
    let measurer = FontBackedMeasurer::new(
        std::sync::Arc::new(SyntheticFontProvider::new()),
        alloy::application::paint::DEFAULT_FONT,
    );
    BlockLayout::new(measurer)
        .layout(&styled_tree, &constraints)
        .expect("layout succeeds")
}

#[test]
fn outline_paints_outside_border_box_and_leaves_layout_identical() {
    let with_outline = "<!doctype html><html><head><style>\
                        body{margin:0;background:#fff}\
                        #box{margin:20px;width:100px;height:100px;border:2px solid #0000ff;\
                        color:#0000ff;outline:3px solid #ff0000;outline-offset:2px}\
                        </style></head>\
                        <body><div id=\"box\"></div></body></html>";

    let without_outline = "<!doctype html><html><head><style>\
                           body{margin:0;background:#fff}\
                           #box{margin:20px;width:100px;height:100px;border:2px solid #0000ff;\
                           color:#0000ff}\
                           </style></head>\
                           <body><div id=\"box\"></div></body></html>";

    let layout_with = layout_for_html(with_outline, 320, 240);
    let layout_without = layout_for_html(without_outline, 320, 240);
    assert_eq!(
        layout_with, layout_without,
        "outline and outline-offset must not alter layout box tree"
    );

    let options = RenderOptions::new(320, 240);
    let bytes = render_html_to_png(with_outline, &options).expect("render succeeds");
    let frame = decode_png(&bytes).expect("valid PNG");

    let red = Color::rgb(255, 0, 0);
    let blue = Color::rgb(0, 0, 255);
    let white = Color::WHITE;

    // 2-5px outside border box (x/y: 15..18 and 126..129) must be red outline.
    assert_eq!(
        frame.pixel(16, 50),
        Some(red),
        "left outline must be red (4px outside border box)"
    );
    assert_eq!(
        frame.pixel(50, 16),
        Some(red),
        "top outline must be red (4px outside border box)"
    );
    assert_eq!(
        frame.pixel(127, 50),
        Some(red),
        "right outline must be red (3px outside border box)"
    );
    assert_eq!(
        frame.pixel(50, 127),
        Some(red),
        "bottom outline must be red (3px outside border box)"
    );

    // Gap 0-2px outside border box (x/y: 18..20 and 124..126) must be white (background).
    assert_eq!(
        frame.pixel(19, 50),
        Some(white),
        "gap between border and outline must be white"
    );
    assert_eq!(frame.pixel(50, 19), Some(white), "top gap must be white");
    assert_eq!(frame.pixel(125, 50), Some(white), "right gap must be white");
    assert_eq!(
        frame.pixel(50, 125),
        Some(white),
        "bottom gap must be white"
    );

    // Border box edges (x/y: 20..22 and 122..124) must be blue border.
    assert_eq!(frame.pixel(21, 50), Some(blue), "left border must be blue");
    assert_eq!(frame.pixel(50, 21), Some(blue), "top border must be blue");
    assert_eq!(
        frame.pixel(123, 50),
        Some(blue),
        "right border must be blue"
    );
    assert_eq!(
        frame.pixel(50, 123),
        Some(blue),
        "bottom border must be blue"
    );
}
