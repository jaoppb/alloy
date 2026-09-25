//! End-to-end integration tests for user navigation and link interactions.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_raw_string_hashes
)]

use std::sync::Arc;

use alloy::run_browser_until;
use network::{
    AllowAllPolicy, HeaderMap, HeaderName, HeaderValue, HttpResponse, MockTransport, StatusCode,
    Url,
};
use window::{
    HeadlessWindowSystem, PhysicalPosition, PointerButton, PumpStatus, RecordingPresenter,
    WindowAttributes, WindowError, WindowEvent, WindowId, WindowSystem, WindowTitle,
};

fn text_response(body: &str) -> HttpResponse {
    let mut headers = HeaderMap::new();
    headers.set(
        HeaderName::content_type(),
        HeaderValue::from_text("text/html").expect("valid header value"),
    );
    HttpResponse::new(StatusCode::OK, headers, network::Body::from_text(body))
}

/// A [`WindowSystem`] decorator that injects a simulated mouse click once the
/// initial page has had enough pump cycles to navigate and layout.
struct DelayedClickWindowSystem {
    inner: HeadlessWindowSystem,
    click_pos: Option<PhysicalPosition>,
    pump_count: usize,
    trigger_pump: usize,
}

impl DelayedClickWindowSystem {
    fn new(click_pos: PhysicalPosition, trigger_pump: usize) -> Self {
        Self {
            inner: HeadlessWindowSystem::new(),
            click_pos: Some(click_pos),
            pump_count: 0,
            trigger_pump,
        }
    }
}

impl WindowSystem for DelayedClickWindowSystem {
    fn create_window(&mut self, attrs: &WindowAttributes) -> Result<WindowId, WindowError> {
        self.inner.create_window(attrs)
    }

    fn pump_events(
        &mut self,
        sink: &mut dyn FnMut(WindowEvent),
    ) -> Result<PumpStatus, WindowError> {
        self.pump_count = self.pump_count.saturating_add(1);
        if self.pump_count >= self.trigger_pump
            && let Some(pos) = self.click_pos.take()
        {
            sink(WindowEvent::PointerMoved { position: pos });
            sink(WindowEvent::PointerButton {
                button: PointerButton::Left,
                pressed: true,
            });
        }
        self.inner.pump_events(sink)
    }
}

#[test]
fn clicking_a_link_navigates_to_destination_page() {
    let page_a = r##"
        <!DOCTYPE html>
        <html>
        <body>
            <a href="page-b.html" style="display: block; width: 100px; height: 50px;">Link to B</a>
        </body>
        </html>
    "##;
    let page_b = r##"
        <!DOCTYPE html>
        <html>
        <body>
            <h1>Page B Loaded Successfully</h1>
        </body>
        </html>
    "##;

    let url_a = Url::parse("http://example.com/page-a.html").unwrap();
    let url_b = Url::parse("http://example.com/page-b.html").unwrap();

    let transport = Arc::new(
        MockTransport::new()
            .with_response(url_a.clone(), text_response(page_a))
            .with_response(url_b, text_response(page_b)),
    );
    let policy = Arc::new(AllowAllPolicy::new());
    let mut system = DelayedClickWindowSystem::new(PhysicalPosition::new(20.0, 20.0), 5);
    let mut presenter = RecordingPresenter::new();
    let size = window::SurfaceSize::new(200, 150).unwrap();
    let attributes = WindowAttributes::new(WindowTitle::from("test"), size);
    system.create_window(&attributes).unwrap();

    let stats = run_browser_until(
        &url_a,
        transport,
        policy,
        &mut system,
        &mut presenter,
        size,
        |stats| stats.navigations >= 2,
    )
    .expect("browser loop runs and navigates");

    assert_eq!(
        stats.navigations, 2,
        "must have completed 2 navigations (initial load + clicked link)"
    );
}

#[test]
fn clicking_an_anchor_fragment_does_not_trigger_network_navigation() {
    let page_with_anchor = r##"
        <!DOCTYPE html>
        <html>
        <body>
            <a href="#section" style="display: block; width: 100px; height: 50px;">Jump to Section</a>
        </body>
        </html>
    "##;

    let start_url = Url::parse("http://example.com/index.html").unwrap();

    let transport = Arc::new(
        MockTransport::new().with_response(start_url.clone(), text_response(page_with_anchor)),
    );
    let policy = Arc::new(AllowAllPolicy::new());
    let mut system = DelayedClickWindowSystem::new(PhysicalPosition::new(20.0, 20.0), 5);
    let mut presenter = RecordingPresenter::new();
    let size = window::SurfaceSize::new(200, 150).unwrap();
    let attributes = WindowAttributes::new(WindowTitle::from("test"), size);
    system.create_window(&attributes).unwrap();

    // Run until after the click has had a chance to be processed
    let mut poll_cycles = 0;
    let stats = run_browser_until(
        &start_url,
        transport,
        policy,
        &mut system,
        &mut presenter,
        size,
        |stats| {
            poll_cycles += 1;
            stats.navigations >= 1 && poll_cycles >= 15
        },
    )
    .expect("browser loop runs");

    assert_eq!(
        stats.navigations, 1,
        "in-page anchor must not trigger additional network navigation"
    );
}
