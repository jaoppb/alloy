//! End-to-end integration tests for user navigation and link interactions.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_raw_string_hashes
)]

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use alloy::application::paint::DEFAULT_FONT;
use alloy::{BrowserServices, DEFAULT_FONT_SIZE, run_browser_until};
use graphics::SyntheticFontProvider;
use network::{
    AllowAllPolicy, HeaderMap, HeaderName, HeaderValue, HttpResponse, MockTransport, StatusCode,
    Url,
};
use window::{
    HeadlessWindowSystem, PhysicalPosition, PointerButton, PumpStatus, RecordingPresenter,
    WindowAttributes, WindowError, WindowEvent, WindowId, WindowSystem, WindowTitle,
};

/// The deterministic synthetic font keeps these tests independent of the
/// host's installed fonts.
fn services(
    transport: MockTransport,
) -> BrowserServices<SyntheticFontProvider, MockTransport, AllowAllPolicy> {
    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    BrowserServices::new(
        font_provider,
        Arc::new(transport),
        Arc::new(AllowAllPolicy::new()),
    )
}

fn text_response(body: &str) -> HttpResponse {
    let mut headers = HeaderMap::new();
    headers.set(
        HeaderName::content_type(),
        HeaderValue::from_text("text/html").expect("valid header value"),
    );
    HttpResponse::new(StatusCode::OK, headers, network::Body::from_text(body))
}

/// A [`WindowSystem`] decorator that injects one simulated mouse click on the
/// first pump after `page_ready` is set.
///
/// The page's navigation is fetched on a background thread, so a click on a
/// fixed pump number can land before the page is laid out: it then hits no
/// link target, is never repeated, and the loop waits forever for a
/// navigation that cannot happen (it hung a Windows CI job for 30 minutes).
/// Gating the click on the loop's own stats removes that race.
struct ClickWhenReadyWindowSystem {
    inner: HeadlessWindowSystem,
    click_position: Option<PhysicalPosition>,
    page_ready: Rc<Cell<bool>>,
}

impl ClickWhenReadyWindowSystem {
    fn new(click_position: PhysicalPosition, page_ready: Rc<Cell<bool>>) -> Self {
        Self {
            inner: HeadlessWindowSystem::new(),
            click_position: Some(click_position),
            page_ready,
        }
    }
}

impl WindowSystem for ClickWhenReadyWindowSystem {
    fn create_window(&mut self, attrs: &WindowAttributes) -> Result<WindowId, WindowError> {
        self.inner.create_window(attrs)
    }

    fn pump_events(
        &mut self,
        sink: &mut dyn FnMut(WindowEvent),
    ) -> Result<PumpStatus, WindowError> {
        if self.page_ready.get()
            && let Some(position) = self.click_position.take()
        {
            sink(WindowEvent::PointerMoved { position });
            sink(WindowEvent::PointerButton {
                button: PointerButton::Left,
                pressed: true,
            });
        }
        self.inner.pump_events(sink)
    }

    fn request_redraw(&mut self) {
        self.inner.request_redraw();
    }
}

/// How many loop cycles a test may run before it gives up. At the loop's
/// 4 ms idle poll this is several seconds — far beyond a healthy run — so a
/// regression fails its assertion instead of hanging CI.
const MAX_LOOP_CYCLES: usize = 2_500;

