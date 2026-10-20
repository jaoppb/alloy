//! Direct, thread-free proofs of the one invariant `06-i4-alloy-url.md` names
//! twice.
//!
//! Resize and subresource-arrival coalescing collapse an arbitrary burst of
//! events into **at most one** relayout per pump cycle. Exercising
//! `pump_once` straight (rather than the threaded `run_browser`) is
//! deliberate: the property under test is what one drain-and-decide cycle
//! does with whatever is already queued, which has nothing to do with how
//! fast a background fetch thread happens to run — testing it through real
//! threads would make the proof racy for no added coverage.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use graphics::{ImageId, SyntheticFontProvider};
use network::{AllowAllPolicy, MockTransport};
use window::{
    HeadlessWindowSystem, KeyCode, KeyState, LogicalKey, RecordingPresenter, SurfaceSize,
    WindowEvent, WindowSystem as _,
};

use super::session::Session;
use super::worker::{LoopMessage, fetch_text};
use super::{initial_window_attributes, pump_once};
use crate::application::browser_services::BrowserServices;
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::DEFAULT_FONT_SIZE;
use crate::application::subresource::{MarkupDiscoverer, placeholder_framebuffer};
use crate::error::AlloyError;

#[test]
fn a_non_2xx_stylesheet_status_is_a_typed_error_not_an_empty_body() {
    let sheet_url = network::Url::parse("http://example.com/missing.css").unwrap();
    let not_found = network::HttpResponse::new(
        network::StatusCode::NOT_FOUND,
        network::HeaderMap::new(),
        network::Body::from_text("<!doctype html><title>404</title>"),
    );
    let transport = MockTransport::new().with_response(sheet_url.clone(), not_found);

    let result = fetch_text(&sheet_url, &transport);

    assert!(
        matches!(result, Err(AlloyError::HttpStatus { status: 404, .. })),
        "a 404 error page must not reach the CSS parser as a stylesheet"
    );
}

type TestSession = Session<SyntheticFontProvider, MockTransport, AllowAllPolicy, MarkupDiscoverer>;

fn session_over(transport: MockTransport, viewport: SurfaceSize) -> TestSession {
    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    let services = BrowserServices::new(
        font_provider,
        Arc::new(transport),
        Arc::new(AllowAllPolicy::new()),
    );
    Session::new(viewport, services)
}

/// The URL a link click navigated to: keeps pumping until the click's
/// navigation — the first one past `navigations_before_click` — has been
/// applied, so the outcome does not depend on how fast its thread runs.
fn completed_navigation(
    system: &mut HeadlessWindowSystem,
    presenter: &mut RecordingPresenter,
    session: &mut TestSession,
    navigations_before_click: usize,
) -> network::Url {
    for _ in 0..5000 {
        if session.stats().navigations > navigations_before_click {
            break;
        }
        pump(system, presenter, session);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(
        session.stats().navigations > navigations_before_click,
        "the click's navigation never arrived"
    );
    session
        .base_url()
        .cloned()
        .expect("an applied navigation sets the document base")
}

fn navigation_to(markup: &str, url: &str) -> LoopMessage {
    let document = dom::parse(markup).unwrap().into_tree();
    LoopMessage::Navigation(Ok((document, network::Url::parse(url).unwrap())))
}

/// Installs `markup` as a navigated document at `url`, the same way a
/// finished navigation thread does.
fn load(session: &mut TestSession, markup: &str, url: &str) {
    session
        .outbox_for_test()
        .send(navigation_to(markup, url))
        .unwrap();
    session.drain_messages();
}

fn loaded_session(viewport: SurfaceSize) -> TestSession {
    let mut session = session_over(MockTransport::new(), viewport);
    load(
        &mut session,
        "<html><body>hi</body></html>",
        "http://example.com/",
    );
    session
}

#[test]
fn multiple_resizes_in_one_pump_coalesce_to_one_relayout() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let bigger = SurfaceSize::new(1024, 768).unwrap();
    let smaller = SurfaceSize::new(640, 480).unwrap();
    system.schedule(WindowEvent::Resized(bigger));
    system.schedule(WindowEvent::Resized(smaller));

    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());

    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        session.stats().relayouts,
        1,
        "three coalesced Resized events (the initial one plus two scheduled) must cost exactly one relayout"
    );
    assert_eq!(
        session.viewport(),
        smaller,
        "the viewport must reflect the LAST resize in the coalesced batch"
    );
}

