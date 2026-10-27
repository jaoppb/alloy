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

use browser::{FetchPayload, FetchResult};
use graphics::{Color, Framebuffer, ImageId};
use network::MockTransport;
use ui::OmniboxTheme;
use window::{
    HeadlessWindowSystem, RecordingPresenter, SurfaceSize, WindowEvent, WindowSystem as _,
};

use super::initial_window_attributes;
use super::test_support::{
    TestSession, completed_navigation, load, loaded_session, loaded_window, pump, session_over,
};
use super::worker::fetch_text;
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

fn placeholder_framebuffer() -> Framebuffer {
    let size = graphics::SurfaceSize::new(1, 1).unwrap();
    Framebuffer::filled(size, Color::TRANSPARENT).unwrap()
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
        let image = FetchPayload::Image(ImageId::new(index), Ok(placeholder_framebuffer()));
        let navigation = session.current_navigation_for_test();
        outbox.send(FetchResult::new(navigation, image)).unwrap();
    }

    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        session.stats().relayouts,
        1,
        "fifty coalesced image arrivals must cost exactly one relayout, not fifty"
    );
}

#[test]
fn a_redraw_request_repaints_the_cached_frame_without_a_relayout() {
    let (mut system, mut presenter, mut session) = loaded_window();

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
    let (mut system, mut presenter, mut session) = loaded_window();

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
    let (mut system, mut presenter, mut session) = loaded_window();

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

    // Move pointer over the link and click
    let navigations_before_click = session.stats().navigations;
    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, window_y(20.0)),
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
        position: window::PhysicalPosition::new(20.0, window_y(20.0)),
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
        !session.has_pending_damage(),
        "the dropped request is not retried"
    );
}

/// Where a point `document_y` pixels into the page sits in the window: the
/// page starts below the default theme's chrome bar.
fn window_y(document_y: f64) -> f64 {
    f64::from(OmniboxTheme::DEFAULT.chrome_height().to_px().get()) + document_y
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

/// `NavigationId` supersession is proved purely in `core/browser`; this
/// proves the session routes worker results through it.
#[test]
fn the_session_drops_a_result_stamped_for_a_navigation_the_user_left() {
    let mut session = loaded_session(initial_window_attributes().unwrap().initial_size());
    let left = session.current_navigation_for_test();
    session.navigate(network::Url::parse(&page_url("next.html")).unwrap());
    let late_sheet =
        FetchPayload::Stylesheet(css::MediaQuery::always(), Ok("p { color: red }".into()));
    session
        .outbox_for_test()
        .send(FetchResult::new(left, late_sheet))
        .unwrap();

    pump_queued(&mut session);

    assert_eq!(session.stats().stylesheets_loaded, 0);
    assert_ne!(session.current_navigation_for_test(), left);
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

    // y = 20 is inside the chrome bar, above the link at the top of the page.
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

    system.schedule(WindowEvent::PointerMoved {
        position: window::PhysicalPosition::new(20.0, window_y(20.0)),
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
    assert_eq!(tree.root().kind(), &ui::UiNodeKind::Window);
    assert_eq!(tree.chrome_height(), ui::ChromeHeight::DEFAULT);
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
        Some(ui::UiHitTarget::Omnibox)
    );
}
