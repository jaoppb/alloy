//! Border radius types (`border-radius`, `border-top-left-radius`, etc.).

use crate::domain::length::Length;

/// Corner radii for the four corners of a box (CSS Backgrounds & Borders L3 §5.1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct BorderRadius {
    top_left: Length,
    top_right: Length,
    bottom_right: Length,
    bottom_left: Length,
}

impl BorderRadius {
    /// All four corners zero (CSS `initial` value).
    pub const ZERO: Self = Self::uniform(Length::ZERO);

    /// Every corner at its CSS `initial` value.
    #[must_use]
    pub const fn initial() -> Self {
        Self::ZERO
    }

    /// Distinct radii per corner.
    #[must_use]
    pub const fn new(
        top_left: Length,
        top_right: Length,
        bottom_right: Length,
        bottom_left: Length,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// The same radius on every corner.
    #[must_use]
    pub const fn uniform(radius: Length) -> Self {
        Self::new(radius, radius, radius, radius)
    }

    #[must_use]
    pub const fn top_left(self) -> Length {
        self.top_left
    }

    #[must_use]
    pub const fn top_right(self) -> Length {
        self.top_right
    }

    #[must_use]
    pub const fn bottom_right(self) -> Length {
        self.bottom_right
    }

    #[must_use]
    pub const fn bottom_left(self) -> Length {
        self.bottom_left
    }

    #[must_use]
    pub const fn with_top_left(self, top_left: Length) -> Self {
        Self { top_left, ..self }
    }

    #[must_use]
    pub const fn with_top_right(self, top_right: Length) -> Self {
        Self { top_right, ..self }
    }

    #[must_use]
    pub const fn with_bottom_right(self, bottom_right: Length) -> Self {
        Self {
            bottom_right,
            ..self
        }
    }

    #[must_use]
    pub const fn with_bottom_left(self, bottom_left: Length) -> Self {
        Self {
            bottom_left,
            ..self
        }
    }

    /// Whether all four corners have zero radius.
    #[must_use]
    pub fn is_zero(self) -> bool {
        self == Self::ZERO
    }
}