#[test]
fn fifty_image_arrivals_in_one_pump_coalesce_to_one_relayout() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();

    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());
    let outbox = session.outbox_for_test();
    for index in 0..50u32 {
        outbox
            .send(LoopMessage::Image(
                ImageId::new(index),
                Ok(placeholder_framebuffer()),
            ))
            .unwrap();
    }

    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        session.stats().relayouts,
        1,
        "fifty coalesced image arrivals must cost exactly one relayout, not fifty"
    );
}

fn pump(
    system: &mut HeadlessWindowSystem,
    presenter: &mut RecordingPresenter,
    session: &mut TestSession,
) {
    pump_once(system, presenter, session).unwrap();
}

#[test]
fn a_redraw_request_repaints_the_cached_frame_without_a_relayout() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());

    // First pump: the auto-seeded Resized lays out and presents once.
    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(session.stats().relayouts, 1);
    assert_eq!(presenter.present_count(), 1);

    system.schedule(WindowEvent::RedrawRequested);
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        session.stats().relayouts,
        1,
        "a RedrawRequested must not trigger another relayout"
    );
    assert_eq!(
        presenter.present_count(),
        2,
        "a RedrawRequested must re-blit the cached frame"
    );
}

#[test]
fn a_redraw_request_before_the_first_frame_is_a_silent_noop() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    // No document yet.
    let mut session = session_over(MockTransport::new(), attributes.initial_size());

    system.schedule(WindowEvent::RedrawRequested);
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        presenter.present_count(),
        0,
        "a RedrawRequested with nothing rendered yet must present nothing"
    );
    assert!(!session.has_frame());
}

#[test]
fn a_relayout_arms_a_following_repaint() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());

    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(presenter.present_count(), 1);

    // Nothing new scheduled: the redraw the relayout re-armed is the only
    // thing this pump sees, and it must repaint (not relayout).
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        session.stats().relayouts,
        1,
        "no new relayout without new content"
    );
    assert_eq!(
        presenter.present_count(),
        2,
        "the re-armed redraw repainted"
    );
}

#[test]
fn many_redraw_requests_in_one_pump_coalesce_to_one_repaint() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());

    pump(&mut system, &mut presenter, &mut session);
    let presents_after_load = presenter.present_count();

    for _ in 0..50 {
        system.schedule(WindowEvent::RedrawRequested);
    }
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(session.stats().relayouts, 1);
    assert_eq!(
        presenter.present_count(),
        presents_after_load + 1,
        "fifty coalesced RedrawRequested events cost exactly one repaint"
    );
}

#[test]
fn clicking_a_link_triggers_navigation_to_resolved_url() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();

    let mut presenter = RecordingPresenter::new();
    let target_url = network::Url::parse("http://example.com/target.html").unwrap();
    let response = network::HttpResponse::new(
        network::StatusCode::OK,
        network::HeaderMap::new(),
        network::Body::from_text("<html><body>target</body></html>"),
    );
    let mut session = session_over(
        MockTransport::new().with_response(target_url, response),
        attributes.initial_size(),
    );
    load(
        &mut session,
        "<html><body><a href=\"target.html\" style=\"display: block; width: 100px; height: 50px;\">Click me</a></body></html>",
        "http://example.com/index.html",
    );

    // First pump: renders the document and collects its link areas
    pump(&mut system, &mut presenter, &mut session);

    assert!(session.has_links(), "link target must be collected");

    // Move pointer over the link (at document y=20, so window y=42+20=62) and click
    let navigations_before_click = session.stats().navigations;
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, 62.0),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });

    pump(&mut system, &mut presenter, &mut session);

    let expected = network::Url::parse("http://example.com/target.html").unwrap();
    assert_eq!(
        completed_navigation(
            &mut system,
            &mut presenter,
            &mut session,
            navigations_before_click
        ),
        expected
    );
}

