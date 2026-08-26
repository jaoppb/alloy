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

/// 2D Axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Rect {
    /// Creates a new 2D Rect.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns the leftmost x coordinate.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    /// Returns the topmost y coordinate.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }

    /// Returns the width.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.width
    }

    /// Returns the height.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.height
    }

    /// Returns the rightmost x coordinate (`x + width`).
    #[must_use]
    pub const fn right(self) -> f32 {
        self.x + self.width
    }

    /// Returns the bottommost y coordinate (`y + height`).
    #[must_use]
    pub const fn bottom(self) -> f32 {
        self.y + self.height
    }
}
