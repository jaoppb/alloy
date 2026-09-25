//! [`TextAlign`] and [`WhiteSpace`] — the two inherited properties the inline
//! formatting context of v0.5 B4 reads.
//!
//! Both are exposed to the layout engine as **questions**
//! ([`WhiteSpace::collapses_spaces`], [`WhiteSpace::allows_soft_wrap`],
//! [`WhiteSpace::preserves_newlines`]) rather than as a variant to `match` on:
//! the three behaviours are what `inline.rs` actually needs, and keeping the
//! mapping here means a fourth keyword (`pre-wrap`, v0.7) changes one file.

use core::fmt;

/// How a line box distributes its leftover space (CSS Text L3 §7.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextAlign {
    /// Flush left — the `initial` value for a left-to-right document.
    #[default]
    Left,
    /// Flush right.
    Right,
    /// Centred within the content box.
    Center,
    /// Stretched to both edges by widening the inter-word gaps. The **last**
    /// line of a block is never justified.
    Justify,
}

impl TextAlign {
    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Center => "center",
            Self::Justify => "justify",
        }
    }
}

impl fmt::Display for TextAlign {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// How white space inside an element is treated (CSS Text L3 §4.1.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WhiteSpace {
    /// Runs of white space collapse to one space, newlines are ordinary white
    /// space, and lines wrap — the `initial` value.
    #[default]
    Normal,
    /// White space is preserved verbatim and only a newline breaks a line.
    Pre,
    /// White space collapses as in `normal`, but lines never wrap.
    NoWrap,
}

impl WhiteSpace {
    /// Whether a run of white space becomes a single space.
    #[must_use]
    pub const fn collapses_spaces(self) -> bool {
        matches!(self, Self::Normal | Self::NoWrap)
    }

    /// Whether a line may break at a space or after a hyphen.
    #[must_use]
    pub const fn allows_soft_wrap(self) -> bool {
        matches!(self, Self::Normal)
    }

    /// Whether a `\n` in the source forces a line break.
    #[must_use]
    pub const fn preserves_newlines(self) -> bool {
        matches!(self, Self::Pre)
    }

    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Pre => "pre",
            Self::NoWrap => "nowrap",
        }
    }
}

impl fmt::Display for WhiteSpace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
