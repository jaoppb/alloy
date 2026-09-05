//! [`LayoutBoxTree`] — boxes with resolved geometry, ready for `DisplayList`
//! generation (`PRD-007:40`).
//!
//! Every length is a computed [`Au`] (`ADR-0016`): box arithmetic is integer
//! arithmetic, so the same [`crate::StyledTree`] plus the same
//! [`crate::ViewportConstraints`] produce a byte-identical tree on every
//! platform (`PRD-007:79-80`, `:100`).

use graphics::{Au, Point, Rect, Size};

use crate::domain::computed::intrinsic::IntrinsicSize;
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

/// One laid-out box: which node it belongs to, its content rectangle, the three
/// edges around it, and whether its size is still provisional.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LayoutBox {
    node: SnapshotId,
    content: Rect,
    margin: EdgeSizes,
    border: EdgeSizes,
    padding: EdgeSizes,
    intrinsic_size: IntrinsicSize,
    children: ChildIds,
}

impl LayoutBox {
    #[must_use]
    pub const fn new(
        node: SnapshotId,
        content: Rect,
        edges: BoxEdges,
        intrinsic_size: IntrinsicSize,
        children: ChildIds,
    ) -> Self {
        Self {
            node,
            content,
            margin: edges.margin,
            border: edges.border,
            padding: edges.padding,
            intrinsic_size,
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
    pub const fn border(&self) -> EdgeSizes {
        self.border
    }

    #[must_use]
    pub const fn padding(&self) -> EdgeSizes {
        self.padding
    }

    /// Whether this box's geometry still waits on an unloaded resource
    /// (Phase X reads this).
    #[must_use]
    pub const fn intrinsic_size(&self) -> IntrinsicSize {
        self.intrinsic_size
    }

    #[must_use]
    pub const fn children(&self) -> &ChildIds {
        &self.children
    }

    /// The content box grown by padding and border — what a background and a
    /// border are painted into.
    #[must_use]
    pub fn border_box(&self) -> Rect {
        grown(self.content, self.padding).map_or(self.content, |padded| {
            grown(padded, self.border).unwrap_or(padded)
        })
    }

    /// The border box grown by margin — the space this box occupies in flow.
    #[must_use]
    pub fn margin_box(&self) -> Rect {
        let border_box = self.border_box();
        grown(border_box, self.margin).unwrap_or(border_box)
    }
}

/// The three edges of a box, passed to [`LayoutBox::new`] as one value so the
/// constructor never grows a fourth positional `EdgeSizes` a caller can swap by
/// accident.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct BoxEdges {
    margin: EdgeSizes,
    border: EdgeSizes,
    padding: EdgeSizes,
}

impl BoxEdges {
    /// All three edges zero.
    pub const ZERO: Self = Self {
        margin: EdgeSizes::ZERO,
        border: EdgeSizes::ZERO,
        padding: EdgeSizes::ZERO,
    };

    #[must_use]
    pub const fn new(margin: EdgeSizes, border: EdgeSizes, padding: EdgeSizes) -> Self {
        Self {
            margin,
            border,
            padding,
        }
    }

    #[must_use]
    pub const fn margin(self) -> EdgeSizes {
        self.margin
    }

    #[must_use]
    pub const fn border(self) -> EdgeSizes {
        self.border
    }

    #[must_use]
    pub const fn padding(self) -> EdgeSizes {
        self.padding
    }

    /// `left + right` across all three edges.
    #[must_use]
    pub const fn horizontal(self) -> Au {
        self.margin
            .horizontal()
            .saturating_add(self.border.horizontal())
            .saturating_add(self.padding.horizontal())
    }

    /// `top + bottom` across all three edges.
    #[must_use]
    pub const fn vertical(self) -> Au {
        self.margin
            .vertical()
            .saturating_add(self.border.vertical())
            .saturating_add(self.padding.vertical())
    }
}

/// `rect` expanded by `edges` on all four sides, or `None` when the result is
/// not a representable [`Size`].
fn grown(rect: Rect, edges: EdgeSizes) -> Option<Rect> {
    let inner = rect.size();
    let origin = Point::new(
        rect.min_x().saturating_sub(edges.left()),
        rect.min_y().saturating_sub(edges.top()),
    );
    let outer = Size::new(
        inner.width().saturating_add(edges.horizontal()),
        inner.height().saturating_add(edges.vertical()),
    )?;
    Some(Rect::new(origin, outer))
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

    /// Appends a run of boxes already in document order.
    pub(crate) fn push_all(&mut self, laid_out: impl IntoIterator<Item = LayoutBox>) {
        self.boxes.extend(laid_out);
    }

    pub(crate) fn finish(self, root: Option<SnapshotId>) -> LayoutBoxTree {
        LayoutBoxTree {
            boxes: self.boxes,
            root,
        }
    }
}
