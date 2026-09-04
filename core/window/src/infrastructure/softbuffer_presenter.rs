//! [`SoftbufferPresenter`] — blits a [`FrameView`] onto a `softbuffer` surface.
//!
//! `softbuffer`'s own `unsafe` is the row-3 nominal exception of `ADR-0018`
//! (`unsafe-allowlist.toml`); this file stays `#![forbid(unsafe_code)]` like
//! the rest of the crate.
//!
//! ## "Present before `create_window`" is a type-level impossibility here
//!
//! [`Self::new`] takes the live `Arc<Window>` a
//! [`WinitSystem::create_window`](crate::infrastructure::winit_system::WinitSystem::create_window)
//! call produced (via
//! [`WinitSystem::window_handle`](crate::infrastructure::winit_system::WinitSystem::window_handle)).
//! There is no path to a `SoftbufferPresenter` that does not already have a
//! real window behind it — the real adapter's answer to the port contract's
//! ordering rule, in the type system rather than a runtime check (see
//! `application::conformance`'s module doc for the full reasoning).

use std::num::NonZeroU32;
use std::sync::Arc;

use winit::window::Window;

use crate::application::ports::Presenter;
use crate::domain::attributes::WindowId;
use crate::domain::error::{WindowError, WindowOperation};
use crate::domain::frame::FrameView;

/// Presents frames onto a `winit` window's surface via `softbuffer`'s CPU
/// blit path — no GPU driver required, mirroring the CPU tier `graphics`
/// already ships for `F4a`.
pub struct SoftbufferPresenter {
    window: WindowId,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
}

impl SoftbufferPresenter {
    /// Binds a presenter to `window`.
    ///
    /// # Errors
    ///
    /// [`WindowError::CreationFailed`] when `softbuffer` cannot open a
    /// connection to the window's display, or bind a surface to it.
    pub fn new(window_id: WindowId, window: Arc<Window>) -> Result<Self, WindowError> {
        let context = softbuffer::Context::new(Arc::clone(&window))
            .map_err(|error| WindowError::creation_failed(error.to_string()))?;
        let surface = softbuffer::Surface::new(&context, window)
            .map_err(|error| WindowError::creation_failed(error.to_string()))?;
        Ok(Self {
            window: window_id,
            surface,
        })
    }
}

impl Presenter for SoftbufferPresenter {
    fn present(&mut self, frame: FrameView<'_>) -> Result<(), WindowError> {
        let window = self.window;
        let width = NonZeroU32::new(frame.width()).ok_or_else(|| {
            WindowError::operation_failed(
                window,
                WindowOperation::Present,
                "the frame has zero width",
            )
        })?;
        let height = NonZeroU32::new(frame.height()).ok_or_else(|| {
            WindowError::operation_failed(
                window,
                WindowOperation::Present,
                "the frame has zero height",
            )
        })?;

        self.surface.resize(width, height).map_err(|error| {
            WindowError::operation_failed(window, WindowOperation::Present, error.to_string())
        })?;

        let mut buffer = self.surface.buffer_mut().map_err(|error| {
            WindowError::operation_failed(window, WindowOperation::Present, error.to_string())
        })?;

        // `softbuffer`'s pixel format is `0x00RRGGBB` (no alpha channel); this
        // port's `FrameView` is `0xAARRGGBB`, pre-multiplied. Masking the
        // alpha byte off is exact for a pre-multiplied source presented onto
        // an opaque window surface — there is nothing behind it to blend with.
        for (destination, source) in buffer.iter_mut().zip(frame.pixels()) {
            *destination = source & 0x00FF_FFFF;
        }

        buffer.present().map_err(|error| {
            WindowError::operation_failed(window, WindowOperation::Present, error.to_string())
        })
    }
}
