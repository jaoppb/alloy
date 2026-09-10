//! [`LetterSpacing`] and [`WordSpacing`] (CSS Text L3 §8).

use core::fmt;
use graphics::Au;

use crate::domain::length::Length;

/// Spacing between character glyphs (CSS Text L3 §8.1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum LetterSpacing {
    /// Normal inter-character spacing — CSS `initial`.
    #[default]
    Normal,
    /// Additional spacing length.
    Length(Length),
}

impl LetterSpacing {
    #[must_use]
    pub const fn is_normal(self) -> bool {
        matches!(self, Self::Normal)
    }

    /// Resolves extra spacing to [`Au`].
    #[must_use]
    pub fn resolve_to_au(self, font_size: Au) -> Option<Au> {
        match self {
            Self::Normal => Some(Au::ZERO),
            Self::Length(length) => length.resolve_to_au(font_size, font_size),
        }
    }
}

impl fmt::Display for LetterSpacing {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => formatter.write_str("normal"),
            Self::Length(length) => length.fmt(formatter),
        }
    }
}

/// Spacing between words (CSS Text L3 §8.2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum WordSpacing {
    /// Normal inter-word spacing — CSS `initial`.
    #[default]
    Normal,
    /// Additional spacing length.
    Length(Length),
}

impl WordSpacing {
    #[must_use]
    pub const fn is_normal(self) -> bool {
        matches!(self, Self::Normal)
    }

    /// Resolves extra word spacing to [`Au`].
    #[must_use]
    pub fn resolve_to_au(self, font_size: Au) -> Option<Au> {
        match self {
            Self::Normal => Some(Au::ZERO),
            Self::Length(length) => length.resolve_to_au(font_size, font_size),
        }
    }
}

impl fmt::Display for WordSpacing {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => formatter.write_str("normal"),
            Self::Length(length) => length.fmt(formatter),
        }
    }
}
