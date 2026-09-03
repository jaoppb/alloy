//! A backend-agnostic conformance suite — `ADR-0011` item 6, guarding **C-14**.
//!
//! Ordinary library code, not `#[cfg(test)]`, so an adapter crate can call it
//! from its own `tests/`. Same shape and same reason as
//! `core/engine/src/conformance.rs`, which `RhaiEngine` and `MockEngine` both
//! run.
//!
//! ```text
//! #[test]
//! fn my_backend_passes_conformance() {
//!     graphics::conformance::run_backend_suite(&mut MyBackend::new());
//! }
//! ```
//!
//! What it pins is the **frame lifecycle** and the guarantees every tier owes a
//! caller. What it deliberately does not pin is *pixels*: the software
//! rasterizer's output is checked by golden images, and a recording or a GPU
//! backend cannot be expected to produce the same bytes from the same list until
//! `I6` says it must.
//!
//! Unlike `run_core_suite`, this takes the backend itself rather than a factory:
//! the trait is object-safe, so `&mut dyn RenderBackend` is a usable handle and
//! there is nothing to construct generically.

// An assertion suite that happens to be `pub` (so adapters can call it from
// their `tests/`) rather than `#[cfg(test)]`: it panics on the first violation
// by design. Same intent as `clippy.toml`'s `allow-*-in-tests`, and the same
// carve-out `core/engine/src/conformance.rs:32-40` already takes.
#![allow(clippy::panic, clippy::expect_used)]

use crate::application::builder::{DisplayListBuilder, PxRect};
use crate::application::ports::RenderBackend;
use crate::domain::color::Color;
use crate::domain::display_list::DisplayList;
use crate::domain::error::GraphicsError;
use crate::domain::geometry::SurfaceSize;

/// Runs every rule a [`RenderBackend`] must obey.
///
/// Panics on the first violation, naming the rule that was broken.
pub fn run_backend_suite(backend: &mut dyn RenderBackend) {
    check_tier_is_stable(backend);
    check_a_frame_round_trips(backend);
    check_an_empty_list_is_accepted(backend);
    check_read_back_matches_the_requested_surface(backend);
    check_the_backend_is_reusable_across_frames(backend);
    check_submit_before_begin_frame_is_refused(backend);
    check_end_frame_before_begin_frame_is_refused(backend);
    check_a_second_begin_frame_is_refused(backend);
}

/// A 2×2 surface — the smallest one that has more than one row, so a backend
/// that confuses stride with width fails here rather than in a golden image.
const fn tiny_surface() -> SurfaceSize {
    SurfaceSize::new(2, 2).expect("2×2 is a valid surface")
}

/// A list every backend must accept: one opaque rectangle.
fn minimal_list() -> DisplayList {
    let mut builder = DisplayListBuilder::new();
    builder
        .draw_rect(PxRect::from_px(0.0, 0.0, 1.0, 1.0), Color::BLACK)
        .expect("a unit rectangle at the origin is always valid");
    builder.build().expect("a balanced list always builds")
}

/// Drives one complete frame, panicking with `stage` on the first failure.
fn paint_one_frame(backend: &mut dyn RenderBackend, stage: &str) {
    backend
        .begin_frame(tiny_surface())
        .unwrap_or_else(|error| panic!("{stage}: begin_frame must succeed: {error}"));
    backend.submit(&minimal_list()).unwrap_or_else(|error| {
        panic!("{stage}: submitting a plain rectangle must succeed: {error}")
    });
    backend
        .end_frame()
        .unwrap_or_else(|error| panic!("{stage}: end_frame must succeed: {error}"));
}

fn check_tier_is_stable(backend: &dyn RenderBackend) {
    assert_eq!(
        backend.tier(),
        backend.tier(),
        "a backend's tier must not change between calls"
    );
}

fn check_a_frame_round_trips(backend: &mut dyn RenderBackend) {
    paint_one_frame(backend, "round trip");

    backend
        .read_back()
        .unwrap_or_else(|error| panic!("read_back after end_frame must succeed: {error}"));
}

fn check_an_empty_list_is_accepted(backend: &mut dyn RenderBackend) {
    backend
        .begin_frame(tiny_surface())
        .expect("begin_frame must succeed");

    backend
        .submit(&DisplayList::empty())
        .unwrap_or_else(|error| {
            panic!("an empty list paints nothing, it is not an error: {error}")
        });

    backend.end_frame().expect("end_frame must succeed");
}

fn check_read_back_matches_the_requested_surface(backend: &mut dyn RenderBackend) {
    paint_one_frame(backend, "surface size");

    let frame = backend.read_back().expect("read_back must succeed");

    assert_eq!(
        frame.size(),
        tiny_surface(),
        "read_back must return a frame the size begin_frame was given"
    );
}

fn check_the_backend_is_reusable_across_frames(backend: &mut dyn RenderBackend) {
    paint_one_frame(backend, "first frame");
    backend.read_back().expect("first read_back must succeed");

    paint_one_frame(backend, "second frame");

    backend
        .read_back()
        .expect("a backend must be reusable: a second frame must work like the first");
}

fn check_submit_before_begin_frame_is_refused(backend: &mut dyn RenderBackend) {
    reset(backend);

    let error = expect_refusal(backend.submit(&minimal_list()), "submit before begin_frame");

    assert_out_of_order(&error, "submit");
}

fn check_end_frame_before_begin_frame_is_refused(backend: &mut dyn RenderBackend) {
    reset(backend);

    let error = expect_refusal(backend.end_frame(), "end_frame before begin_frame");

    assert_out_of_order(&error, "end_frame");
}

fn check_a_second_begin_frame_is_refused(backend: &mut dyn RenderBackend) {
    reset(backend);
    backend
        .begin_frame(tiny_surface())
        .expect("the first begin_frame must succeed");

    let error = expect_refusal(backend.begin_frame(tiny_surface()), "nested begin_frame");

    assert_out_of_order(&error, "begin_frame");
    backend.end_frame().expect("the frame still closes cleanly");
}

/// Leaves the backend idle, whatever state the previous check left it in.
fn reset(backend: &mut dyn RenderBackend) {
    let _ = backend.end_frame();
    let _ = backend.read_back();
}

fn expect_refusal<T>(outcome: Result<T, GraphicsError>, what: &str) -> GraphicsError {
    match outcome {
        Ok(_) => panic!("{what} must be refused, not accepted"),
        Err(error) => error,
    }
}

fn assert_out_of_order(error: &GraphicsError, attempted: &str) {
    let GraphicsError::FrameOutOfOrder {
        attempted: named, ..
    } = error
    else {
        panic!("{attempted} out of order must be FrameOutOfOrder, got {error:?}");
    };
    assert_eq!(
        named.name(),
        attempted,
        "the error must name the operation that was attempted"
    );
}
