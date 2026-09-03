//! Points, sizes and rectangles, all in [`Au`].
//!
//! [`Size`] and [`SurfaceSize`] carry their non-negativity as an invariant
//! rather than as a convention: a negative extent has no correct reading, so it
//! is refused at construction and can never reach a backend (`PRD-005:80`).

use core::fmt;

use crate::domain::unit::Au;

/// A position in the coordinate space of the surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    horizontal: Au,
    vertical: Au,
}

impl Point {
    /// The surface origin, top-left.
    pub const ORIGIN: Self = Self {
        horizontal: Au::ZERO,
        vertical: Au::ZERO,
    };

    #[must_use]
    pub const fn new(horizontal: Au, vertical: Au) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    #[must_use]
    pub const fn horizontal(self) -> Au {
        self.horizontal
    }

    #[must_use]
    pub const fn vertical(self) -> Au {
        self.vertical
    }

    /// Moves the point, saturating rather than wrapping at the `Au` extremes.
    #[must_use]
    pub const fn translated(self, horizontal: Au, vertical: Au) -> Self {
        Self {
            horizontal: self.horizontal.saturating_add(horizontal),
            vertical: self.vertical.saturating_add(vertical),
        }
    }
}

impl fmt::Display for Point {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "({}, {})", self.horizontal, self.vertical)
    }
}

/// A non-negative extent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Size {
    width: Au,
    height: Au,
}

impl Size {
    /// An empty extent.
    pub const EMPTY: Self = Self {
        width: Au::ZERO,
        height: Au::ZERO,
    };

    /// Builds an extent, or `None` when either dimension is negative.
    ///
    /// Refusal, not clamping: a negative width means the caller computed
    /// something wrong, and silently turning it into zero would hide the defect
    /// (v0.3 report §2.3).
    #[must_use]
    pub const fn new(width: Au, height: Au) -> Option<Self> {
        if width.is_negative() || height.is_negative() {
            return None;
        }
        Some(Self { width, height })
    }

    #[must_use]
    pub const fn width(self) -> Au {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> Au {
        self.height
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.width.is_zero() || self.height.is_zero()
    }
}

impl fmt::Display for Size {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} × {}", self.width, self.height)
    }
}

/// An axis-aligned rectangle: an origin plus a non-negative extent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    origin: Point,
    size: Size,
}

impl Rect {
    #[must_use]
    pub const fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    #[must_use]
    pub const fn origin(self) -> Point {
        self.origin
    }

    #[must_use]
    pub const fn size(self) -> Size {
        self.size
    }

    #[must_use]
    pub const fn min_x(self) -> Au {
        self.origin.horizontal()
    }

    #[must_use]
    pub const fn min_y(self) -> Au {
        self.origin.vertical()
    }

    /// The exclusive right edge, saturating at the `Au` extreme.
    #[must_use]
    pub const fn max_x(self) -> Au {
        self.origin.horizontal().saturating_add(self.size.width())
    }

    /// The exclusive bottom edge, saturating at the `Au` extreme.
    #[must_use]
    pub const fn max_y(self) -> Au {
        self.origin.vertical().saturating_add(self.size.height())
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.size.is_empty()
    }

    /// The overlap with `other`, or `None` when they do not overlap.
    ///
    /// This is how a clip stack narrows: each `PushClip` intersects with the
    /// region already in force, so a backend never has to reason about more
    /// than one rectangle.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Option<Self> {
        let left = self.min_x().larger(other.min_x());
        let top = self.min_y().larger(other.min_y());
        let right = self.max_x().smaller(other.max_x());
        let bottom = self.max_y().smaller(other.max_y());
        Self::between(left, top, right, bottom)
    }

    /// Builds the rectangle spanned by two corners, or `None` when the second
    /// corner does not sit past the first on both axes.
    const fn between(left: Au, top: Au, right: Au, bottom: Au) -> Option<Self> {
        let width = right.saturating_sub(left);
        let height = bottom.saturating_sub(top);
        match Size::new(width, height) {
            Some(size) if !size.is_empty() => Some(Self::new(Point::new(left, top), size)),
            _ => None,
        }
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at {}", self.size, self.origin)
    }
}

/// The pixel dimensions of a render surface.
///
/// Whole pixels, not [`Au`]: a surface is a buffer, and a buffer has an integral
/// number of rows and columns. Zero in either dimension is refused, because
/// every backend operation on it would be a silent no-op.
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

    /// How many pixels the surface holds, or `None` when the product overflows
    /// the addressable range of this platform.
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