/// Whether the first document has been applied and laid out, so its link
/// targets exist and a click can hit one.
const fn first_page_laid_out(stats: &alloy::LoopStats) -> bool {
    stats.navigations >= 1 && stats.relayouts >= 1
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

    let transport = MockTransport::new()
        .with_response(url_a.clone(), text_response(page_a))
        .with_response(url_b, text_response(page_b));
    let page_ready = Rc::new(Cell::new(false));
    let mut system =
        ClickWhenReadyWindowSystem::new(PhysicalPosition::new(20.0, 20.0), Rc::clone(&page_ready));
    let mut presenter = RecordingPresenter::new();
    let size = window::SurfaceSize::new(200, 150).unwrap();
    let attributes = WindowAttributes::new(WindowTitle::from("test"), size);
    system.create_window(&attributes).unwrap();

    let mut cycles = 0_usize;
    let stats = run_browser_until(
        &url_a,
        services(transport),
        &mut system,
        &mut presenter,
        size,
        |stats| {
            cycles += 1;
            page_ready.set(first_page_laid_out(stats));
            stats.navigations >= 2 || cycles >= MAX_LOOP_CYCLES
        },
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

    let transport =
        MockTransport::new().with_response(start_url.clone(), text_response(page_with_anchor));
    let page_ready = Rc::new(Cell::new(false));
    let mut system =
        ClickWhenReadyWindowSystem::new(PhysicalPosition::new(20.0, 20.0), Rc::clone(&page_ready));
    let mut presenter = RecordingPresenter::new();
    let size = window::SurfaceSize::new(200, 150).unwrap();
    let attributes = WindowAttributes::new(WindowTitle::from("test"), size);
    system.create_window(&attributes).unwrap();

    // Run until the click has been delivered and a few more cycles have had
    // the chance to start (wrongly) a network navigation.
    let mut cycles = 0_usize;
    let mut cycles_since_ready = 0_usize;
    let stats = run_browser_until(
        &start_url,
        services(transport),
        &mut system,
        &mut presenter,
        size,
        |stats| {
            cycles += 1;
            page_ready.set(first_page_laid_out(stats));
            if page_ready.get() {
                cycles_since_ready += 1;
            }
            cycles_since_ready >= 15 || cycles >= MAX_LOOP_CYCLES
        },
    )
    .expect("browser loop runs");

    assert_eq!(
        stats.navigations, 1,
        "in-page anchor must not trigger additional network navigation"
    );
}

/// Drives a full browser session against a single start URL whose response is
/// `response`, stopping once the loop has presented at least one frame.
fn navigate_once(start_url: &Url, response: HttpResponse) -> alloy::LoopStats {
    let transport = MockTransport::new().with_response(start_url.clone(), response);
    let mut system = HeadlessWindowSystem::new();
    let mut presenter = RecordingPresenter::new();
    let size = window::SurfaceSize::new(200, 150).unwrap();
    let attributes = WindowAttributes::new(WindowTitle::from("test"), size);
    system.create_window(&attributes).unwrap();

    let stats = run_browser_until(
        start_url,
        services(transport),
        &mut system,
        &mut presenter,
        size,
        |stats| stats.navigation_errors >= 1,
    )
    .expect("browser loop runs");

    assert!(
        presenter.last_frame().is_some(),
        "a frame must have been presented (the error card)"
    );
    stats
}

#[test]
fn navigating_to_an_empty_body_shows_the_error_card_not_a_blank_window() {
    let start_url = Url::parse("http://example.com/empty").unwrap();
    let empty_ok = HttpResponse::new(StatusCode::OK, HeaderMap::new(), network::Body::empty());

    let stats = navigate_once(&start_url, empty_ok);

    assert_eq!(
        stats.navigations, 0,
        "an empty body is not a rendered document"
    );
    assert_eq!(
        stats.navigation_errors, 1,
        "it must fall back to the error card"
    );
}

#[test]
fn navigating_to_a_204_shows_the_error_card() {
    let start_url = Url::parse("http://example.com/no-content").unwrap();
    let no_content = HttpResponse::new(
        StatusCode::new(204).unwrap(),
        HeaderMap::new(),
        network::Body::empty(),
    );

    let stats = navigate_once(&start_url, no_content);

    assert_eq!(stats.navigations, 0);
    assert_eq!(stats.navigation_errors, 1);
}

#[test]
fn navigating_to_a_non_utf8_body_shows_the_error_card() {
    let start_url = Url::parse("http://example.com/binary").unwrap();
    let binary = HttpResponse::new(
        StatusCode::OK,
        HeaderMap::new(),
        network::Body::from_bytes(vec![0xFF, 0xFE, 0x00, 0x9C]),
    );

    let stats = navigate_once(&start_url, binary);

    assert_eq!(stats.navigations, 0);
    assert_eq!(stats.navigation_errors, 1);
}
