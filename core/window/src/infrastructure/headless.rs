//! [`HeadlessWindowSystem`] and [`RecordingPresenter`] — the reference
//! adapters this crate's own conformance suite runs in CI, and the `no-window`
//! build's only implementations.
//!
//! Neither adapter opens a real window or surface: `HeadlessWindowSystem`
//! plays back a scripted [`WindowEvent`] queue (auto-seeded with a `Resized`
//! at `create_window`, exactly what a real backend delivers first), and
//! `RecordingPresenter` keeps the last frame it was given for a golden
//! comparison — the same role `graphics`'s `RecordingBackend` plays for
//! `RenderBackend` (`core/graphics/tests/recording_backend.rs`).

use std::collections::VecDeque;

use crate::application::ports::{Presenter, PumpStatus, WindowSystem};
use crate::domain::attributes::{WindowAttributes, WindowId};
use crate::domain::error::{WindowError, WindowOperation};
use crate::domain::event::WindowEvent;
use crate::domain::frame::FrameView;

/// A scripted, in-memory [`WindowSystem`] — no OS window, no event loop.
#[derive(Debug, Default)]
pub struct HeadlessWindowSystem {
    window: Option<WindowId>,
    next_window_id: u64,
    scripted_events: VecDeque<WindowEvent>,
    exited: bool,
}

impl HeadlessWindowSystem {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `event` to be handed to the next
    /// [`WindowSystem::pump_events`] call's sink, after any already queued.
    /// A test uses this to script pointer/keyboard/scroll traffic that a real
    /// backend would otherwise have to be driven through a display server to
    /// produce.
    pub fn schedule(&mut self, event: WindowEvent) {
        self.scripted_events.push_back(event);
    }
}

impl WindowSystem for HeadlessWindowSystem {
    fn create_window(&mut self, attrs: &WindowAttributes) -> Result<WindowId, WindowError> {
        let id = WindowId::from_raw(self.next_window_id);
        let next = self
            .next_window_id
            .checked_add(1)
            .ok_or_else(|| WindowError::creation_failed("window id counter exhausted"))?;
        self.next_window_id = next;
        self.window = Some(id);
        self.scripted_events
            .push_back(WindowEvent::Resized(attrs.initial_size()));
        Ok(id)
    }

    fn pump_events(
        &mut self,
        sink: &mut dyn FnMut(WindowEvent),
    ) -> Result<PumpStatus, WindowError> {
        if self.window.is_none() {
            return Err(WindowError::no_window_yet(WindowOperation::PumpEvents));
        }
        if self.exited {
            return Err(WindowError::EventLoopExited);
        }
        while let Some(event) = self.scripted_events.pop_front() {
            let is_close = matches!(event, WindowEvent::CloseRequested);
            sink(event);
            if is_close {
                self.exited = true;
                return Ok(PumpStatus::Exit);
            }
        }
        Ok(PumpStatus::Continue)
    }
}

/// A [`Presenter`] that records the last [`FrameView`] it was given, owned
/// (not borrowed) so it outlives the `present` call — a test reads it back
/// with [`Self::last_frame`] for a golden comparison.
#[derive(Debug, Default)]
pub struct RecordingPresenter {
    last_frame: Option<RecordedFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedFrame {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

impl RecordingPresenter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The most recent frame this presenter was given, as a fresh borrowed
    /// [`FrameView`], or `None` before the first [`Presenter::present`] call.
    #[must_use]
    pub fn last_frame(&self) -> Option<FrameView<'_>> {
        let recorded = self.last_frame.as_ref()?;
        FrameView::new(recorded.width, recorded.height, &recorded.pixels)
    }
}

impl Presenter for RecordingPresenter {
    fn present(&mut self, frame: FrameView<'_>) -> Result<(), WindowError> {
        self.last_frame = Some(RecordedFrame {
            width: frame.width(),
            height: frame.height(),
            pixels: frame.pixels().to_vec(),
        });
        Ok(())
    }
}
