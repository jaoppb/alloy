//! [`Overflow`] and [`OverflowStyle`] — how content that exceeds its box is
//! clipped or scrolled (CSS Overflow L3 §3).

use core::fmt;

/// How content is handled when it overflows the box edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Overflow {
    /// Content is not clipped and renders outside the box — the `initial` value.
    #[default]
    Visible,
    /// Content is clipped, no scrollbars provided.
    Hidden,
    /// Content is clipped to the overflow clip edge.
    Clip,
    /// Scrolling mechanism is always provided.
    Scroll,
    /// Scrolling mechanism provided only when content overflows.
    Auto,
}

impl Overflow {
    /// Whether content overflowing this box is clipped.
    #[must_use]
    pub const fn is_clipped(self) -> bool {
        matches!(self, Self::Hidden | Self::Clip | Self::Scroll | Self::Auto)
    }

    /// Keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Visible => "visible",
            Self::Hidden => "hidden",
            Self::Clip => "clip",
            Self::Scroll => "scroll",
            Self::Auto => "auto",
        }
    }
}

impl fmt::Display for Overflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// The horizontal and vertical overflow behaviors grouped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct OverflowStyle {
    x: Overflow,
    y: Overflow,
}

impl OverflowStyle {
    /// Both axes visible — CSS `initial`.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            x: Overflow::Visible,
            y: Overflow::Visible,
        }
    }

    /// Same overflow behavior on both axes.
    #[must_use]
    pub const fn uniform(overflow: Overflow) -> Self {
        Self {
            x: overflow,
            y: overflow,
        }
    }

    #[must_use]
    pub const fn x(self) -> Overflow {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> Overflow {
        self.y
    }

    #[must_use]
    pub const fn with_x(self, x: Overflow) -> Self {
        Self { x, ..self }
    }

    #[must_use]
    pub const fn with_y(self, y: Overflow) -> Self {
        Self { y, ..self }
    }
}
