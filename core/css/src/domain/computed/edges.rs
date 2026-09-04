//! [`LengthEdges`] — the four sides of a box property (`margin`, `padding`)
//! before resolution, each a [`Length`].

use crate::domain::length::Length;

/// A `top` / `right` / `bottom` / `left` quartet of author lengths.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct LengthEdges {
    top: Length,
    right: Length,
    bottom: Length,
    left: Length,
}

impl LengthEdges {
    /// All four sides zero.
    pub const ZERO: Self = Self {
        top: Length::ZERO,
        right: Length::ZERO,
        bottom: Length::ZERO,
        left: Length::ZERO,
    };

    /// Distinct lengths per side.
    #[must_use]
    pub const fn new(top: Length, right: Length, bottom: Length, left: Length) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The same length on every side.
    #[must_use]
    pub const fn uniform(length: Length) -> Self {
        Self::new(length, length, length, length)
    }

    /// Vertical margins/padding: `top` and `bottom` equal, `right` and `left`
    /// zero — the shorthand the UA rules for `<p>` and `<h1>` use.
    #[must_use]
    pub const fn vertical(length: Length) -> Self {
        Self::new(length, Length::ZERO, length, Length::ZERO)
    }

    /// The same quartet with one side replaced — what the `margin-top` /
    /// `padding-left` longhands apply, leaving the other three alone.
    #[must_use]
    pub const fn with_top(self, top: Length) -> Self {
        Self { top, ..self }
    }

    #[must_use]
    pub const fn with_right(self, right: Length) -> Self {
        Self { right, ..self }
    }

    #[must_use]
    pub const fn with_bottom(self, bottom: Length) -> Self {
        Self { bottom, ..self }
    }

    #[must_use]
    pub const fn with_left(self, left: Length) -> Self {
        Self { left, ..self }
    }

    #[must_use]
    pub const fn top(self) -> Length {
        self.top
    }

    #[must_use]
    pub const fn right(self) -> Length {
        self.right
    }

    #[must_use]
    pub const fn bottom(self) -> Length {
        self.bottom
    }

    #[must_use]
    pub const fn left(self) -> Length {
        self.left
    }
}