#[test]
fn a_link_click_resolves_against_the_documents_base_href_not_the_navigation_url() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();

    let mut presenter = RecordingPresenter::new();
    let expected = network::Url::parse("https://cdn.example/app/docs.html").unwrap();
    let response = network::HttpResponse::new(
        network::StatusCode::OK,
        network::HeaderMap::new(),
        network::Body::from_text("<html><body>docs</body></html>"),
    );
    let mut session = session_over(
        MockTransport::new().with_response(expected.clone(), response),
        attributes.initial_size(),
    );
    load(
        &mut session,
        "<html><head><base href=\"https://cdn.example/app/\"></head><body>\
         <a href=\"docs.html\" style=\"display: block; width: 100px; height: 50px;\">Docs</a>\
         </body></html>",
        "https://example.com/index.html",
    );

    assert_eq!(
        session.base_url().map(ToString::to_string).as_deref(),
        Some("https://cdn.example/app/"),
        "the session keeps the `<base href>`, not the navigation URL"
    );

    pump(&mut system, &mut presenter, &mut session);
    let navigations_before_click = session.stats().navigations;
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, 62.0),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        completed_navigation(
            &mut system,
            &mut presenter,
            &mut session,
            navigations_before_click
        ),
        expected
    );
}

#[test]
fn a_resize_before_any_document_counts_no_relayout() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    // The auto-seeded Resized arrives while the navigation is still in flight.
    let mut session = session_over(MockTransport::new(), attributes.initial_size());

    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        session.stats().relayouts,
        0,
        "nothing was presented, so `run_browser_until_first_frame` must keep waiting"
    );
    assert_eq!(presenter.present_count(), 0);
    assert!(
        !session.needs_relayout(),
        "the dropped request is not retried"
    );
}

fn page_url(path: &str) -> String {
    format!("http://example.com/{path}")
}

fn pump_queued(session: &mut TestSession) {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    pump(&mut system, &mut presenter, session);
}

fn late_stylesheet() -> LoopMessage {
    LoopMessage::Stylesheet(css::MediaQuery::always(), Ok("p { color: red }".into()))
}

#[test]
fn a_subresource_fetched_for_a_page_the_user_left_is_not_applied_to_the_next_one() {
    let mut session = loaded_session(initial_window_attributes().unwrap().initial_size());
    let page_a = session.outbox_for_test();
    // Already queued when the user leaves A; ImageId 12 names a different <img> in B.
    page_a.send(late_stylesheet()).unwrap();
    page_a
        .send(LoopMessage::Image(
            ImageId::new(12),
            Ok(placeholder_framebuffer()),
        ))
        .unwrap();
    session.begin_navigation_for_test();
    let page_b = session.outbox_for_test();
    page_b
        .send(navigation_to(
            "<html><body>b</body></html>",
            &page_url("b.html"),
        ))
        .unwrap();

    pump_queued(&mut session);

    let stats = session.stats();
    assert_eq!(stats.navigations, 2, "page B itself must still be applied");
    assert_eq!(
        stats.stylesheets_loaded, 0,
        "A's stylesheet must not enter B's cascade"
    );
    assert_eq!(
        stats.images_loaded, 0,
        "A's image must not replace whatever B has at the same ImageId"
    );
    assert!(
        page_a.send(late_stylesheet()).is_err(),
        "a worker of A still running has no way back to the session"
    );
}

#[test]
fn a_slower_earlier_navigation_does_not_replace_the_latest_one() {
    let mut session = loaded_session(initial_window_attributes().unwrap().initial_size());
    let slow = session.outbox_for_test();
    session.begin_navigation_for_test();
    let fast = session.outbox_for_test();
    fast.send(navigation_to(
        "<html><body>fast</body></html>",
        &page_url("fast.html"),
    ))
    .unwrap();

    let slow_result = slow.send(navigation_to(
        "<html><body>slow</body></html>",
        &page_url("slow.html"),
    ));
    pump_queued(&mut session);

    assert!(
        slow_result.is_err(),
        "the response of the link clicked first has no way back"
    );
    assert_eq!(
        session.base_url().map(ToString::to_string),
        Some(page_url("fast.html")),
        "the page clicked last stays"
    );
    assert_eq!(
        session.stats().navigations,
        2,
        "the slow one is not counted"
    );
}

#[test]
fn a_failure_of_a_superseded_navigation_does_not_replace_the_current_page() {
    let mut session = loaded_session(initial_window_attributes().unwrap().initial_size());
    let slow = session.outbox_for_test();
    session.begin_navigation_for_test();
    session
        .outbox_for_test()
        .send(navigation_to(
            "<html><body>fast</body></html>",
            &page_url("fast.html"),
        ))
        .unwrap();

    let slow_result = slow.send(LoopMessage::Navigation(Err(AlloyError::InvalidDimensions)));
    pump_queued(&mut session);

    assert!(slow_result.is_err());
    assert_eq!(
        session.stats().navigation_errors,
        0,
        "an error nobody is waiting for must not become the error card"
    );
    assert_eq!(
        session.base_url().map(ToString::to_string),
        Some(page_url("fast.html"))
    );
}

