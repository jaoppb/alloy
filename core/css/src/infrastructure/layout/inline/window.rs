//! Block-in-inline (CSS 2.1 §9.2.1.1, #135): which stretch of an inline box
//! one run of line boxes holds.
//!
//! A block-level box inside an inline box splits the inline run around it:
//! the content before it is one anonymous block of lines, the block is
//! stacked like a sibling, the content after it is another anonymous block.
//! `block.rs` finds the block-level descendants ([`block_descendants`]) and
//! hands each run an [`InlineItem`] bounded by the blocks on either side; the
//! inline layout then walks the item in document order and keeps only the
//! content between those bounds ([`Walk`]).

use crate::application::ports::TextMeasurer;
use crate::domain::dom_snapshot::{ChildIds, SnapshotId};
use crate::domain::error::CssError;
use crate::infrastructure::layout::context::LayoutContext;

/// One inline-level child of a block container, as one run sees it: the whole
/// node, or — when block-level descendants split it — only the stretch after
/// one of them and before the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InlineItem {
    node: SnapshotId,
    after: Option<SnapshotId>,
    before: Option<SnapshotId>,
}

impl InlineItem {
    /// `node`'s content after the block-level descendant `after` (from its
    /// start when `None`) and before `before` (to its end when `None`).
    pub const fn between(
        node: SnapshotId,
        after: Option<SnapshotId>,
        before: Option<SnapshotId>,
    ) -> Self {
        Self {
            node,
            after,
            before,
        }
    }

    pub const fn node(self) -> SnapshotId {
        self.node
    }

    /// A fresh document-order walk over this item.
    pub(super) const fn walk(self) -> Walk {
        Walk {
            after: self.after,
            before: self.before,
            inside: self.after.is_none(),
            past: false,
        }
    }
}

/// Where a document-order walk of one [`InlineItem`] stands: before its
/// stretch, inside it, or past it.
pub(super) struct Walk {
    after: Option<SnapshotId>,
    before: Option<SnapshotId>,
    inside: bool,
    past: bool,
}

impl Walk {
    /// A walk with no bounds, before any item is collected: everything it
    /// meets belongs to the run.
    pub(super) const fn unbounded() -> Self {
        Self {
            after: None,
            before: None,
            inside: true,
            past: false,
        }
    }

    /// Whether text or an atomic inline met now belongs to this run.
    pub(super) const fn takes_content(&self) -> bool {
        self.inside && !self.past
    }

    /// Whether the walk has left its stretch: nothing later belongs to it.
    pub(super) const fn is_done(&self) -> bool {
        self.past
    }

    /// Records passing the block-level descendant `block`, which no run
    /// walks into: it opens the stretch when it is the item's `after`
    /// bound and closes it when it is the `before` bound.
    pub(super) fn cross(&mut self, block: SnapshotId) {
        if self.before == Some(block) {
            self.past = true;
            return;
        }
        if self.after == Some(block) {
            self.inside = true;
        }
    }
}

/// The block-level boxes inside the inline box `node_id`, in document order,
/// reached through its inline (not atomic) descendants: the boxes that split
/// its run. Empty for an atomic inline, which lays its own content out.
pub fn block_descendants<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
) -> Result<ChildIds, CssError> {
    let styled = context.node(node_id)?;
    let mut found = Vec::new();
    if styled.style().display().is_atomic_inline() {
        return Ok(ChildIds::from_ids(found));
    }
    gather_children(context, node_id, &mut found)?;
    Ok(ChildIds::from_ids(found))
}

fn gather_children<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    found: &mut Vec<SnapshotId>,
) -> Result<(), CssError> {
    let styled = context.node(node_id)?;
    for child in styled.children().iter() {
        gather(context, child, found)?;
    }
    Ok(())
}

/// A block-level descendant is a split point and is not walked into; an
/// inline box is walked through; an atomic inline, a `display: none` box and
/// foreign content hold no split point of this box.
fn gather<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    found: &mut Vec<SnapshotId>,
) -> Result<(), CssError> {
    let styled = context.node(node_id)?;
    let display = styled.style().display();
    if styled.is_foreign_descendant() || display.is_none() || display.is_atomic_inline() {
        return Ok(());
    }
    if !display.is_inline_level() {
        found.push(node_id);
        return Ok(());
    }
    gather_children(context, node_id, found)
}
