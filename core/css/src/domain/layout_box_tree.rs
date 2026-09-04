//! [`LayoutBoxTree`] — boxes with resolved geometry, ready for `DisplayList`
//! generation (`PRD-007:40`).
//!
//! Every length is a computed [`Au`] (`ADR-0016`): box arithmetic is integer
//! arithmetic, so the same [`crate::StyledTree`] plus the same
//! [`crate::ViewportConstraints`] produce a byte-identical tree on every
//! platform (`PRD-007:79-80`, `:100`).

use graphics::{Au, Rect};

use crate::domain::dom_snapshot::{ChildIds, SnapshotId};

/// The four sides of a box's margin or padding, resolved to [`Au`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct EdgeSizes {
    top: Au,
    right: Au,
    bottom: Au,
    left: Au,
}

impl EdgeSizes {
    /// All four sides zero.
    pub const ZERO: Self = Self {
        top: Au::ZERO,
        right: Au::ZERO,
        bottom: Au::ZERO,
        left: Au::ZERO,
    };

    #[must_use]
    pub const fn new(top: Au, right: Au, bottom: Au, left: Au) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// `left + right`, saturating at the `Au` extreme.
    #[must_use]
    pub const fn horizontal(self) -> Au {
        self.left.saturating_add(self.right)
    }

    /// `top + bottom`, saturating at the `Au` extreme.
    #[must_use]
    pub const fn vertical(self) -> Au {
        self.top.saturating_add(self.bottom)
    }

    #[must_use]
    pub const fn top(self) -> Au {
        self.top
    }

    #[must_use]
    pub const fn right(self) -> Au {
        self.right
    }

    #[must_use]
    pub const fn bottom(self) -> Au {
        self.bottom
    }

    #[must_use]
    pub const fn left(self) -> Au {
        self.left
    }
}

/// One laid-out box: which node it belongs to, its content rectangle, and the
/// margin and padding around it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LayoutBox {
    node: SnapshotId,
    content: Rect,
    margin: EdgeSizes,
    padding: EdgeSizes,
    children: ChildIds,
}

impl LayoutBox {
    #[must_use]
    pub const fn new(
        node: SnapshotId,
        content: Rect,
        margin: EdgeSizes,
        padding: EdgeSizes,
        children: ChildIds,
    ) -> Self {
        Self {
            node,
            content,
            margin,
            padding,
            children,
        }
    }

    #[must_use]
    pub const fn node(&self) -> SnapshotId {
        self.node
    }

    #[must_use]
    pub const fn content(&self) -> Rect {
        self.content
    }

    #[must_use]
    pub const fn margin(&self) -> EdgeSizes {
        self.margin
    }

    #[must_use]
    pub const fn padding(&self) -> EdgeSizes {
        self.padding
    }

    #[must_use]
    pub const fn children(&self) -> &ChildIds {
        &self.children
    }
}

/// The tree of laid-out boxes. Nodes with `display: none` generate no box, so
/// this is not indexed by [`SnapshotId`] — look a box up with
/// [`LayoutBoxTree::box_of`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct LayoutBoxTree {
    boxes: Vec<LayoutBox>,
    root: Option<SnapshotId>,
}

impl LayoutBoxTree {
    /// The id of the root box, or `None` when the root generated no box.
    #[must_use]
    pub const fn root(&self) -> Option<SnapshotId> {
        self.root
    }

    /// The box for `node`, or `None` when it generated none.
    #[must_use]
    pub fn box_of(&self, node: SnapshotId) -> Option<&LayoutBox> {
        self.boxes.iter().find(|laid_out| laid_out.node == node)
    }

    /// Every box in document order.
    pub fn boxes_in_document_order(&self) -> impl Iterator<Item = &LayoutBox> + '_ {
        self.boxes.iter()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.boxes.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }
}

/// Accumulates boxes during layout. Crate-internal — a `LayoutBoxTree` only
/// ever comes out of a [`crate::LayoutEngine`].
pub(crate) struct LayoutBoxTreeBuilder {
    boxes: Vec<LayoutBox>,
}

impl LayoutBoxTreeBuilder {
    pub(crate) const fn new() -> Self {
        Self { boxes: Vec::new() }
    }

    pub(crate) fn push(&mut self, laid_out: LayoutBox) {
        self.boxes.push(laid_out);
    }

    pub(crate) fn finish(self, root: Option<SnapshotId>) -> LayoutBoxTree {
        LayoutBoxTree {
            boxes: self.boxes,
            root,
        }
    }
}
