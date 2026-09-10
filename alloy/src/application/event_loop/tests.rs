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
use std::sync::mpsc;

use graphics::{ImageId, SyntheticFontProvider};
use network::MockTransport;
use window::{
    HeadlessWindowSystem, RecordingPresenter, SurfaceSize, WindowEvent, WindowSystem as _,
};

use super::session::{LoopMessage, Session};
use super::worker::fetch_text;
use super::{initial_window_attributes, pump_once};
use crate::application::paint::DEFAULT_FONT;
use crate::application::pipeline::DEFAULT_FONT_SIZE;
use crate::application::subresource;
use crate::error::AlloyError;

#[test]
fn a_non_2xx_stylesheet_status_is_a_typed_error_not_an_empty_body() {
    let sheet_url = network::Url::parse("http://example.com/missing.css").unwrap();
    let not_found = network::HttpResponse::new(
        network::StatusCode::NOT_FOUND,
        network::HeaderMap::new(),
        network::Body::from_text("<!doctype html><title>404</title>"),
    );
    let transport: Arc<dyn network::HttpTransport> =
        Arc::new(MockTransport::new().with_response(sheet_url.clone(), not_found));

    let result = fetch_text(&sheet_url, transport.as_ref());

    assert!(
        matches!(result, Err(AlloyError::HttpStatus { status: 404, .. })),
        "a 404 error page must not reach the CSS parser as a stylesheet"
    );
}

fn loaded_session(viewport: SurfaceSize) -> Session {
    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    let policy = Arc::new(network::AllowAllPolicy);
    let mut session = Session::new(viewport, font_provider, policy);
    session.dom_tree = Some(html::parse("<html><body>hi</body></html>").unwrap());
    session
}

fn mock_transport() -> Arc<dyn network::HttpTransport> {
    Arc::new(MockTransport::new())
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
    let transport = mock_transport();
    let mut session = loaded_session(attributes.initial_size());

    pump_once(
        &mut system,
        &mut presenter,
        &receiver,
        &transport,
        &sender,
        &mut session,
    )
    .unwrap();

    assert_eq!(
        session.stats.relayouts, 1,
        "three coalesced Resized events (the initial one plus two scheduled) must cost exactly one relayout"
    );
    assert_eq!(
        session.viewport, smaller,
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
    let transport = mock_transport();
    let mut session = loaded_session(attributes.initial_size());
    for index in 0..50u32 {
        sender
            .send(LoopMessage::Image(
                ImageId::new(index),
                Ok(subresource::placeholder_framebuffer()),
            ))
            .unwrap();
    }

    pump_once(
        &mut system,
        &mut presenter,
        &receiver,
        &transport,
        &sender,
        &mut session,
    )
    .unwrap();

    assert_eq!(
        session.stats.relayouts, 1,
        "fifty coalesced image arrivals must cost exactly one relayout, not fifty"
    );
}

fn pump(
    system: &mut HeadlessWindowSystem,
    presenter: &mut RecordingPresenter,
    receiver: &mpsc::Receiver<LoopMessage>,
    transport: &Arc<dyn network::HttpTransport>,
    sender: &mpsc::Sender<LoopMessage>,
    session: &mut Session,
) {
    pump_once(system, presenter, receiver, transport, sender, session).unwrap();
}

#[test]
fn a_redraw_request_repaints_the_cached_frame_without_a_relayout() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let (sender, receiver) = mpsc::channel();
    let transport = mock_transport();
    let mut session = loaded_session(attributes.initial_size());

    // First pump: the auto-seeded Resized lays out and presents once.
    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &transport,
        &sender,
        &mut session,
    );
    assert_eq!(session.stats.relayouts, 1);
    assert_eq!(presenter.present_count(), 1);

    system.schedule(WindowEvent::RedrawRequested);
    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &transport,
        &sender,
        &mut session,
    );

    assert_eq!(
        session.stats.relayouts, 1,
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
    let transport = mock_transport();
    let font_provider =
        Arc::new(SyntheticFontProvider::new().with_size(DEFAULT_FONT, DEFAULT_FONT_SIZE));
    let policy = Arc::new(network::AllowAllPolicy);
    // No document yet.
    let mut session = Session::new(attributes.initial_size(), font_provider, policy);

    system.schedule(WindowEvent::RedrawRequested);
    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &transport,
        &sender,
        &mut session,
    );

    assert_eq!(
        presenter.present_count(),
        0,
        "a RedrawRequested with nothing rendered yet must present nothing"
    );
    assert!(session.last_frame.is_none());
}

#[test]
fn a_relayout_arms_a_following_repaint() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    let (sender, receiver) = mpsc::channel();
    let transport = mock_transport();
    let mut session = loaded_session(attributes.initial_size());

    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &transport,
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
        &transport,
        &sender,
        &mut session,
    );

    assert_eq!(
        session.stats.relayouts, 1,
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
    let transport = mock_transport();
    let mut session = loaded_session(attributes.initial_size());

    pump(
        &mut system,
        &mut presenter,
        &receiver,
        &transport,
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
        &transport,
        &sender,
        &mut session,
    );

    assert_eq!(session.stats.relayouts, 1);
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
    let transport: Arc<dyn network::HttpTransport> =
        Arc::new(MockTransport::new().with_response(target_url, response));
    let mut session = loaded_session(attributes.initial_size());
    session.base_url = Some(network::Url::parse("http://example.com/index.html").unwrap());
    session.dom_tree = Some(
        html::parse("<html><body><a href=\"target.html\" style=\"display: block; width: 100px; height: 50px;\">Click me</a></body></html>").unwrap(),
    );
    session.dirty = true;

    // First pump: renders document and populates session.links
    pump_once(
        &mut system,
        &mut presenter,
        &receiver,
        &transport,
        &sender,
        &mut session,
    )
    .unwrap();

    assert!(!session.links.is_empty(), "link target must be collected");

    // Move pointer over the link and click
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
        &transport,
        &sender,
        &mut session,
    )
    .unwrap();

    // The click should have spawned a navigation message to receiver
    let message = receiver
        .recv_timeout(std::time::Duration::from_millis(500))
        .expect("navigation message received");
    match message {
        LoopMessage::Navigation(Ok((_, target_url))) => {
            let expected = network::Url::parse("http://example.com/target.html").unwrap();
            assert_eq!(target_url, expected);
        }
        _ => panic!("expected successful navigation to target.html"),
    }
}
