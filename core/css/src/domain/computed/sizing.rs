//! [`Sizing`] and [`BoxSizing`] — the computed values of `width` / `height` /
//! `flex-basis` and of `box-sizing` (v0.5 B4).
//!
//! [`Length`] cannot express `auto`, and `auto` is not a length: it is the
//! instruction "let the formatting context decide". [`Sizing`] is that sum, so
//! the layout engine never has to read a sentinel magnitude out of an `f32`.

use core::fmt;

use graphics::Au;

use crate::domain::length::Length;

/// A box's size along one axis, as the cascade computed it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum Sizing {
    /// The formatting context decides — the `initial` value of `width`,
    /// `height` and `flex-basis`.
    #[default]
    Auto,
    /// An author-declared length.
    Fixed(Length),
}

impl Sizing {
    /// Whether the formatting context decides this axis.
    #[must_use]
    pub const fn is_auto(self) -> bool {
        matches!(self, Self::Auto)
    }

    /// The declared length resolved to a computed [`Au`], or `None` for
    /// `auto` and for a non-finite magnitude (the rule of
    /// [`Length::resolve_to_au`]).
    #[must_use]
    pub fn resolve(self, font_size: Au, container: Au) -> Option<Au> {
        match self {
            Self::Auto => None,
            Self::Fixed(length) => length.resolve_to_au(font_size, container),
        }
    }
}

impl fmt::Display for Sizing {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => formatter.write_str("auto"),
            Self::Fixed(length) => write!(formatter, "{length}"),
        }
    }
}

/// What a declared `width` / `height` measures (CSS Box Sizing L3 §5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BoxSizing {
    /// The declared size is the **content** box — the CSS `initial` value.
    #[default]
    ContentBox,
    /// The declared size is the **border** box: border and padding are taken
    /// out of it rather than added to it.
    BorderBox,
}

impl BoxSizing {
    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::ContentBox => "content-box",
            Self::BorderBox => "border-box",
        }
    }
}

impl fmt::Display for BoxSizing {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
