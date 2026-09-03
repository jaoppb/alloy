//! [`RenderBackend`] — the replaceable rasterization port (`ADR-0011`,
//! `PRD-005:87`, **C-14**).
//!
//! ## Object-safety, and why there is no `dyn` companion
//!
//! Every method here speaks only this crate's own types, so
//! `Box<dyn RenderBackend>` compiles directly and `ADR-0011:87-89` item 2 is
//! satisfied without the companion-trait machinery that `RuntimeEngine` needed
//! (`ADR-0013`). The difference is not an accident: `RuntimeEngine` has generic
//! sugar methods (`eval::<T>`) that break object-safety, whereas a backend never
//! needs to be generic over a caller's type. `Box<dyn RenderBackend>` is exactly
//! what the tier cascade of `PRD-005:33-58` returns.
//!
//! ## Why `read_back` is on the trait
//!
//! It would be easy to put frame read-back only on the software backend, since
//! that is the only tier that has the pixels in host memory today. Putting it on
//! the port is what makes roadmap point `I6` verifiable: in `F12`, Vulkan reads
//! back through a staging buffer and is compared against the *same* golden image
//! the software rasterizer produced. Without it there would be no way to tell a
//! backend defect from a layout defect.
//!
//! ## Frame lifecycle (`ADR-0011` item 5)
//!
//! `begin_frame` → `submit`* → `end_frame` → `read_back`. Calling out of that
//! order is [`GraphicsError::FrameOutOfOrder`], never a panic and never
//! undefined output. A backend is reusable: a new `begin_frame` after
//! `read_back` starts a fresh frame.

use crate::domain::display_list::DisplayList;
use crate::domain::error::GraphicsError;
use crate::domain::framebuffer::Framebuffer;
use crate::domain::geometry::SurfaceSize;
use crate::domain::tier::BackendTier;

/// A rasterizer that turns display lists into pixels.
pub trait RenderBackend: Send + Sync {
    /// Which technology this backend is built on. Constant for its lifetime.
    fn tier(&self) -> BackendTier;

    /// Starts a frame on a surface of `size`, discarding any previous content.
    fn begin_frame(&mut self, size: SurfaceSize) -> Result<(), GraphicsError>;

    /// Rasterizes `list` into the frame in progress.
    ///
    /// A backend that does not implement a command in the list reports
    /// [`GraphicsError::Unsupported`] naming it — the v0.3 software rasterizer
    /// does exactly this for `DrawImage` and `DrawPath`.
    fn submit(&mut self, list: &DisplayList) -> Result<(), GraphicsError>;

    /// Finishes the frame and makes it available to [`Self::read_back`].
    fn end_frame(&mut self) -> Result<(), GraphicsError>;

    /// Copies the finished frame into host memory.
    fn read_back(&self) -> Result<Framebuffer, GraphicsError>;
}
