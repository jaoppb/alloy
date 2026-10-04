//! [`LetterSpacing`] and [`WordSpacing`] (CSS Text L3 §8).

use core::fmt;
use graphics::Au;

use crate::domain::length::Length;

/// `length` as the absolute length it computes to once the element's own
/// computed `font_size` is known: `em` and `%` (both font-relative for the
/// text-spacing properties) become pixels, everything else is already
/// absolute. `rem` stays as written — it is relative to the root, not to this
/// element, so inheriting it unresolved loses nothing. A magnitude with no
/// correct reading (`NaN`, `±inf`) is left for layout to reject.
pub(super) fn absolute_length(length: Length, font_size: Au) -> Length {
    match length {
        Length::Em(_) | Length::Percent(_) => length
            .resolve_to_au(font_size, font_size)
            .map_or(length, |resolved| Length::Pixels(resolved.to_px().get())),
        Length::Pixels(_) | Length::Rem(_) | Length::Points(_) => length,
    }
}

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

    /// The computed value: a font-relative length made absolute against the
    /// element's own computed `font_size` (CSS Text L3 §8.1, "computed value:
    /// an absolute length"), so a descendant inherits the length rather than
    /// re-resolving `em` against its own font size.
    #[must_use]
    pub fn absolutized(self, font_size: Au) -> Self {
        match self {
            Self::Normal => self,
            Self::Length(length) => Self::Length(absolute_length(length, font_size)),
        }
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

    /// The computed value: a font-relative length made absolute against the
    /// element's own computed `font_size` (CSS Text L3 §8.2), for the same
    /// reason as [`LetterSpacing::absolutized`].
    #[must_use]
    pub fn absolutized(self, font_size: Au) -> Self {
        match self {
            Self::Normal => self,
            Self::Length(length) => Self::Length(absolute_length(length, font_size)),
        }
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
