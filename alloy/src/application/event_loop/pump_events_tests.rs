//! The fold alone: what one drain of window events collapses to.
#![allow(clippy::unwrap_used)]

use window::{PhysicalPosition, PointerButton, PumpStatus, SurfaceSize, WindowEvent};

use super::pump_events::PumpEvents;

fn size(width: u32) -> SurfaceSize {
    SurfaceSize::new(width, 600).unwrap()
}

fn click() -> WindowEvent {
    WindowEvent::PointerButton {
        button: PointerButton::Left,
        pressed: true,
    }
}

#[test]
fn only_the_last_resize_of_a_batch_counts() {
    let mut events = PumpEvents::default();

    events.observe(&WindowEvent::Resized(size(1024)));
    events.observe(&WindowEvent::Resized(size(640)));

    assert_eq!(events.take_input().latest_resize(), Some(size(640)));
}

#[test]
fn a_click_lands_where_the_pointer_was_when_it_came() {
    let mut events = PumpEvents::default();
    let first = PhysicalPosition::new(1.0, 2.0);

    events.observe(&WindowEvent::PointerMoved { position: first });
    events.observe(&click());
    events.observe(&WindowEvent::PointerMoved {
        position: PhysicalPosition::new(9.0, 9.0),
    });

    assert_eq!(
        events.pointer_position(),
        Some(PhysicalPosition::new(9.0, 9.0))
    );
    assert_eq!(events.take_input().clicked_at(), Some(first));
}

#[test]
fn many_redraw_requests_are_one_repaint_and_a_close_exits() {
    let mut events = PumpEvents::default();
    events.observe(&WindowEvent::RedrawRequested);
    events.observe(&WindowEvent::RedrawRequested);
    assert!(events.needs_repaint());
    assert_eq!(events.status(PumpStatus::Continue), PumpStatus::Continue);

    events.observe(&WindowEvent::CloseRequested);

    assert_eq!(events.status(PumpStatus::Continue), PumpStatus::Exit);
}

#[test]
fn a_batch_with_no_events_did_no_work_and_a_fetch_alone_is_work() {
    let mut events = PumpEvents::default();
    assert!(!events.did_work());

    events.observe_fetch();

    assert!(events.did_work());
}
