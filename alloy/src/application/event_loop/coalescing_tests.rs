//! Direct, thread-free proofs of the one invariant `06-i4-alloy-url.md` names
//! twice.
//!
//! Resize and subresource-arrival coalescing collapse an arbitrary burst of
//! events into **at most one** relayout per pump cycle. Exercising
//! `pump_once` straight (rather than the threaded `BrowserApp::run`) is
//! deliberate: the property under test is what one drain-and-decide cycle
//! does with whatever is already queued, which has nothing to do with how
//! fast a background fetch thread happens to run — testing it through real
//! threads would make the proof racy for no added coverage.
#![allow(clippy::unwrap_used)]

use browser::{FetchPayload, FetchResult};
use graphics::{Color, Framebuffer, ImageId};
use window::{
    HeadlessWindowSystem, RecordingPresenter, SurfaceSize, WindowEvent, WindowSystem as _,
};

use crate::application::test_support::{
    FakeWeb, app_over, deliver, loaded_app, loaded_window, pump,
};
use crate::composition::initial_window_attributes;

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
    let mut app = loaded_app(attributes.initial_size());

    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        app.stats().relayouts(),
        1,
        "three coalesced Resized events (the initial one plus two scheduled) must cost exactly one relayout"
    );
    assert_eq!(
        app.viewport(),
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
    let mut app = loaded_app(attributes.initial_size());
    for index in 0..50u32 {
        let image = FetchPayload::Image(ImageId::new(index), Ok(placeholder_framebuffer()));
        let navigation = app.browsing().current_navigation();
        deliver(&app, FetchResult::new(navigation, image));
    }

    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        app.stats().relayouts(),
        1,
        "fifty coalesced image arrivals must cost exactly one relayout, not fifty"
    );
}

#[test]
fn a_redraw_request_repaints_the_cached_frame_without_a_relayout() {
    let (mut system, mut presenter, mut app) = loaded_window();

    // First pump: the auto-seeded Resized lays out and presents once.
    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(app.stats().relayouts(), 1);
    assert_eq!(presenter.present_count(), 1);

    system.schedule(WindowEvent::RedrawRequested);
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        app.stats().relayouts(),
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
    let mut app = app_over(FakeWeb::new(), attributes.initial_size());

    system.schedule(WindowEvent::RedrawRequested);
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        presenter.present_count(),
        0,
        "a RedrawRequested with nothing rendered yet must present nothing"
    );
}

#[test]
fn a_relayout_arms_a_following_repaint() {
    let (mut system, mut presenter, mut app) = loaded_window();

    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(presenter.present_count(), 1);

    // Nothing new scheduled: the redraw the relayout re-armed is the only
    // thing this pump sees, and it must repaint (not relayout).
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        app.stats().relayouts(),
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
    let (mut system, mut presenter, mut app) = loaded_window();

    pump(&mut system, &mut presenter, &mut app);
    let presents_after_load = presenter.present_count();

    for _ in 0..50 {
        system.schedule(WindowEvent::RedrawRequested);
    }
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(app.stats().relayouts(), 1);
    assert_eq!(
        presenter.present_count(),
        presents_after_load + 1,
        "fifty coalesced RedrawRequested events cost exactly one repaint"
    );
}

#[test]
fn a_resize_before_any_document_counts_no_relayout() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();
    // The auto-seeded Resized arrives while the navigation is still in flight.
    let mut app = app_over(FakeWeb::new(), attributes.initial_size());

    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        app.stats().relayouts(),
        0,
        "nothing was presented, so `run_until_first_frame` must keep waiting"
    );
    assert_eq!(presenter.present_count(), 0);

    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(
        presenter.present_count(),
        0,
        "the dropped request is not retried"
    );
}
