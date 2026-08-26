use crate::domain::error::GraphicsError;

/// 2D Point coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    x: f32,
    y: f32,
}

impl Point {
    /// Creates a new 2D Point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns the X coordinate.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    /// Returns the Y coordinate.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }
}

/// 2D Size dimension.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    width: f32,
    height: f32,
}

impl Size {
    /// Creates a new 2D Size, validating that dimensions are non-negative and finite.
    ///
    /// # Errors
    /// Returns `GraphicsError::InvalidCommand` if width or height is negative or non-finite.
    pub fn new(width: f32, height: f32) -> Result<Self, GraphicsError> {
        if !width.is_finite() || width < 0.0 || !height.is_finite() || height < 0.0 {
            return Err(GraphicsError::InvalidCommand(format!(
                "Size dimensions must be finite and >= 0, got ({width}, {height})"
            )));
        }
        Ok(Self { width, height })
    }

    /// Returns the width dimension.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.width
    }

    /// Returns the height dimension.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.height
    }
}

/// 2D raster position in discrete pixel coordinates (C-31).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Position {
    x: u32,
    y: u32,
}

impl Position {
    /// Creates a new `Position`.
    #[must_use]
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    /// Returns the X pixel coordinate.
    #[must_use]
    pub const fn x(self) -> u32 {
        self.x
    }

    /// Returns the Y pixel coordinate.
    #[must_use]
    pub const fn y(self) -> u32 {
        self.y
    }
}

/// 2D Axis-aligned rectangle composed of origin Point and dimension Size (C-30, C-34).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    origin: Point,
    size: Size,
}

impl Rect {
    /// Creates a new 2D Rect from origin `Point` and dimension `Size`.
    #[must_use]
    pub const fn from_origin_size(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    /// Creates a new 2D Rect from coordinates and dimensions.
    #[must_use]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        let size = Size::new(width.max(0.0), height.max(0.0)).unwrap_or(Size {
            width: 0.0,
            height: 0.0,
        });
        Self {
            origin: Point::new(x, y),
            size,
        }
    }

    /// Returns the origin point of the rectangle.
    #[must_use]
    pub const fn origin(&self) -> Point {
        self.origin
    }

    /// Returns the size dimension of the rectangle.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// Returns the leftmost x coordinate.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.origin.x()
    }

    /// Returns the topmost y coordinate.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.origin.y()
    }

    /// Returns the width.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.size.width()
    }

    /// Returns the height.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.size.height()
    }

    /// Returns the rightmost x coordinate (`x + width`).
    #[must_use]
    pub const fn right(self) -> f32 {
        self.origin.x() + self.size.width()
    }

    /// Returns the bottommost y coordinate (`y + height`).
    #[must_use]
    pub const fn bottom(self) -> f32 {
        self.origin.y() + self.size.height()
    }
}
