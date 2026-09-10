//! Border color types (`border-color`, `border-top-color`, etc.).

use crate::domain::color::CssColor;

/// A quartet of border colours for the four sides of a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct BorderColorEdges {
    top: CssColor,
    right: CssColor,
    bottom: CssColor,
    left: CssColor,
}

impl BorderColorEdges {
    /// All four sides opaque black (CSS `initial` value).
    pub const BLACK: Self = Self::uniform(CssColor::BLACK);

    /// Distinct colours per side.
    #[must_use]
    pub const fn new(top: CssColor, right: CssColor, bottom: CssColor, left: CssColor) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The same colour on every side.
    #[must_use]
    pub const fn uniform(color: CssColor) -> Self {
        Self::new(color, color, color, color)
    }

    #[must_use]
    pub const fn top(self) -> CssColor {
        self.top
    }

    #[must_use]
    pub const fn right(self) -> CssColor {
        self.right
    }

    #[must_use]
    pub const fn bottom(self) -> CssColor {
        self.bottom
    }

    #[must_use]
    pub const fn left(self) -> CssColor {
        self.left
    }

    #[must_use]
    pub const fn with_top(self, top: CssColor) -> Self {
        Self { top, ..self }
    }

    #[must_use]
    pub const fn with_right(self, right: CssColor) -> Self {
        Self { right, ..self }
    }

    #[must_use]
    pub const fn with_bottom(self, bottom: CssColor) -> Self {
        Self { bottom, ..self }
    }

    #[must_use]
    pub const fn with_left(self, left: CssColor) -> Self {
        Self { left, ..self }
    }
}

impl Default for BorderColorEdges {
    fn default() -> Self {
        Self::BLACK
    }
}
