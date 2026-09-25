//! [`WinitSystem`] — the [`WindowSystem`] adapter over a real `winit` event loop.
//!
//! `winit`'s own `unsafe` is the row-3 nominal exception of `ADR-0018`
//! (`unsafe-allowlist.toml`); this file itself stays `#![forbid(unsafe_code)]`
//! like the rest of the crate.
//!
//! ## Bridging `winit`'s push model to a pull-driven port (`ADR-0019`)
//!
//! `winit` 0.30 wants an [`ApplicationHandler`] driven by
//! [`EventLoopExtPumpEvents::pump_app_events`] — a window can only be created
//! from inside a callback (`resumed`), never directly by the caller. This
//! adapter buffers every mapped event `winit` delivers to
//! [`Handler::window_event`] into an internal queue, then
//! [`WinitSystem::pump_events`] pumps `winit` once and drains that queue into
//! the caller's `sink` — the callback boundary never has to see the caller's
//! closure, which keeps the bridge free of the raw pointer an `unsafe`-free
//! crate cannot otherwise stash a `&mut dyn FnMut` behind.
//!
//! [`WinitSystem::create_window`] resolves the same way: it stores the
//! requested attributes and pumps the loop (bounded, so a display-less
//! environment fails typed instead of hanging) until `resumed` has created
//! the window.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use winit::application::ApplicationHandler;
use winit::event_loop::ActiveEventLoop;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus as WinitPumpStatus};
use winit::window::Window;

use crate::application::ports::{PumpStatus, WindowSystem};
use crate::domain::attributes::{WindowAttributes, WindowId};
use crate::domain::error::{WindowError, WindowOperation};
use crate::domain::event::WindowEvent;
use crate::infrastructure::event_map::map_window_event;

/// How many bounded pumps [`WinitSystem::create_window`] tries before giving
/// up typed. Each pump has a zero timeout, so this bounds wall-clock time to
/// a handful of milliseconds even when nothing ever resumes the loop (no
/// display server).
const MAX_CREATE_WINDOW_PUMPS: u32 = 64;

/// A [`WindowSystem`] backed by a real `winit::event_loop::EventLoop`.
///
/// Must be constructed on the process's main thread on macOS and Windows
/// (`ADR-0019`) — build exactly one per process, before any other thread
/// touches the GUI.
pub struct WinitSystem {
    event_loop: winit::event_loop::EventLoop<()>,
    handler: Handler,
}

impl WinitSystem {
    /// # Errors
    ///
    /// [`WindowError::CreationFailed`] when the platform's event loop itself
    /// could not be built (no display server, a sandboxed environment, …).
    pub fn new() -> Result<Self, WindowError> {
        let event_loop = winit::event_loop::EventLoop::new()
            .map_err(|error| WindowError::creation_failed(error.to_string()))?;
        Ok(Self {
            event_loop,
            handler: Handler::default(),
        })
    }

    /// The live window handle, once [`Self::create_window`] has succeeded.
    ///
    /// A caller uses this to bind a
    /// [`SoftbufferPresenter`](crate::infrastructure::softbuffer_presenter::SoftbufferPresenter)
    /// to the same window — the seam that keeps `WindowSystem` (main-thread
    /// only) and `Presenter` (`Send`) two independently owned objects sharing
    /// one OS window (`ADR-0019`).
    #[must_use]
    pub fn window_handle(&self) -> Option<Arc<Window>> {
        self.handler.window.clone()
    }
}

impl WindowSystem for WinitSystem {
    fn create_window(&mut self, attrs: &WindowAttributes) -> Result<WindowId, WindowError> {
        if self.handler.window.is_some() {
            return Err(WindowError::creation_failed(
                "a window already exists; this adapter supports one window per WinitSystem",
            ));
        }
        self.handler.pending_attributes = Some(to_winit_attributes(attrs));

        for _ in 0..MAX_CREATE_WINDOW_PUMPS {
            let status = self
                .event_loop
                .pump_app_events(Some(Duration::ZERO), &mut self.handler);
            if let WinitPumpStatus::Exit(_) = status {
                return Err(WindowError::creation_failed(
                    "the event loop exited before a window was created",
                ));
            }
            if let Some(error) = self.handler.creation_error.take() {
                return Err(error);
            }
            if self.handler.window.is_some() {
                break;
            }
        }

        let window = self.handler.window.as_ref().ok_or_else(|| {
            WindowError::creation_failed("the platform never resumed the event loop")
        })?;
        Ok(WindowId::from_raw(u64::from(window.id())))
    }

    fn pump_events(
        &mut self,
        sink: &mut dyn FnMut(WindowEvent),
    ) -> Result<PumpStatus, WindowError> {
        if self.handler.window.is_none() {
            return Err(WindowError::no_window_yet(WindowOperation::PumpEvents));
        }

        let status = self
            .event_loop
            .pump_app_events(Some(Duration::ZERO), &mut self.handler);
        while let Some(event) = self.handler.buffered_events.pop_front() {
            sink(event);
        }

        match status {
            WinitPumpStatus::Continue => Ok(PumpStatus::Continue),
            WinitPumpStatus::Exit(_) => Ok(PumpStatus::Exit),
        }
    }
}

/// The `winit::application::ApplicationHandler` this adapter drives —
/// buffers mapped events for `WinitSystem::pump_events` to drain, and creates
/// the window `WinitSystem::create_window` requested as soon as the platform
/// resumes the loop.
#[derive(Default)]
struct Handler {
    pending_attributes: Option<winit::window::WindowAttributes>,
    window: Option<Arc<Window>>,
    creation_error: Option<WindowError>,
    buffered_events: VecDeque<WindowEvent>,
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let Some(attributes) = self.pending_attributes.take() else {
            return;
        };
        match event_loop.create_window(attributes) {
            Ok(window) => self.window = Some(Arc::new(window)),
            Err(error) => {
                self.creation_error = Some(WindowError::creation_failed(error.to_string()));
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(mapped) = map_window_event(event) else {
            return;
        };
        if matches!(mapped, WindowEvent::CloseRequested) {
            event_loop.exit();
        }
        self.buffered_events.push_back(mapped);
    }
}

fn to_winit_attributes(attrs: &WindowAttributes) -> winit::window::WindowAttributes {
    let size = attrs.initial_size();
    let physical_size = winit::dpi::PhysicalSize::new(size.width(), size.height());
    winit::window::Window::default_attributes()
        .with_title(attrs.title().as_str())
        .with_inner_size(physical_size)
}