#[test]
fn following_a_link_supersedes_the_navigation_in_flight() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = session_over(MockTransport::new(), attributes.initial_size());
    load(
        &mut session,
        "<html><body><a href=\"target.html\" style=\"display: block; width: 100px; height: 50px;\">Click me</a></body></html>",
        &page_url("index.html"),
    );
    pump(&mut system, &mut presenter, &mut session);
    let before_click = session.outbox_for_test();
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, 62.0),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });

    pump(&mut system, &mut presenter, &mut session);

    assert!(
        before_click.send(late_stylesheet()).is_err(),
        "a click that navigates must cut off everything still in flight"
    );
}

#[test]
fn a_stylesheet_with_never_matching_media_is_not_absorbed() {
    let mut session = loaded_session(initial_window_attributes().unwrap().initial_size());
    let page = session.outbox_for_test();
    page.send(LoopMessage::Stylesheet(
        css::MediaQuery::never(),
        Ok("body { background: red; }".into()),
    ))
    .unwrap();

    pump_queued(&mut session);

    let stats = session.stats();
    assert_eq!(
        stats.stylesheets_loaded, 0,
        "print or non-matching stylesheet must not be absorbed"
    );
}

#[test]
fn ctrl_l_shortcut_focuses_omnibox_and_selects_all_text() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());

    pump(&mut system, &mut presenter, &mut session);
    assert!(!session.omnibox().is_focused());
    assert!(!session.omnibox().is_all_selected());

    system.schedule(WindowEvent::Key {
        code: KeyCode::CONTROL_LEFT,
        pressed: true,
    });
    system.schedule(WindowEvent::Key {
        code: KeyCode::KEY_L,
        pressed: true,
    });
    system.schedule(WindowEvent::Key {
        code: KeyCode::KEY_L,
        pressed: false,
    });
    system.schedule(WindowEvent::Key {
        code: KeyCode::CONTROL_LEFT,
        pressed: false,
    });
    pump(&mut system, &mut presenter, &mut session);

    assert!(
        session.omnibox().is_focused(),
        "Ctrl+L must grant focus to the omnibox"
    );
    assert!(
        session.omnibox().is_all_selected(),
        "Ctrl+L must select existing omnibox text"
    );
}

#[test]
fn alt_d_shortcut_focuses_omnibox_and_selects_all_text() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());

    pump(&mut system, &mut presenter, &mut session);
    assert!(!session.omnibox().is_focused());

    system.schedule(WindowEvent::Key {
        code: KeyCode::ALT_LEFT,
        pressed: true,
    });
    system.schedule(WindowEvent::Key {
        code: KeyCode::KEY_D,
        pressed: true,
    });
    system.schedule(WindowEvent::Key {
        code: KeyCode::KEY_D,
        pressed: false,
    });
    system.schedule(WindowEvent::Key {
        code: KeyCode::ALT_LEFT,
        pressed: false,
    });
    pump(&mut system, &mut presenter, &mut session);

    assert!(
        session.omnibox().is_focused(),
        "Alt+D must grant focus to the omnibox"
    );
    assert!(
        session.omnibox().is_all_selected(),
        "Alt+D must select existing omnibox text"
    );
}

#[test]
fn typing_when_focused_appends_to_buffer_and_requests_redraw_without_relayout() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());

    // Initial pump: document relayouts once.
    pump(&mut system, &mut presenter, &mut session);
    let relayouts_before = session.stats().relayouts;
    assert_eq!(relayouts_before, 1);

    // Focus omnibox via Ctrl+L.
    system.schedule(WindowEvent::Key {
        code: KeyCode::CONTROL_LEFT,
        pressed: true,
    });
    system.schedule(WindowEvent::Key {
        code: KeyCode::KEY_L,
        pressed: true,
    });
    pump(&mut system, &mut presenter, &mut session);
    assert!(session.omnibox().is_focused());
    assert_eq!(
        session.stats().relayouts,
        relayouts_before,
        "focusing omnibox must not trigger a DOM relayout"
    );

    // First character replaces selected text.
    system.schedule(WindowEvent::TextInput {
        text: "h".to_string(),
    });
    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(session.omnibox().text(), "h");
    assert_eq!(
        session.stats().relayouts,
        relayouts_before,
        "typing in omnibox must strictly leave stats.relayouts unchanged"
    );

    // Subsequent characters append to the buffer.
    system.schedule(WindowEvent::TextInput {
        text: "ello".to_string(),
    });
    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(session.omnibox().text(), "hello");
    assert_eq!(
        session.stats().relayouts,
        relayouts_before,
        "typing in omnibox must strictly leave stats.relayouts unchanged"
    );
}

