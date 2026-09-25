//! [`PositionStyle`] — the computed values of the positioning module
//! (`position`, `top`, `right`, `bottom`, `left`, `z-index`).

use core::fmt;

use crate::domain::computed::sizing::Sizing;

/// How a box is positioned in the formatting context (CSS Positioned Layout L3 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PositionType {
    /// In normal flow — the `initial` value.
    #[default]
    Static,
    /// Offset relative to normal position.
    Relative,
    /// Removed from flow, positioned relative to containing block.
    Absolute,
    /// Positioned relative to the viewport.
    Fixed,
    /// Hybrid relative/fixed depending on scroll position.
    Sticky,
}

impl PositionType {
    /// Whether this box is in normal flow.
    #[must_use]
    pub const fn is_in_flow(self) -> bool {
        matches!(self, Self::Static | Self::Relative | Self::Sticky)
    }

    /// Whether this box is positioned (non-static).
    #[must_use]
    pub const fn is_positioned(self) -> bool {
        !matches!(self, Self::Static)
    }

    /// Keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Relative => "relative",
            Self::Absolute => "absolute",
            Self::Fixed => "fixed",
            Self::Sticky => "sticky",
        }
    }
}

impl fmt::Display for PositionType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

/// The stacking order along the z-axis (CSS Positioned Layout L3 §4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ZIndex {
    /// Stack level determined by tree order — the `initial` value.
    #[default]
    Auto,
    /// Explicit integer stack level.
    Index(i32),
}

impl fmt::Display for ZIndex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => formatter.write_str("auto"),
            Self::Index(level) => write!(formatter, "{level}"),
        }
    }
}

/// A node's positioning properties grouped into one aggregate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PositionStyle {
    position: PositionType,
    top: Sizing,
    right: Sizing,
    bottom: Sizing,
    left: Sizing,
    z_index: ZIndex,
}

impl PositionStyle {
    /// Every positioning property at its CSS `initial` value.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            position: PositionType::Static,
            top: Sizing::Auto,
            right: Sizing::Auto,
            bottom: Sizing::Auto,
            left: Sizing::Auto,
            z_index: ZIndex::Auto,
        }
    }

    #[must_use]
    pub const fn position(self) -> PositionType {
        self.position
    }

    #[must_use]
    pub const fn top(self) -> Sizing {
        self.top
    }

    #[must_use]
    pub const fn right(self) -> Sizing {
        self.right
    }

    #[must_use]
    pub const fn bottom(self) -> Sizing {
        self.bottom
    }

    #[must_use]
    pub const fn left(self) -> Sizing {
        self.left
    }

    #[must_use]
    pub const fn z_index(self) -> ZIndex {
        self.z_index
    }

    #[must_use]
    pub const fn with_position(self, position: PositionType) -> Self {
        Self { position, ..self }
    }

    #[must_use]
    pub const fn with_top(self, top: Sizing) -> Self {
        Self { top, ..self }
    }

    #[must_use]
    pub const fn with_right(self, right: Sizing) -> Self {
        Self { right, ..self }
    }

    #[must_use]
    pub const fn with_bottom(self, bottom: Sizing) -> Self {
        Self { bottom, ..self }
    }

    #[must_use]
    pub const fn with_left(self, left: Sizing) -> Self {
        Self { left, ..self }
    }

    #[must_use]
    pub const fn with_z_index(self, z_index: ZIndex) -> Self {
        Self { z_index, ..self }
    }
}
