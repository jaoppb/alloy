//! Background position types (`background-position`).

use crate::domain::length::Length;

/// The position of a background image relative to its box (CSS Backgrounds & Borders L3 §3.6).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BackgroundPosition {
    x: Length,
    y: Length,
}

impl BackgroundPosition {
    /// The CSS `initial` value: `0% 0%` (top left).
    pub const INITIAL: Self = Self::new(Length::Percent(0.0), Length::Percent(0.0));

    /// Constructs a position from horizontal and vertical offsets.
    #[must_use]
    pub const fn new(x: Length, y: Length) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn x(self) -> Length {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> Length {
        self.y
    }

    #[must_use]
    pub const fn with_x(self, x: Length) -> Self {
        Self { x, ..self }
    }

    #[must_use]
    pub const fn with_y(self, y: Length) -> Self {
        Self { y, ..self }
    }
}

impl Default for BackgroundPosition {
    fn default() -> Self {
        Self::INITIAL
    }
}
