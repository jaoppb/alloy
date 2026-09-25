//! [`FrameView`] — a borrowed, read-only view onto a rendered frame's pixels.
//!
//! **Not** `graphics::Framebuffer`: naming that type here would put
//! `core/graphics` in this crate's dependency graph, which the crate doc's
//! `## Layout` section forbids. A caller that already depends on both crates
//! (later, `alloy`) builds a `FrameView` from whatever it actually renders
//! with, in one borrow, right before calling
//! [`Presenter::present`](crate::application::ports::Presenter::present). This
//! is the seam that keeps `core/window` dependency-free of `core/graphics`.

use core::fmt;

/// A rectangle of RGBA8, pre-multiplied-alpha pixels — one `u32` per pixel,
/// `0xAARRGGBB` — borrowed for the duration of one
/// [`Presenter::present`](crate::application::ports::Presenter::present) call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameView<'pixels> {
    width: u32,
    height: u32,
    pixels: &'pixels [u32],
}

impl<'pixels> FrameView<'pixels> {
    /// Builds a view, or `None` when `pixels` does not have exactly
    /// `width * height` elements — a presenter can trust the slice matches
    /// the declared dimensions without re-checking it on every call.
    #[must_use]
    pub fn new(width: u32, height: u32, pixels: &'pixels [u32]) -> Option<Self> {
        let expected = u64::from(width).checked_mul(u64::from(height))?;
        let actual = u64::try_from(pixels.len()).ok()?;
        if expected != actual {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    #[must_use]
    pub const fn pixels(self) -> &'pixels [u32] {
        self.pixels
    }
}

impl fmt::Display for FrameView<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}×{} frame", self.width, self.height)
    }
}
