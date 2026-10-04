//! Background size types (`background-size`).

use crate::domain::computed::sizing::Sizing;

/// The sizing rule for a background image (CSS Backgrounds & Borders L3 §3.9).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum BackgroundSize {
    /// Preserves intrinsic proportions — the CSS `initial` value (`auto auto`).
    #[default]
    Auto,
    /// Scales the image to cover the background positioning area.
    Cover,
    /// Scales the image to fit entirely within the positioning area.
    Contain,
    /// Explicit width and height dimensions.
    Explicit {
        /// The horizontal sizing component.
        width: Sizing,
        /// The vertical sizing component.
        height: Sizing,
    },
}

impl BackgroundSize {
    /// Constructs an explicit dimension size.
    #[must_use]
    pub const fn explicit(width: Sizing, height: Sizing) -> Self {
        Self::Explicit { width, height }
    }

    /// Whether this is the `auto` size.
    #[must_use]
    pub const fn is_auto(self) -> bool {
        matches!(self, Self::Auto)
    }

    /// Whether this is the `cover` keyword.
    #[must_use]
    pub const fn is_cover(self) -> bool {
        matches!(self, Self::Cover)
    }

    /// Whether this is the `contain` keyword.
    #[must_use]
    pub const fn is_contain(self) -> bool {
        matches!(self, Self::Contain)
    }
}