#[test]
fn typing_when_unfocused_is_ignored() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());

    pump(&mut system, &mut presenter, &mut session);
    let initial_text = session.omnibox().text().to_string();

    system.schedule(WindowEvent::TextInput {
        text: "ignored".to_string(),
    });
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        session.omnibox().text(),
        initial_text.as_str(),
        "typing without omnibox focus must not mutate the buffer"
    );
}

#[test]
fn escape_cancels_editing_reverts_to_active_url_and_unfocuses() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());

    pump(&mut system, &mut presenter, &mut session);
    let original_url_text = session.omnibox().text().to_string();

    // Focus and type.
    system.schedule(WindowEvent::Key {
        code: KeyCode::CONTROL_LEFT,
        pressed: true,
    });
    system.schedule(WindowEvent::Key {
        code: KeyCode::KEY_L,
        pressed: true,
    });
    pump(&mut system, &mut presenter, &mut session);

    system.schedule(WindowEvent::TextInput {
        text: "draft query".to_string(),
    });
    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(session.omnibox().text(), "draft query");
    assert!(session.omnibox().is_focused());

    // Press Escape.
    system.schedule(WindowEvent::ControlKey {
        key: LogicalKey::Escape,
        state: KeyState::Pressed,
    });
    pump(&mut system, &mut presenter, &mut session);

    assert!(
        !session.omnibox().is_focused(),
        "Escape must release omnibox focus"
    );
    assert_eq!(
        session.omnibox().text(),
        original_url_text.as_str(),
        "Escape must revert omnibox text to active page URL"
    );
}

#[test]
fn end_to_end_shortcut_typing_and_navigation_dispatch() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();

    let target = network::Url::parse("https://rust-lang.org/").unwrap();
    let response = network::HttpResponse::new(
        network::StatusCode::OK,
        network::HeaderMap::new(),
        network::Body::from_text("<html><body>Rust</body></html>"),
    );
    let mut session = session_over(
        MockTransport::new().with_response(target.clone(), response),
        attributes.initial_size(),
    );
    load(
        &mut session,
        "<html><body>Initial page</body></html>",
        "http://example.com/index.html",
    );
    pump(&mut system, &mut presenter, &mut session);
    let navigations_before = session.stats().navigations;

    // 1. Press Ctrl+L to focus omnibox.
    system.schedule(WindowEvent::Key {
        code: KeyCode::CONTROL_LEFT,
        pressed: true,
    });
    system.schedule(WindowEvent::Key {
        code: KeyCode::KEY_L,
        pressed: true,
    });
    pump(&mut system, &mut presenter, &mut session);
    assert!(session.omnibox().is_focused());

    // 2. Type "rust-lang.org" (without scheme).
    system.schedule(WindowEvent::TextInput {
        text: "rust-lang.org".to_string(),
    });
    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(session.omnibox().text(), "rust-lang.org");

    // 3. Press Enter.
    system.schedule(WindowEvent::ControlKey {
        key: LogicalKey::Enter,
        state: KeyState::Pressed,
    });
    pump(&mut system, &mut presenter, &mut session);

    // Focus must be released immediately.
    assert!(
        !session.omnibox().is_focused(),
        "pressing Enter must release omnibox focus"
    );

    // Navigation must be classified as https://rust-lang.org/ and completed.
    assert_eq!(
        completed_navigation(
            &mut system,
            &mut presenter,
            &mut session,
            navigations_before
        ),
        target,
        "Enter must classify input and dispatch navigation to classified destination"
    );
}

