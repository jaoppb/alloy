//! [`WindowSystem`] and [`Presenter`] — the two replaceable ports of
//! `PRD-010` (`ADR-0011`), mechanism only.
//!
//! ## Threading (`ADR-0019`)
//!
//! `WindowSystem` is deliberately **not** `Send + Sync`: the window event
//! loop is the sole owner of the main thread, and a type that could cross a
//! thread boundary would invite a second loop fighting it for that thread.
//! `Presenter` **is** `Send` — it may be handed to a render worker, blitting
//! frames a different thread produced, exactly as `SoftbufferPresenter` does
//! in this crate's own `infrastructure`.
//!
//! ## Object-safety
//!
//! Every method speaks only this crate's own types, so `&mut dyn WindowSystem`
//! and `&mut dyn Presenter` compile directly (`ADR-0011` item 2) — the same
//! shape as `graphics::RenderBackend` and `network::HttpTransport`.

use crate::domain::attributes::{WindowAttributes, WindowId};
use crate::domain::error::WindowError;
use crate::domain::event::WindowEvent;
use crate::domain::frame::FrameView;

/// Owns a native window and its event queue. Not `Send + Sync` — see the
/// module doc.
pub trait WindowSystem {
    /// Creates the window, or refuses typed when the backend cannot.
    fn create_window(&mut self, attrs: &WindowAttributes) -> Result<WindowId, WindowError>;

    /// Drains events waiting on the window into `sink`, without blocking past
    /// what the backend needs to check for new ones (`ADR-0019`:
    /// `pump_events` is pull-driven from the main thread, never a callback
    /// that hands the thread away).
    fn pump_events(&mut self, sink: &mut dyn FnMut(WindowEvent))
    -> Result<PumpStatus, WindowError>;
}

/// Turns a rendered frame into pixels on screen. `Send` — see the module doc.
pub trait Presenter: Send {
    /// Presents `frame`. A backend that owns a real surface blits it; the
    /// headless reference records it for a golden comparison.
    fn present(&mut self, frame: FrameView<'_>) -> Result<(), WindowError>;
}

/// What a call to [`WindowSystem::pump_events`] found.
///
/// Not a `bool` (Object Calisthenics: no boolean standing in for a meaningful
/// choice) — mirrors `winit::platform::pump_events::PumpStatus` without
/// naming `winit` here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PumpStatus {
    /// Keep pumping — the loop has not ended.
    Continue,
    /// The backend ended the loop, typically after observing
    /// [`WindowEvent::CloseRequested`] and honouring it itself. No further
    /// [`WindowSystem::pump_events`] call is meaningful.
    Exit,
}
