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

use std::sync::{Arc, mpsc};

use graphics::{ImageId, SyntheticFontProvider};
use network::{AllowAllPolicy, MockTransport};
use window::{
    HeadlessWindowSystem, RecordingPresenter, SurfaceSize, WindowEvent, WindowSystem as _,
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

/// The URL a link click navigated to. `pump_once` spawns the navigation
/// and then drains the very channel the test reads, so a navigation thread
/// that finishes before that `try_recv` is applied inside the pump rather
/// than left for the test: read the result from whichever side got it, or
/// the outcome depends on thread scheduling. The click's navigation is the
/// first one past `navigations_before_click`.
fn completed_navigation(
    receiver: &mpsc::Receiver<LoopMessage>,
    session: &TestSession,
    navigations_before_click: usize,
) -> network::Url {
    if session.stats().navigations > navigations_before_click {
        return session
            .base_url()
            .cloned()
            .expect("an applied navigation sets the document base");
    }
    let message = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("navigation message received");
    let LoopMessage::Navigation(Ok((_, target_url))) = message else {
        panic!("expected a successful navigation");
    };
    target_url
}

/// Installs `markup` as a navigated document at `url`, the same way a
/// finished navigation thread does.
fn load(session: &mut TestSession, markup: &str, url: &str) {
    let (sender, _receiver) = mpsc::channel();
    let document = dom::parse(markup).unwrap().into_tree();
    let url = network::Url::parse(url).unwrap();
    session.apply(LoopMessage::Navigation(Ok((document, url))), &sender);
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
    let (sender, receiver) = mpsc::channel();
    let mut session = loaded_session(attributes.initial_size());

    pump_once(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    )
    .unwrap();

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
    let (sender, receiver) = mpsc::channel();
    let mut session = loaded_session(attributes.initial_size());
    for index in 0..50u32 {
        sender
            .send(LoopMessage::Image(
                ImageId::new(index),
                Ok(placeholder_framebuffer()),
            ))
            .unwrap();
    }

    pump_once(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    )
    .unwrap();

    assert_eq!(
        session.stats().relayouts,
        1,
        "fifty coalesced image arrivals must cost exactly one relayout, not fifty"
    );
}

fn pump(
    system: &mut HeadlessWindowSystem,
    presenter: &mut RecordingPresenter,
    receiver: &mpsc::Receiver<LoopMessage>,
    sender: &mpsc::Sender<LoopMessage>,
    session: &mut TestSession,
) {
    pump_once(system, presenter, receiver, sender, session).unwrap();
}

#[test]
fn a_redraw_request_repaints_the_cached_frame_without_a_relayout() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let (sender, receiver) = mpsc::channel();
    let mut session = loaded_session(attributes.initial_size());

    // First pump: the auto-seeded Resized lays out and presents once.
    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    );
    assert_eq!(session.stats().relayouts, 1);
    assert_eq!(presenter.present_count(), 1);

    system.schedule(WindowEvent::RedrawRequested);
    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    );

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
    let (sender, receiver) = mpsc::channel();
    // No document yet.
    let mut session = session_over(MockTransport::new(), attributes.initial_size());

    system.schedule(WindowEvent::RedrawRequested);
    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    );

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
    let (sender, receiver) = mpsc::channel();
    let mut session = loaded_session(attributes.initial_size());

    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    );
    assert_eq!(presenter.present_count(), 1);

    // Nothing new scheduled: the redraw the relayout re-armed is the only
    // thing this pump sees, and it must repaint (not relayout).
    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    );

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
    let (sender, receiver) = mpsc::channel();
    let mut session = loaded_session(attributes.initial_size());

    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    );
    let presents_after_load = presenter.present_count();

    for _ in 0..50 {
        system.schedule(WindowEvent::RedrawRequested);
    }
    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    );

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
    let (sender, receiver) = mpsc::channel();
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
    pump_once(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    )
    .unwrap();

    assert!(session.has_links(), "link target must be collected");

    // Move pointer over the link and click
    let navigations_before_click = session.stats().navigations;
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, 20.0),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });

    pump_once(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    )
    .unwrap();

    let expected = network::Url::parse("http://example.com/target.html").unwrap();
    assert_eq!(
        completed_navigation(&receiver, &session, navigations_before_click),
        expected
    );
}

#[test]
fn a_link_click_resolves_against_the_documents_base_href_not_the_navigation_url() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();

    let mut presenter = RecordingPresenter::new();
    let (sender, receiver) = mpsc::channel();
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
    let document = dom::parse(
        "<html><head><base href=\"https://cdn.example/app/\"></head><body>\
         <a href=\"docs.html\" style=\"display: block; width: 100px; height: 50px;\">Docs</a>\
         </body></html>",
    )
    .unwrap()
    .into_tree();
    let navigation_url = network::Url::parse("https://example.com/index.html").unwrap();
    session.apply(
        LoopMessage::Navigation(Ok((document, navigation_url))),
        &sender,
    );

    assert_eq!(
        session.base_url().map(ToString::to_string).as_deref(),
        Some("https://cdn.example/app/"),
        "the session keeps the `<base href>`, not the navigation URL"
    );

    pump_once(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    )
    .unwrap();
    let navigations_before_click = session.stats().navigations;
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, 20.0),
    });
    system.schedule(WindowEvent::PointerButton {
        button: window::PointerButton::Left,
        pressed: true,
    });
    pump_once(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    )
    .unwrap();

    assert_eq!(
        completed_navigation(&receiver, &session, navigations_before_click),
        expected
    );
}

#[test]
fn a_resize_before_any_document_counts_no_relayout() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let (sender, receiver) = mpsc::channel();
    // The auto-seeded Resized arrives while the navigation is still in flight.
    let mut session = session_over(MockTransport::new(), attributes.initial_size());

    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &sender,
        &mut session,
    );

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
