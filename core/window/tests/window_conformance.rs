//! `ADR-0011` item 6 / `PRD-010`: [`HeadlessWindowSystem`] and
//! [`RecordingPresenter`] pass the same backend-agnostic suite `WinitSystem`
//! and `SoftbufferPresenter` would. This is the suite CI actually runs — it
//! needs no display server, and runs identically under
//! `--no-default-features`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use window::{
    HeadlessWindowSystem, Presenter as _, RecordingPresenter, SurfaceSize, WindowAttributes,
    WindowError, WindowEvent, WindowSystem as _,
};

#[test]
fn the_headless_reference_pair_passes_the_conformance_suite() {
    let mut system = HeadlessWindowSystem::new();
    let mut presenter = RecordingPresenter::new();

    window::conformance::run_window_suite(&mut system, &mut presenter);
}

#[test]
fn create_window_hands_out_distinct_ids_across_systems() {
    let size = SurfaceSize::new(4, 4).expect("4x4 is valid");
    let attrs = WindowAttributes::new("probe", size);

    let mut first = HeadlessWindowSystem::new();
    let mut second = HeadlessWindowSystem::new();

    let first_id = first.create_window(&attrs).expect("create_window succeeds");
    let second_id = second
        .create_window(&attrs)
        .expect("create_window succeeds");

    // Each system starts its own id sequence at 0 — ids are only promised
    // unique *within* one system's lifetime (`ADR-0011` item 3), so this
    // documents that rather than asserting a false cross-system guarantee.
    assert_eq!(first_id, second_id);
}

#[test]
fn scheduled_events_are_delivered_in_order_after_the_automatic_resize() {
    let size = SurfaceSize::new(4, 4).expect("4x4 is valid");
    let attrs = WindowAttributes::new("probe", size);
    let mut system = HeadlessWindowSystem::new();
    system
        .create_window(&attrs)
        .expect("create_window succeeds");
    system.schedule(WindowEvent::RedrawRequested);
    system.schedule(WindowEvent::CloseRequested);

    let mut observed = Vec::new();
    let mut sink = |event: WindowEvent| observed.push(event);
    let status = system.pump_events(&mut sink).expect("pump_events succeeds");

    assert!(matches!(
        observed.as_slice(),
        [
            WindowEvent::Resized(_),
            WindowEvent::RedrawRequested,
            WindowEvent::CloseRequested,
        ]
    ));
    assert_eq!(status, window::PumpStatus::Exit);
}

#[test]
fn pump_events_after_close_requested_is_a_typed_refusal_not_a_hang() {
    let size = SurfaceSize::new(4, 4).expect("4x4 is valid");
    let attrs = WindowAttributes::new("probe", size);
    let mut system = HeadlessWindowSystem::new();
    system
        .create_window(&attrs)
        .expect("create_window succeeds");
    system.schedule(WindowEvent::CloseRequested);

    let mut sink = |_event: WindowEvent| {};
    let first = system
        .pump_events(&mut sink)
        .expect("the loop exits cleanly");
    assert_eq!(first, window::PumpStatus::Exit);

    let second = system.pump_events(&mut sink);
    assert!(
        matches!(second, Err(WindowError::EventLoopExited)),
        "pumping an exited loop must be a typed refusal, got {second:?}"
    );
}

#[test]
fn recording_presenter_has_no_frame_before_the_first_present() {
    let presenter = RecordingPresenter::new();
    assert!(presenter.last_frame().is_none());
}

#[test]
fn recording_presenter_remembers_only_the_most_recent_frame() {
    let mut presenter = RecordingPresenter::new();
    let first_pixels = [0x0000_0000u32; 4];
    let second_pixels = [0xFFFF_FFFFu32; 4];

    presenter
        .present(window::FrameView::new(2, 2, &first_pixels).expect("2x2 frame"))
        .expect("present succeeds");
    presenter
        .present(window::FrameView::new(2, 2, &second_pixels).expect("2x2 frame"))
        .expect("present succeeds");

    let last = presenter.last_frame().expect("a frame was presented");
    assert_eq!(last.pixels(), &second_pixels);
}
