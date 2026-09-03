//! **C-17** (`PRD-005:90`): the fall to `SoftwareCpuBackend` is the real
//! algorithm falling through, not a tautology.
//!
//! The distinction this file exists to prove: a cascade that simply returned the
//! software backend would satisfy the *words* of C-17 while testing nothing.
//! So every test here makes specific rungs refuse and then asserts **which**
//! rung answered — including the case that catches the most likely defect, a
//! cascade that jumps straight to the last rung when the first one fails.
//!
//! Rung constructors are injected rather than forced through
//! `GRAPHICS_FORCE_TIER`, so these tests carry no process-wide state and cannot
//! interfere with each other under `cargo test`'s default parallelism.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use core::cell::RefCell;

use graphics::{
    BackendPreference, BackendTier, DisplayList, DisplayListBuilder, GraphicsError, PxRect,
    RenderBackend, SurfaceSize, select_backend, select_backend_with,
};

mod recording_backend;

use recording_backend::RecordingBackend;

/// A constructor that succeeds only for the tiers in `available`.
fn only(
    available: &[BackendTier],
) -> impl Fn(BackendTier) -> Result<Box<dyn RenderBackend>, GraphicsError> + '_ {
    move |tier| {
        if available.contains(&tier) {
            return Ok(Box::new(TieredBackend::new(tier)));
        }
        Err(GraphicsError::BackendUnavailable { tier })
    }
}

/// A backend that reports an arbitrary tier, so a test can tell the rungs apart.
struct TieredBackend {
    tier: BackendTier,
    inner: RecordingBackend,
}

impl TieredBackend {
    const fn new(tier: BackendTier) -> Self {
        Self {
            tier,
            inner: RecordingBackend::new(),
        }
    }
}

impl RenderBackend for TieredBackend {
    fn tier(&self) -> BackendTier {
        self.tier
    }

    fn begin_frame(&mut self, size: SurfaceSize) -> Result<(), GraphicsError> {
        self.inner.begin_frame(size)
    }

    fn submit(&mut self, list: &DisplayList) -> Result<(), GraphicsError> {
        self.inner.submit(list)
    }

    fn end_frame(&mut self) -> Result<(), GraphicsError> {
        self.inner.end_frame()
    }

    fn read_back(&self) -> Result<graphics::Framebuffer, GraphicsError> {
        self.inner.read_back()
    }
}

#[test]
fn the_top_rung_wins_when_it_is_available() {
    let selection = select_backend_with(BackendPreference::Automatic, &only(&BackendTier::CASCADE))
        .expect("every rung is available");

    assert_eq!(selection.tier(), BackendTier::Vulkan);
    assert!(
        selection.skipped().is_empty(),
        "nothing was skipped, so nothing should be reported"
    );
}

#[test]
fn a_failing_top_rung_yields_the_next_one_and_not_the_last() {
    // The defect this catches: a cascade written as "try Vulkan, else software"
    // passes every other test in this file and fails only here.
    let selection = select_backend_with(
        BackendPreference::Automatic,
        &only(&[BackendTier::OpenGl, BackendTier::Software]),
    )
    .expect("OpenGL is available");

    assert_eq!(
        selection.tier(),
        BackendTier::OpenGl,
        "losing Vulkan must cost exactly one rung"
    );
    assert_eq!(selection.skipped().len(), 1);
}

#[test]
fn losing_both_gpu_rungs_falls_all_the_way_to_software() {
    let selection = select_backend_with(
        BackendPreference::Automatic,
        &only(&[BackendTier::Software]),
    )
    .expect("software is always the floor");

    assert_eq!(selection.tier(), BackendTier::Software);
    assert_eq!(
        selection
            .skipped()
            .iter()
            .map(|(tier, _)| *tier)
            .collect::<Vec<_>>(),
        vec![BackendTier::Vulkan, BackendTier::OpenGl],
        "both GPU rungs must be reported as attempted and refused"
    );
}

