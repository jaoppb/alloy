//! `RecordingBackend` — the in-repo reference adapter of `ADR-0011` item 6, and
//! the second implementation that makes `run_backend_suite` a real contract
//! rather than a description of one rasterizer.
//!
//! It records what it was asked to draw instead of drawing it. That is the
//! point: a backend that shares no rasterization code with `SoftwareCpuBackend`
//! and still passes the suite proves the suite pins the *port*, not an
//! implementation. It is also what keeps the port testable under the
//! `no-backend` feature, where no real rasterizer is linked at all.
//!
//! Kept in `tests/` rather than in `src/`, following `MockEngine` in
//! `core/engine/tests/mock_engine.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Also included as a module by `tier_cascade.rs`, which uses the adapter but not
// every accessor on it.
#![allow(dead_code)]

use graphics::{
    BackendTier, Color, CommandKind, DisplayList, DisplayListBuilder, FrameOperation, FrameState,
    Framebuffer, GraphicsError, PxRect, RenderBackend, SurfaceSize,
};

/// A backend that logs commands and paints a flat colour.
pub struct RecordingBackend {
    state: FrameState,
    surface: Option<SurfaceSize>,
    recorded: Vec<CommandKind>,
    fill: Color,
}

impl RecordingBackend {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: FrameState::Idle,
            surface: None,
            recorded: Vec::new(),
            fill: Color::WHITE,
        }
    }

    /// Every command kind submitted since the backend was built.
    #[must_use]
    pub fn recorded(&self) -> &[CommandKind] {
        &self.recorded
    }

    /// Refuses `attempted` unless the backend is in `required`.
    fn require(
        &self,
        attempted: FrameOperation,
        required: FrameState,
    ) -> Result<(), GraphicsError> {
        if self.state == required {
            return Ok(());
        }
        Err(GraphicsError::FrameOutOfOrder {
            attempted,
            state: self.state,
        })
    }
}

impl Default for RecordingBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderBackend for RecordingBackend {
    fn tier(&self) -> BackendTier {
        // It runs on the CPU and is always available; it simply records rather
        // than rasterizes. The tier cascade never selects it — only tests do.
        BackendTier::Software
    }

    fn begin_frame(&mut self, size: SurfaceSize) -> Result<(), GraphicsError> {
        // Valid from `Idle` *and* from `Presented`: a backend is reusable, and a
        // new frame after a read-back is the normal case. Only a nested frame —
        // `begin_frame` while still `Recording` — is refused.
        if self.state == FrameState::Recording {
            return Err(GraphicsError::FrameOutOfOrder {
                attempted: FrameOperation::BeginFrame,
                state: self.state,
            });
        }
        self.surface = Some(size);
        self.recorded.clear();
        self.state = FrameState::Recording;
        Ok(())
    }

    fn submit(&mut self, list: &DisplayList) -> Result<(), GraphicsError> {
        self.require(FrameOperation::Submit, FrameState::Recording)?;
        self.recorded
            .extend(list.iter().map(graphics::DisplayCommand::kind));
        Ok(())
    }

    fn end_frame(&mut self) -> Result<(), GraphicsError> {
        self.require(FrameOperation::EndFrame, FrameState::Recording)?;
        self.state = FrameState::Presented;
        Ok(())
    }

    fn read_back(&self) -> Result<Framebuffer, GraphicsError> {
        self.require(FrameOperation::ReadBack, FrameState::Presented)?;
        let failed = GraphicsError::ReadbackFailed { tier: self.tier() };
        let size = self.surface.ok_or_else(|| failed.clone())?;
        Framebuffer::filled(size, self.fill).ok_or(failed)
    }
}

#[test]
fn the_reference_backend_passes_the_conformance_suite() {
    graphics::conformance::run_backend_suite(&mut RecordingBackend::new());
}

#[test]
fn the_port_is_usable_behind_a_box_dyn_without_a_companion_trait() {
    // ADR-0011 item 2: `RenderBackend` is object-safe as written, so — unlike
    // `RuntimeEngine`, which needed the `dyn` companion of ADR-0013 — a boxed
    // handle is the trait itself. This is what the tier cascade returns.
    let mut backend: Box<dyn RenderBackend> = Box::new(RecordingBackend::new());

    graphics::conformance::run_backend_suite(backend.as_mut());

    assert_eq!(backend.tier(), BackendTier::Software);
}

#[test]
fn a_recording_backend_sees_exactly_the_commands_that_were_submitted() {
    let mut builder = DisplayListBuilder::new();
    builder
        .push_clip(PxRect::from_px(0.0, 0.0, 4.0, 4.0))
        .unwrap();
    builder
        .draw_rect(PxRect::from_px(1.0, 1.0, 2.0, 2.0), Color::BLACK)
        .unwrap();
    builder.pop_clip().unwrap();
    let list = builder.build().unwrap();
    let mut backend = RecordingBackend::new();
    let surface = SurfaceSize::new(4, 4).unwrap();

    backend.begin_frame(surface).unwrap();
    backend.submit(&list).unwrap();
    backend.end_frame().unwrap();

    assert_eq!(
        backend.recorded(),
        [
            CommandKind::PushClip,
            CommandKind::DrawRect,
            CommandKind::PopClip
        ],
        "a backend receives the list exactly as the builder sealed it"
    );
}

#[test]
fn read_back_returns_a_frame_the_size_of_the_surface_it_was_given() {
    let mut backend = RecordingBackend::new();
    let surface = SurfaceSize::new(3, 7).unwrap();

    backend.begin_frame(surface).unwrap();
    backend.end_frame().unwrap();
    let frame = backend.read_back().unwrap();

    assert_eq!(frame.size(), surface);
    assert_eq!(frame.width(), 3);
    assert_eq!(frame.height(), 7);
    assert_eq!(frame.pixel(2, 6), Some(Color::WHITE));
    assert_eq!(
        frame.pixel(3, 0),
        None,
        "outside the buffer is None, not a panic"
    );
}
