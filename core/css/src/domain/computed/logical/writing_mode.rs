//! [`WritingMode`] — whether lines of text are laid out horizontally or vertically
//! and the direction in which blocks progress (CSS Writing Modes L3 §3.1).

use core::fmt;

/// The writing mode for an element (CSS Writing Modes L3 §3.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WritingMode {
    /// Horizontal lines, top-to-bottom block flow — the CSS `initial` value.
    #[default]
    HorizontalTb,
    /// Vertical lines, right-to-left block flow (e.g. traditional East Asian).
    VerticalRl,
    /// Vertical lines, left-to-right block flow (e.g. Mongolian).
    VerticalLr,
}

impl WritingMode {
    /// Whether block flow is horizontal (lines are horizontal).
    #[must_use]
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::HorizontalTb)
    }

    /// Whether block flow is vertical (lines are vertical).
    #[must_use]
    pub const fn is_vertical(self) -> bool {
        !self.is_horizontal()
    }

    /// Keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::HorizontalTb => "horizontal-tb",
            Self::VerticalRl => "vertical-rl",
            Self::VerticalLr => "vertical-lr",
        }
    }
}

impl fmt::Display for WritingMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
