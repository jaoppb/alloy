//! [`OverflowWrap`] and [`WordBreak`] — line breaking rules (CSS Text L3 §5).

use core::fmt;

/// Whether unbreakable words may be broken to prevent overflow (CSS Text L3 §5.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OverflowWrap {
    /// Lines may break only at normal word break points — CSS `initial`.
    #[default]
    Normal,
    /// Unbreakable words may break at arbitrary points if overflow occurs.
    BreakWord,
    /// An otherwise unbreakable string of characters may be broken at any point.
    Anywhere,
}

impl OverflowWrap {
    #[must_use]
    pub const fn allows_emergency_break(self) -> bool {
        matches!(self, Self::BreakWord | Self::Anywhere)
    }

    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::BreakWord => "break-word",
            Self::Anywhere => "anywhere",
        }
    }
}

impl fmt::Display for OverflowWrap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// How words break inside text (CSS Text L3 §5.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WordBreak {
    /// Default line breaking rules — CSS `initial`.
    #[default]
    Normal,
    /// Word breaks may be inserted between any two characters for non-CJK text.
    BreakAll,
    /// Word breaks are forbidden for CJK text.
    KeepAll,
}

impl WordBreak {
    #[must_use]
    pub const fn allows_break_all(self) -> bool {
        matches!(self, Self::BreakAll)
    }

    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::BreakAll => "break-all",
            Self::KeepAll => "keep-all",
        }
    }
}

impl fmt::Display for WordBreak {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
