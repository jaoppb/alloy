//! The OpenGL rung of the cascade (`PRD-005:54`, `ADR-0009`).
//!
//! **Not implemented until F12** — and deliberately present anyway. A rung that
//! reports itself unavailable is what makes the fall to the next tier the real
//! algorithm executing rather than a tautology: the cascade of
//! [`super::cascade::select_backend`] walks three rungs today and will walk the
//! same three when `glow` and `glutin` land, with nothing above this line changing.
//!
//! It is `Err(BackendUnavailable)` rather than `todo!()` — which the lint gate
//! denies anyway — because "no usable OpenGL on this machine" is an ordinary,
//! expected, recoverable condition, not an unfinished thought. **C-16** is F12.

use crate::application::ports::RenderBackend;
use crate::domain::error::GraphicsError;
use crate::domain::tier::BackendTier;

/// Attempts to bring up an OpenGL context.
pub(super) fn initialise() -> Result<Box<dyn RenderBackend>, GraphicsError> {
    Err(GraphicsError::BackendUnavailable {
        tier: BackendTier::OpenGl,
    })
}
