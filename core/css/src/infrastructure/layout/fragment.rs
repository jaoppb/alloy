//! [`Fragment`] and [`Fragments`] — a laid-out box before it knows where it is.
//!
//! Every formatting context of v0.5 B4 returns fragments positioned
//! **relative** to the origin its caller will place it at (the border box for a
//! block result, the content box for a children flow), and the caller
//! translates. That is what lets `block.rs`, `inline.rs` and `flex.rs` share one
//! result shape without any of them owning a page-global cursor.

use graphics::{Au, Point, Rect, Size};

use crate::domain::computed::intrinsic::IntrinsicSize;
use crate::domain::dom_snapshot::{ChildIds, SnapshotId};
use crate::domain::layout_box_tree::{BoxEdges, LayoutBox};

/// One box with a relative rectangle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment {
    node: SnapshotId,
    content: Rect,
    edges: BoxEdges,
    intrinsic_size: IntrinsicSize,
    children: ChildIds,
}

impl Fragment {
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
            edges,
            intrinsic_size,
            children,
        }
    }

    /// The same fragment moved by `(horizontal, vertical)`.
    pub fn translated(self, horizontal: Au, vertical: Au) -> Self {
        let origin = Point::new(
            self.content.min_x().saturating_add(horizontal),
            self.content.min_y().saturating_add(vertical),
        );
        Self {
            content: Rect::new(origin, self.content.size()),
            ..self
        }
    }

    fn into_layout_box(self) -> LayoutBox {
        LayoutBox::new(
            self.node,
            self.content,
            self.edges,
            self.intrinsic_size,
            self.children,
        )
    }
}

/// A run of fragments in document order. A first-class collection — no public
/// `Vec` crosses a layout seam.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fragments {
    items: Vec<Fragment>,
}

impl Fragments {
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn one(fragment: Fragment) -> Self {
        Self {
            items: vec![fragment],
        }
    }

    pub fn push(&mut self, fragment: Fragment) {
        self.items.push(fragment);
    }

    /// Appends `other`, keeping both runs in their own order.
    pub fn absorb(&mut self, other: Self) {
        self.items.extend(other.items);
    }

    /// Every fragment moved by `(horizontal, vertical)`.
    pub fn translated(self, horizontal: Au, vertical: Au) -> Self {
        Self {
            items: self
                .items
                .into_iter()
                .map(|fragment| fragment.translated(horizontal, vertical))
                .collect(),
        }
    }

    pub fn into_boxes(self) -> impl Iterator<Item = LayoutBox> {
        self.items.into_iter().map(Fragment::into_layout_box)
    }
}

/// A rectangle at `origin` sized `width × height`, saturating an unrepresentable
/// size to empty rather than refusing to lay the box out at all.
pub fn rect_at(origin: Point, width: Au, height: Au) -> Rect {
    let size = Size::new(width.larger(Au::ZERO), height.larger(Au::ZERO));
    Rect::new(origin, size.unwrap_or(Size::EMPTY))
}
