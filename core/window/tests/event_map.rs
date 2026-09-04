//! `winit::event::WindowEvent` → `domain::WindowEvent` mapping totality
//! (`ADR-0011` item 2, `PRD-010`).
//!
//! Pure unit tests on `event_map::map_window_event` — no event loop, no
//! display server, so these run in ordinary CI. Only compiled under the
//! `winit-system` feature (the default): the `no-window` build links no
//! `winit` at all, so there is nothing here to test.
//!
//! `winit::event::WindowEvent::KeyboardInput` is exercised only indirectly:
//! its `KeyEvent` field has no public constructor outside the `winit` crate
//! (`platform_specific` is `pub(crate)`), so no fixture can be built for it
//! here. The exhaustive match in `map_window_event` still covers it — a
//! `winit` upgrade that renamed or removed the variant would fail to compile,
//! which is the totality guarantee this file cannot add a runtime check for.

#![cfg(feature = "winit-system")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use window::infrastructure::event_map::map_window_event;
use window::{PointerButton, SurfaceSize, WindowEvent};

#[test]
fn resized_maps_to_the_same_dimensions() {
    let event = winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize::new(800, 600));

    let mapped = map_window_event(event).expect("Resized must map to Some");

    assert_eq!(
        mapped,
        WindowEvent::Resized(SurfaceSize::new(800, 600).expect("800x600 is valid"))
    );
}

#[test]
fn a_zero_sized_resize_is_dropped_rather_than_producing_an_invalid_surface_size() {
    let event = winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize::new(0, 0));

    assert_eq!(
        map_window_event(event),
        None,
        "a 0x0 surface has no SurfaceSize reading, so this must not fabricate one"
    );
}

#[test]
fn close_requested_maps_directly() {
    let event = winit::event::WindowEvent::CloseRequested;
    assert_eq!(map_window_event(event), Some(WindowEvent::CloseRequested));
}

#[test]
fn redraw_requested_maps_directly() {
    let event = winit::event::WindowEvent::RedrawRequested;
    assert_eq!(map_window_event(event), Some(WindowEvent::RedrawRequested));
}

#[test]
fn cursor_moved_maps_to_pointer_moved_with_the_same_coordinates() {
    let event = winit::event::WindowEvent::CursorMoved {
        device_id: winit::event::DeviceId::dummy(),
        position: winit::dpi::PhysicalPosition::new(12.5, 7.0),
    };

    let mapped = map_window_event(event).expect("CursorMoved must map to Some");

    let WindowEvent::PointerMoved { position } = mapped else {
        panic!("expected PointerMoved, got {mapped:?}");
    };
    assert_eq!(position.x(), 12.5);
    assert_eq!(position.y(), 7.0);
}

#[test]
fn mouse_input_maps_button_and_pressed_state() {
    let event = winit::event::WindowEvent::MouseInput {
        device_id: winit::event::DeviceId::dummy(),
        state: winit::event::ElementState::Pressed,
        button: winit::event::MouseButton::Right,
    };

    let mapped = map_window_event(event).expect("MouseInput must map to Some");

    assert_eq!(
        mapped,
        WindowEvent::PointerButton {
            button: PointerButton::Right,
            pressed: true,
        }
    );
}

#[test]
fn mouse_wheel_line_delta_maps_to_scroll() {
    let event = winit::event::WindowEvent::MouseWheel {
        device_id: winit::event::DeviceId::dummy(),
        delta: winit::event::MouseScrollDelta::LineDelta(1.0, -2.0),
        phase: winit::event::TouchPhase::Moved,
    };

    let mapped = map_window_event(event).expect("MouseWheel must map to Some");

    let WindowEvent::Scroll { delta_x, delta_y } = mapped else {
        panic!("expected Scroll, got {mapped:?}");
    };
    assert_eq!(delta_x, 1.0);
    assert_eq!(delta_y, -2.0);
}

#[test]
fn mouse_wheel_pixel_delta_maps_to_scroll() {
    let event = winit::event::WindowEvent::MouseWheel {
        device_id: winit::event::DeviceId::dummy(),
        delta: winit::event::MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(
            3.0, 4.0,
        )),
        phase: winit::event::TouchPhase::Moved,
    };

    let mapped = map_window_event(event).expect("MouseWheel must map to Some");

    let WindowEvent::Scroll { delta_x, delta_y } = mapped else {
        panic!("expected Scroll, got {mapped:?}");
    };
    assert_eq!(delta_x, 3.0);
    assert_eq!(delta_y, 4.0);
}

/// Every variant this port has decided **not** to represent yet must still be
/// named explicitly in `map_window_event` (compile-time totality) and produce
/// `None` here (behavioural confirmation it is a reviewed, not a silent, drop).
#[test]
fn variants_not_yet_represented_in_the_domain_vocabulary_map_to_none() {
    let dropped = [
        winit::event::WindowEvent::Moved(winit::dpi::PhysicalPosition::new(0, 0)),
        winit::event::WindowEvent::Destroyed,
        winit::event::WindowEvent::HoveredFileCancelled,
        winit::event::WindowEvent::Focused(true),
        winit::event::WindowEvent::CursorEntered {
            device_id: winit::event::DeviceId::dummy(),
        },
        winit::event::WindowEvent::CursorLeft {
            device_id: winit::event::DeviceId::dummy(),
        },
        winit::event::WindowEvent::Occluded(false),
    ];

    for event in dropped {
        assert_eq!(
            map_window_event(event.clone()),
            None,
            "{event:?} is a deliberate, reviewed drop — it must map to None, not panic or map to Some"
        );
    }
}
