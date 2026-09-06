//! Box shadow types (`box-shadow`).

use crate::domain::color::CssColor;
use crate::domain::length::Length;

/// Whether a box-shadow is rendered inside or outside the border box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShadowPlacement {
    /// Outset shadow (the CSS default).
    #[default]
    Outset,
    /// Inset shadow (`inset` keyword).
    Inset,
}

impl ShadowPlacement {
    /// Whether this placement is inset.
    #[must_use]
    pub const fn is_inset(self) -> bool {
        matches!(self, Self::Inset)
    }
}

/// A single box shadow (CSS Backgrounds & Borders L3 §7.1).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BoxShadow {
    horizontal: Length,
    vertical: Length,
    blur: Length,
    spread: Length,
    color: CssColor,
    placement: ShadowPlacement,
}

impl BoxShadow {
    /// Constructs a box shadow.
    #[must_use]
    pub const fn new(
        horizontal: Length,
        vertical: Length,
        blur: Length,
        spread: Length,
        color: CssColor,
        placement: ShadowPlacement,
    ) -> Self {
        Self {
            horizontal,
            vertical,
            blur,
            spread,
            color,
            placement,
        }
    }

    #[must_use]
    pub const fn horizontal(self) -> Length {
        self.horizontal
    }

    #[must_use]
    pub const fn vertical(self) -> Length {
        self.vertical
    }

    #[must_use]
    pub const fn blur(self) -> Length {
        self.blur
    }

    #[must_use]
    pub const fn spread(self) -> Length {
        self.spread
    }

    #[must_use]
    pub const fn color(self) -> CssColor {
        self.color
    }

    #[must_use]
    pub const fn placement(self) -> ShadowPlacement {
        self.placement
    }
}
