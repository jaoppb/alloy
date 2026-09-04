//! Geometry and scale value objects for this port: [`SurfaceSize`],
//! [`ScaleFactor`], [`PhysicalPosition`].
//!
//! Deliberately **not** `graphics::SurfaceSize`: naming a `graphics` type here
//! would put `core/graphics` in this crate's dependency graph, which the
//! crate doc's `## Layout` section forbids.
//! [`FrameView`](crate::domain::frame::FrameView) is the only seam between the
//! two ports, and it is built one level up, by a caller that already has both.

use core::fmt;

/// The pixel dimensions of a window's drawable surface.
///
/// Zero in either dimension is refused at construction, not clamped: a
/// surface no backend could draw into has no correct reading (mirrors
/// `graphics::SurfaceSize`, `PRD-005:80`'s reasoning applied to this port).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SurfaceSize {
    width: u32,
    height: u32,
}

impl SurfaceSize {
    /// Builds a surface size, or `None` when either dimension is zero.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        Some(Self { width, height })
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    /// How many pixels this surface holds, or `None` when the product
    /// overflows `usize` on this platform.
    #[must_use]
    pub fn pixel_count(self) -> Option<usize> {
        let width = usize::try_from(self.width).ok()?;
        let height = usize::try_from(self.height).ok()?;
        width.checked_mul(height)
    }
}

impl fmt::Display for SurfaceSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}×{} px", self.width, self.height)
    }
}

/// The ratio between physical pixels and logical (author-facing) pixels.
///
/// Always positive and finite: a scale factor with no correct reading is
/// refused at construction rather than propagated, the same rule
/// `graphics::Au::from_px` applies to `NaN` (v0.3 report §2.3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleFactor(f64);

impl ScaleFactor {
    /// Builds a scale factor, or `None` when `value` is not finite and
    /// positive.
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        if !value.is_finite() || value <= 0.0 {
            return None;
        }
        Some(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl fmt::Display for ScaleFactor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}x", self.0)
    }
}

/// A position in the physical pixel space of a surface — a pointer location,
/// most often.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalPosition {
    x: f64,
    y: f64,
}

impl PhysicalPosition {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn x(self) -> f64 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> f64 {
        self.y
    }
}

impl fmt::Display for PhysicalPosition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "({}, {})", self.x, self.y)
    }
}
