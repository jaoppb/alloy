//! A backend-agnostic conformance suite — `ADR-0011` item 6, guarding the
//! `PRD-010` port contract.
//!
//! Ordinary library code, not `#[cfg(test)]`, so an adapter crate can call it
//! from its own `tests/` — same shape and same reason as
//! `graphics::conformance::run_backend_suite` and
//! `network::conformance::run_transport_suite`.
//!
//! ## What this pins, and what it deliberately does not
//!
//! - **Surface-size round trip**: creating a window at size `X` must be
//!   followed by a `Resized` event reporting `X` (or the backend's real
//!   size).
//! - **Refusal ordering**: `pump_events` before any `create_window` call ever
//!   succeeded is a typed [`WindowError::NoWindowYet`], never a panic and
//!   never a hang.
//! - **Reusability**: both `system` and `presenter` survive more than one
//!   call — a second `pump_events`, a second `present`.
//!
//! What it does **not** pin is *pixels*: `Presenter::present`'s job is to
//! move bytes onto a surface, and only a golden comparison (`I4`) can judge
//! whether it drew the right ones.
//!
//! `Presenter` has no analogous "present before `create_window`" check: the
//! trait carries no reference to a `WindowSystem` to compare against (the two
//! are deliberately independent, `ADR-0019`), and the real adapter,
//! [`SoftbufferPresenter`](crate::infrastructure::softbuffer_presenter::SoftbufferPresenter),
//! makes the ordering a **type-level** impossibility instead of a runtime
//! one: its constructor takes the live window handle
//! [`WinitSystem::create_window`](crate::infrastructure::winit_system::WinitSystem::create_window)
//! produced, so there is no way to obtain one before a window exists.

#![allow(clippy::panic, clippy::expect_used)]

use crate::application::ports::{Presenter, WindowSystem};
use crate::domain::attributes::WindowAttributes;
use crate::domain::error::WindowError;
use crate::domain::event::WindowEvent;
use crate::domain::frame::FrameView;
use crate::domain::surface::SurfaceSize;

/// Runs every rule a [`WindowSystem`] / [`Presenter`] pair must obey.
///
/// Panics on the first violation, naming the rule that was broken. `system`
/// and `presenter` must both be freshly constructed — `pump_events` is
/// exercised before any window exists, which only makes sense once per
/// `system`.
pub fn run_window_suite(system: &mut dyn WindowSystem, presenter: &mut dyn Presenter) {
    check_pump_events_before_create_window_is_refused(system);
    let size = check_surface_size_round_trips(system);
    check_pump_events_is_reusable(system);
    check_presenting_is_reusable(presenter, size);
}

/// A small, easy-to-eyeball surface for the whole suite.
const fn tiny_surface_size() -> SurfaceSize {
    match SurfaceSize::new(2, 2) {
        Some(size) => size,
        None => panic!("2×2 is a valid surface size"),
    }
}

fn tiny_attributes() -> WindowAttributes {
    WindowAttributes::new("conformance probe", tiny_surface_size())
}

fn check_pump_events_before_create_window_is_refused(system: &mut dyn WindowSystem) {
    let mut sink = |_event: WindowEvent| {};

    let error = expect_refusal(
        system.pump_events(&mut sink),
        "pump_events before create_window",
    );

    assert!(
        matches!(error, WindowError::NoWindowYet { .. }),
        "pump_events before create_window must be a typed NoWindowYet refusal, got {error:?}"
    );
}

fn check_surface_size_round_trips(system: &mut dyn WindowSystem) -> SurfaceSize {
    let requested = tiny_surface_size();

    system
        .create_window(&tiny_attributes())
        .unwrap_or_else(|error| panic!("create_window must succeed: {error}"));

    let mut reported = None;
    let mut sink = |event: WindowEvent| {
        if let WindowEvent::Resized(size) = event {
            reported = Some(size);
        }
    };
    system
        .pump_events(&mut sink)
        .unwrap_or_else(|error| panic!("pump_events after create_window must succeed: {error}"));

    let reported = reported.unwrap_or_else(|| {
        panic!("create_window at {requested} must be followed by a Resized event")
    });
    assert_eq!(
        reported, requested,
        "Resized must report the size create_window was given, or the real size"
    );
    reported
}

fn check_pump_events_is_reusable(system: &mut dyn WindowSystem) {
    let mut sink = |_event: WindowEvent| {};

    system
        .pump_events(&mut sink)
        .unwrap_or_else(|error| panic!("a second pump_events call must still succeed: {error}"));
}

fn check_presenting_is_reusable(presenter: &mut dyn Presenter, size: SurfaceSize) {
    let pixel_count = size
        .pixel_count()
        .expect("a 2×2 surface always has an addressable pixel count");
    let pixels = vec![0xFFFF_FFFFu32; pixel_count];
    let frame = FrameView::new(size.width(), size.height(), &pixels)
        .expect("the pixel buffer matches the surface size");

    presenter
        .present(frame)
        .unwrap_or_else(|error| panic!("the first present must succeed: {error}"));
    presenter
        .present(frame)
        .unwrap_or_else(|error| panic!("a second present must still succeed: {error}"));
}

fn expect_refusal<T>(outcome: Result<T, WindowError>, what: &str) -> WindowError {
    match outcome {
        Ok(_) => panic!("{what} must be refused, not accepted"),
        Err(error) => error,
    }
}
