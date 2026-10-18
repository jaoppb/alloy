//! Border color types (`border-color`, `border-top-color`, etc.).

use crate::domain::color::CssColor;

/// The color of a box border (CSS Backgrounds & Borders L3 §4.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BorderColor {
    /// Resolved against the element's `color` property at paint time (CSS `initial` value).
    #[default]
    CurrentColor,
    /// An explicit colour.
    Color(CssColor),
}

impl BorderColor {
    /// Resolved colour for this border side given the element's text colour.
    #[must_use]
    pub const fn resolve(self, current_color: CssColor) -> CssColor {
        match self {
            Self::CurrentColor => current_color,
            Self::Color(color) => color,
        }
    }
}

impl From<CssColor> for BorderColor {
    fn from(color: CssColor) -> Self {
        Self::Color(color)
    }
}

/// A quartet of border colours for the four sides of a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct BorderColorEdges {
    top: BorderColor,
    right: BorderColor,
    bottom: BorderColor,
    left: BorderColor,
}

impl BorderColorEdges {
    /// All four sides `currentColor` (CSS `initial` value).
    pub const CURRENT_COLOR: Self = Self::uniform(BorderColor::CurrentColor);

    /// All four sides opaque black.
    pub const BLACK: Self = Self::uniform(BorderColor::Color(CssColor::BLACK));

    /// Distinct colours per side.
    #[must_use]
    pub const fn new(
        top: BorderColor,
        right: BorderColor,
        bottom: BorderColor,
        left: BorderColor,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The same colour on every side.
    #[must_use]
    pub const fn uniform(color: BorderColor) -> Self {
        Self::new(color, color, color, color)
    }

    #[must_use]
    pub const fn top(self) -> BorderColor {
        self.top
    }

    #[must_use]
    pub const fn right(self) -> BorderColor {
        self.right
    }

    #[must_use]
    pub const fn bottom(self) -> BorderColor {
        self.bottom
    }

    #[must_use]
    pub const fn left(self) -> BorderColor {
        self.left
    }

    #[must_use]
    pub const fn with_top(self, top: BorderColor) -> Self {
        Self { top, ..self }
    }

    #[must_use]
    pub const fn with_right(self, right: BorderColor) -> Self {
        Self { right, ..self }
    }

    #[must_use]
    pub const fn with_bottom(self, bottom: BorderColor) -> Self {
        Self { bottom, ..self }
    }

    #[must_use]
    pub const fn with_left(self, left: BorderColor) -> Self {
        Self { left, ..self }
    }
}

impl Default for BorderColorEdges {
    fn default() -> Self {
        Self::CURRENT_COLOR
    }
}