#[test]
fn every_skipped_rung_carries_the_reason_it_refused() {
    let selection = select_backend_with(
        BackendPreference::Automatic,
        &only(&[BackendTier::Software]),
    )
    .unwrap();

    for (tier, reason) in selection.skipped() {
        assert_eq!(
            reason,
            &GraphicsError::BackendUnavailable { tier: *tier },
            "the diagnostic PRD-005:33-58 asks for must name the rung and the cause"
        );
    }
}

#[test]
fn an_exhausted_cascade_reports_a_typed_error_rather_than_panicking() {
    let error = select_backend_with(BackendPreference::Automatic, &only(&[]))
        .expect_err("no rung is available");

    assert_eq!(
        error,
        GraphicsError::BackendUnavailable {
            tier: BackendTier::Software
        },
        "the last rung tried is the surprising one, so it is the one named"
    );
}

#[test]
fn starting_at_software_never_probes_a_gpu() {
    let probed = RefCell::new(Vec::new());

    let selection = select_backend_with(
        BackendPreference::StartingAt(BackendTier::Software),
        &|tier| {
            probed.borrow_mut().push(tier);
            Ok(Box::new(TieredBackend::new(tier)))
        },
    )
    .expect("software is available");

    assert_eq!(selection.tier(), BackendTier::Software);
    assert_eq!(
        probed.into_inner(),
        vec![BackendTier::Software],
        "a headless caller must not pay for a GPU probe it excluded"
    );
}

#[test]
fn starting_at_opengl_skips_vulkan_but_still_falls_to_software() {
    let selection = select_backend_with(
        BackendPreference::StartingAt(BackendTier::OpenGl),
        &only(&[BackendTier::Vulkan, BackendTier::Software]),
    )
    .expect("software is reachable");

    assert_eq!(
        selection.tier(),
        BackendTier::Software,
        "an available Vulkan must not be selected when it was excluded"
    );
    assert_eq!(
        selection
            .skipped()
            .iter()
            .map(|(tier, _)| *tier)
            .collect::<Vec<_>>(),
        vec![BackendTier::OpenGl],
        "Vulkan was never attempted, so it is not in the skipped list"
    );
}

#[test]
fn the_real_cascade_on_this_machine_lands_on_software_and_renders() {
    // The end-to-end shape of C-17: with `vulkan.rs` and `opengl.rs` reporting
    // themselves unavailable, the production `select_backend` walks past both
    // and returns something that actually paints.
    let selection = select_backend(BackendPreference::Automatic)
        .expect("the software rung is linked in this build");
    let skipped: Vec<BackendTier> = selection.skipped().iter().map(|(tier, _)| *tier).collect();
    let mut backend = selection.into_backend();

    assert_eq!(skipped, vec![BackendTier::Vulkan, BackendTier::OpenGl]);
    assert_eq!(backend.tier(), BackendTier::Software);

    let mut builder = DisplayListBuilder::new();
    builder
        .draw_rect(PxRect::from_px(0.0, 0.0, 1.0, 1.0), graphics::Color::BLACK)
        .unwrap();
    let list = builder.build().unwrap();
    let surface = SurfaceSize::new(1, 1).unwrap();

    backend.begin_frame(surface).unwrap();
    backend.submit(&list).unwrap();
    backend.end_frame().unwrap();

    assert_eq!(
        backend.read_back().unwrap().pixel(0, 0),
        Some(graphics::Color::BLACK),
        "the page must still render after falling two rungs"
    );
}

#[test]
fn the_boxed_backend_the_cascade_returns_passes_conformance() {
    let mut backend = select_backend(BackendPreference::Automatic)
        .expect("software is linked")
        .into_backend();

    graphics::conformance::run_backend_suite(backend.as_mut());
}

#[test]
fn an_unrecognised_force_tier_value_is_ignored_rather_than_fatal() {
    // Read through `BackendTier::parse`, which `from_environment` delegates to:
    // a typo in an operator's environment must not stop a browser from starting.
    assert_eq!(BackendTier::parse("metal"), None);
    assert_eq!(BackendTier::parse(""), None);
    assert_eq!(graphics::FORCE_TIER_VARIABLE, "GRAPHICS_FORCE_TIER");
    assert_eq!(BackendPreference::default(), BackendPreference::Automatic);
}
