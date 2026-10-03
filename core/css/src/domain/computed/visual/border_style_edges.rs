//! Border style quartet types.

use super::border_style::BorderStyle;

/// A quartet of border styles for the four sides of a box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct BorderStyleEdges {
    top: BorderStyle,
    right: BorderStyle,
    bottom: BorderStyle,
    left: BorderStyle,
}

impl BorderStyleEdges {
    /// All four sides `none`.
    pub const NONE: Self = Self::uniform(BorderStyle::None);

    /// Distinct styles per side.
    #[must_use]
    pub const fn new(
        top: BorderStyle,
        right: BorderStyle,
        bottom: BorderStyle,
        left: BorderStyle,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The same style on every side.
    #[must_use]
    pub const fn uniform(style: BorderStyle) -> Self {
        Self::new(style, style, style, style)
    }

    #[must_use]
    pub const fn top(self) -> BorderStyle {
        self.top
    }

    #[must_use]
    pub const fn right(self) -> BorderStyle {
        self.right
    }

    #[must_use]
    pub const fn bottom(self) -> BorderStyle {
        self.bottom
    }

    #[must_use]
    pub const fn left(self) -> BorderStyle {
        self.left
    }

    #[must_use]
    pub const fn with_top(self, top: BorderStyle) -> Self {
        Self { top, ..self }
    }

    #[must_use]
    pub const fn with_right(self, right: BorderStyle) -> Self {
        Self { right, ..self }
    }

    #[must_use]
    pub const fn with_bottom(self, bottom: BorderStyle) -> Self {
        Self { bottom, ..self }
    }

    #[must_use]
    pub const fn with_left(self, left: BorderStyle) -> Self {
        Self { left, ..self }
    }
}
