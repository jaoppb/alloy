//! [`Direction`] — inline base direction / text direction (CSS Writing Modes L3 §2.1).

use core::fmt;

/// Inline base direction for text, layout, and logical-to-physical mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Direction {
    /// Left-to-right inline flow — the CSS `initial` value.
    #[default]
    Ltr,
    /// Right-to-left inline flow.
    Rtl,
}

impl Direction {
    /// Whether direction is left-to-right.
    #[must_use]
    pub const fn is_ltr(self) -> bool {
        matches!(self, Self::Ltr)
    }

    /// Whether direction is right-to-left.
    #[must_use]
    pub const fn is_rtl(self) -> bool {
        matches!(self, Self::Rtl)
    }

    /// Keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
