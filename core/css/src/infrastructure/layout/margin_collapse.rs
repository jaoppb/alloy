//! Vertical margin collapsing (CSS 2.1 §8.3.1) — step 1 of v0.5 B4.
//!
//! Two adjoining vertical margins do not add up: they become **one** margin
//! whose size is the larger of the positive ones plus the smaller (most
//! negative) of the negative ones. [`CollapsedMargin`] is that pair, and
//! [`CollapsedMargin::adjoin`] is the associative, commutative operation with
//! [`CollapsedMargin::ZERO`] as its identity — three properties the unit tests
//! assert directly, because every rectangle in a block layout rests on them.
//!
//! Which margins adjoin is decided by the **parent**, from its own edges:
//! [`collapses_at_top`] and [`collapses_at_bottom`] are the two conditions of
//! §8.3.1 written once.

use graphics::Au;

use crate::domain::layout_box_tree::BoxEdges;

/// A set of adjoining margins, reduced to the one margin they produce.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CollapsedMargin {
    positive: Au,
    negative: Au,
}

impl CollapsedMargin {
    /// The identity of [`CollapsedMargin::adjoin`].
    pub const ZERO: Self = Self {
        positive: Au::ZERO,
        negative: Au::ZERO,
    };

    /// One margin on its own.
    pub const fn from_length(value: Au) -> Self {
        if value.is_negative() {
            return Self {
                positive: Au::ZERO,
                negative: value,
            };
        }
        Self {
            positive: value,
            negative: Au::ZERO,
        }
    }

    /// The set holding both this one's margins and `other`'s.
    pub const fn adjoin(self, other: Self) -> Self {
        Self {
            positive: self.positive.larger(other.positive),
            negative: self.negative.smaller(other.negative),
        }
    }

    /// The single margin the set collapses to.
    pub const fn resolve(self) -> Au {
        self.positive.saturating_add(self.negative)
    }
}

/// Whether a box's top margin adjoins the top margin of its first in-flow
/// child: it does unless a top border or top padding separates them
/// (CSS 2.1 §8.3.1).
pub const fn collapses_at_top(edges: BoxEdges) -> bool {
    let border = edges.border();
    let padding = edges.padding();
    border.top().is_zero() && padding.top().is_zero()
}

/// Whether a box's bottom margin adjoins the bottom margin of its last in-flow
/// child: it does unless a bottom border, bottom padding, or a declared height
/// separates them (CSS 2.1 §8.3.1).
pub const fn collapses_at_bottom(edges: BoxEdges, declared_height: Option<Au>) -> bool {
    if declared_height.is_some() {
        return false;
    }
    let border = edges.border();
    let padding = edges.padding();
    border.bottom().is_zero() && padding.bottom().is_zero()
}

/// Whether a box's own top and bottom margins end up adjoining each other —
/// the "collapsed through" case of a box with no height, no border and no
/// padding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarginFlow {
    /// Content, a border or a padding keeps the two margins apart.
    Separated,
    /// Nothing separates them: the box's margins join its neighbours' as one
    /// set, and the box occupies no vertical space.
    CollapsesThrough,
}

impl MarginFlow {
    pub const fn collapses_through(self) -> bool {
        matches!(self, Self::CollapsesThrough)
    }
}