#[test]
fn control_keys_backspace_and_arrows_operate_on_focused_omnibox() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = loaded_session(attributes.initial_size());

    pump(&mut system, &mut presenter, &mut session);

    // Focus via Ctrl+L and type "cat"
    system.schedule(WindowEvent::Key {
        code: KeyCode::CONTROL_LEFT,
        pressed: true,
    });
    system.schedule(WindowEvent::Key {
        code: KeyCode::KEY_L,
        pressed: true,
    });
    system.schedule(WindowEvent::TextInput {
        text: "cat".to_string(),
    });
    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(session.omnibox().text(), "cat");

    // Backspace removes 't' -> "ca"
    system.schedule(WindowEvent::ControlKey {
        key: LogicalKey::Backspace,
        state: KeyState::Pressed,
    });
    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(session.omnibox().text(), "ca");

    // ArrowLeft moves cursor between 'c' and 'a', then type 'o' -> "coa"
    system.schedule(WindowEvent::ControlKey {
        key: LogicalKey::ArrowLeft,
        state: KeyState::Pressed,
    });
    system.schedule(WindowEvent::TextInput {
        text: "o".to_string(),
    });
    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(session.omnibox().text(), "coa");
}

#[test]
fn clicks_in_chrome_region_target_omnibox_and_do_not_trigger_links() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let mut session = session_over(MockTransport::new(), attributes.initial_size());
    load(
        &mut session,
        "<html><body><a href=\"target.html\" style=\"display: block; width: 100px; height: 50px;\">Click me</a></body></html>",
        &page_url("index.html"),
    );
    pump(&mut system, &mut presenter, &mut session);
    assert!(!session.omnibox().is_focused());
    let navigations_before_click = session.stats().navigations;

    // Click at y=20.0 (in the chrome area, y < CHROME_HEIGHT=42)
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, 20.0),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });
    pump(&mut system, &mut presenter, &mut session);

    assert!(
        session.omnibox().is_focused(),
        "clicking within the chrome bar must focus the omnibox"
    );
    assert_eq!(
        session.stats().navigations,
        navigations_before_click,
        "clicking in the chrome area must not trigger links underneath in the document"
    );
}

#[test]
fn clicks_below_chrome_region_trigger_link_with_offset_compensation() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let target = network::Url::parse("http://example.com/target.html").unwrap();
    let response = network::HttpResponse::new(
        network::StatusCode::OK,
        network::HeaderMap::new(),
        network::Body::from_text("<html><body>target</body></html>"),
    );
    let mut session = session_over(
        MockTransport::new().with_response(target.clone(), response),
        attributes.initial_size(),
    );
    load(
        &mut session,
        "<html><body><a href=\"target.html\" style=\"display: block; width: 100px; height: 50px;\">Click me</a></body></html>",
        "http://example.com/index.html",
    );
    pump(&mut system, &mut presenter, &mut session);
    let navigations_before_click = session.stats().navigations;

    // In window coordinates, the link (0..50 in document y) begins at CHROME_HEIGHT=42.
    // y = 62.0 corresponds to document y = 20.0, which hits the link.
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, 62.0),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        completed_navigation(
            &mut system,
            &mut presenter,
            &mut session,
            navigations_before_click
        ),
        target,
        "clicking below chrome height must trigger the link with offset compensation"
    );
}

#[test]
fn session_maintains_ui_render_tree_hierarchy() {
    let attributes = initial_window_attributes().unwrap();
    let session = session_over(MockTransport::new(), attributes.initial_size());
    let tree = session.ui_tree();
    assert_eq!(tree.root().kind(), &super::UiNodeKind::Window);
    assert_eq!(tree.chrome_height(), super::ChromeHeight::DEFAULT);
}

#[test]
fn resizing_below_chrome_height_updates_ui_tree_to_fallback() {
    let attributes = initial_window_attributes().unwrap();
    let mut session = session_over(MockTransport::new(), attributes.initial_size());
    let small_size = SurfaceSize::new(800, 30).unwrap();
    session.resize(small_size);

    assert_eq!(session.ui_tree().viewport(), small_size);
    assert!(session.ui_tree().content_size().is_err());
    assert_eq!(
        session
            .ui_tree()
            .hit_test(window::PhysicalPosition::new(20.0, 20.0)),
        None
    );

    // Resizing back to a valid viewport restores the full hierarchy
    let normal_size = SurfaceSize::new(800, 600).unwrap();
    session.resize(normal_size);
    assert_eq!(session.ui_tree().viewport(), normal_size);
    assert!(session.ui_tree().content_size().is_ok());
    assert_eq!(
        session
            .ui_tree()
            .hit_test(window::PhysicalPosition::new(20.0, 20.0)),
        Some(super::UiHitTarget::Omnibox)
    );
}
