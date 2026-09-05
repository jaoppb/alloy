//! v0.5 Phase I4 end-to-end golden test.
//!
//! Drives [`alloy::run_browser_until`] — the exact loop `alloy <url>` runs —
//! over `network::MockTransport` and `window::{HeadlessWindowSystem,
//! RecordingPresenter}` instead of real sockets and a real display: a page
//! with author CSS (an external `<link rel=stylesheet>`, proving the
//! subresource-fetch path, not just an inline `<style>`), text, and an image
//! is navigated, and the presented frame is compared pixel-for-pixel against
//! a committed reference (`UPDATE_GOLDEN=1` blesses a new one, same
//! convention as `render_golden.rs`).
//!
//! The two coalescing invariants this phase is named for — a burst of
//! resizes or image arrivals costs at most one relayout — are proven as
//! direct, thread-free unit tests of `pump_once` inside
//! `alloy/src/application/event_loop.rs` itself (that function is private,
//! so an external `tests/` file cannot reach it, and the property under test
//! has nothing to do with real thread timing — see that module's own test
//! doc comment for why).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;

use alloy::run_browser_until;
use graphics::golden::assert_matches_golden;
use graphics::{Color, Framebuffer, SurfaceSize as GraphicsSurfaceSize};
use network::{
    AllowAllPolicy, HeaderMap, HeaderName, HeaderValue, HttpResponse, HttpTransport, MockTransport,
    RequestPolicy, StatusCode, Url,
};
use window::{
    FrameView, HeadlessWindowSystem, RecordingPresenter, WindowAttributes, WindowSystem,
    WindowTitle,
};

const PAGE_HTML: &str = include_str!("fixtures/i4_page.html");
const PAGE_CSS: &str = include_str!("fixtures/i4_style.css");

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

fn text_response(body: &str, content_type: &str) -> HttpResponse {
    let mut headers = HeaderMap::new();
    headers.set(
        HeaderName::content_type(),
        HeaderValue::from_text(content_type).expect("valid header value"),
    );
    HttpResponse::new(StatusCode::OK, headers, network::Body::from_text(body))
}

fn image_response() -> HttpResponse {
    let size = GraphicsSurfaceSize::new(4, 4).expect("4x4 is a valid surface size");
    let framebuffer =
        Framebuffer::filled(size, Color::rgb(0xCC, 0x33, 0x11)).expect("framebuffer fills");
    let png_bytes = graphics::png::encode(&framebuffer);
    let mut headers = HeaderMap::new();
    headers.set(
        HeaderName::content_type(),
        HeaderValue::from_text("image/png").expect("valid header value"),
    );
    HttpResponse::new(
        StatusCode::OK,
        headers,
        network::Body::from_slice(&png_bytes),
    )
}

fn mock_transport(base: &Url) -> Arc<dyn HttpTransport> {
    let page_url = base.join("i4_page.html").expect("valid relative URL");
    let style_url = base.join("style.css").expect("valid relative URL");
    let image_url = base.join("pic.png").expect("valid relative URL");
    let transport = MockTransport::new()
        .with_response(
            page_url,
            text_response(PAGE_HTML, "text/html; charset=utf-8"),
        )
        .with_response(style_url, text_response(PAGE_CSS, "text/css"))
        .with_response(image_url, image_response());
    Arc::new(transport)
}

/// Straight-alpha `RGBA8` from a `FrameView`'s premultiplied `0xAARRGGBB` —
/// exact for the fully-opaque scene this test paints (premultiplying by
/// `alpha == 255` is a no-op), which is the only case this test needs.
fn framebuffer_from_frame_view(view: FrameView<'_>) -> Framebuffer {
    let mut bytes = Vec::with_capacity(view.pixels().len().saturating_mul(4));
    for pixel in view.pixels() {
        let alpha = u8::try_from((pixel >> 24) & 0xFF).unwrap_or(0);
        let red = u8::try_from((pixel >> 16) & 0xFF).unwrap_or(0);
        let green = u8::try_from((pixel >> 8) & 0xFF).unwrap_or(0);
        let blue = u8::try_from(pixel & 0xFF).unwrap_or(0);
        bytes.extend_from_slice(&[red, green, blue, alpha]);
    }
    let size = GraphicsSurfaceSize::new(view.width(), view.height()).expect("non-zero frame size");
    Framebuffer::from_rgba8(size, bytes).expect("byte count matches width*height*4")
}

#[test]
fn navigated_page_with_stylesheet_and_image_matches_golden_reference() {
    let base = Url::parse("http://example.invalid/").expect("valid base URL");
    let page_url = base.join("i4_page.html").expect("valid relative URL");
    let transport = mock_transport(&base);
    let policy: Arc<dyn RequestPolicy> = Arc::new(AllowAllPolicy::new());

    // A small viewport keeps the committed golden PNG small — this test
    // proves the pipeline wiring, not a real page's dimensions.
    let small_size = window::SurfaceSize::new(64, 48).expect("64x48 is a valid surface size");
    let attributes = WindowAttributes::new(WindowTitle::from("i4 e2e"), small_size);
    let mut system = HeadlessWindowSystem::new();
    system
        .create_window(&attributes)
        .expect("headless window always creates");
    let mut presenter = RecordingPresenter::new();

    let stats = run_browser_until(
        &page_url,
        transport,
        policy,
        &mut system,
        &mut presenter,
        attributes.initial_size(),
        |stats| stats.stylesheets_loaded >= 1 && stats.images_loaded >= 1,
    )
    .expect("the session runs to the requested stopping point without error");

    assert!(stats.navigations >= 1, "the page must have navigated");
    assert!(
        stats.relayouts >= 1,
        "at least one frame must have been presented"
    );

    let frame = presenter
        .last_frame()
        .expect("a frame was presented once the stylesheet and image landed");
    let framebuffer = framebuffer_from_frame_view(frame);
    assert_matches_golden(&framebuffer, &golden_path("i4_page.png"));
}
