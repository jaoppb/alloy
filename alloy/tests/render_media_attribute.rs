//! Render integration tests verifying media attribute support on `<style>` and `<link>` (issue #79).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use alloy::{RenderOptions, render_html_to_png};

#[test]
fn print_style_attribute_renders_byte_identical_to_control() {
    let options = RenderOptions::new(400, 200);
    let control_html = "<!doctype html><html><head><style>body{margin:0;background:#fff}</style></head><body>x</body></html>";
    let print_attr_html = "<!doctype html><html><head><style>body{margin:0;background:#fff}</style><style media=\"print\">body{background:red}</style></head><body>x</body></html>";

    let control_png = render_html_to_png(control_html, &options).expect("control renders");
    let print_png = render_html_to_png(print_attr_html, &options).expect("print attr renders");

    assert_eq!(
        print_png, control_png,
        "print stylesheet must not apply to screen rendering"
    );
}

#[test]
fn style_element_media_query_respects_viewport_width() {
    let test_html = "<!doctype html><html><head><style>body{margin:0;background:#fff}</style><style media=\"(min-width: 300px)\">body{background:red}</style></head><body>x</body></html>";
    let control_html = "<!doctype html><html><head><style>body{margin:0;background:#fff}</style></head><body>x</body></html>";

    let options_400 = RenderOptions::new(400, 200);
    let options_200 = RenderOptions::new(200, 200);

    let test_400_png = render_html_to_png(test_html, &options_400).expect("renders at 400");
    let test_200_png = render_html_to_png(test_html, &options_200).expect("renders at 200");

    let control_400_png =
        render_html_to_png(control_html, &options_400).expect("control renders at 400");
    let control_200_png =
        render_html_to_png(control_html, &options_200).expect("control renders at 200");

    assert_ne!(
        test_400_png, control_400_png,
        "style media=\"(min-width: 300px)\" must apply at width 400"
    );
    assert_eq!(
        test_200_png, control_200_png,
        "style media=\"(min-width: 300px)\" must not apply at width 200"
    );
}
