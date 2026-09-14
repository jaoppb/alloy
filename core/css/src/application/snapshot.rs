//! [`snapshot`] — the explicit `dom::DomTree → DomSnapshot` mapping
//! (`PRD-007:36`).
//!
//! Non-recursive: an explicit [`WorkStack`] of pending `(dom::NodeId, parent
//! SnapshotId)` frames, never a self-call — the same discipline as
//! `core/dom/src/application/serialize.rs`. Nodes are visited pre-order, so the
//! [`SnapshotId`]s come out in document order with every parent's id smaller
//! than its children's.

use crate::domain::dom_snapshot::{
    AttributeKey, AttributeList, AttributeValue, DomSnapshot, SnapshotBuilder, SnapshotId,
    SnapshotNodeKind,
};

/// Project the subtree of `tree` rooted at `root` into a [`DomSnapshot`].
///
/// Infallible: an unresolvable `root` projects to a lone `Document` node rather
/// than an error — a resolver handed an empty snapshot still produces an empty
/// styled tree, and the page still renders.
#[must_use]
pub fn snapshot(tree: &dom::DomTree, root: dom::NodeId) -> DomSnapshot {
    let mut builder = SnapshotBuilder::new();
    let root_id = add_node(tree, &mut builder, root, None);
    let mut stack = WorkStack::new();
    push_children(tree, root, root_id, &mut stack);
    while let Some((dom_node, parent)) = stack.pop() {
        let snapshot_id = add_node(tree, &mut builder, dom_node, Some(parent));
        push_children(tree, dom_node, snapshot_id, &mut stack);
    }
    builder.finish(root_id)
}

/// One pending `(dom node, projected parent)` frame still to visit.
struct Frame {
    dom_node: dom::NodeId,
    parent: SnapshotId,
}

/// The explicit LIFO work stack [`snapshot`] walks instead of recursing.
///
/// A first-class collection (`ADR-0010` rule 3) — no naked `Vec` of tuples —
/// so a caller reads `push`/`pop` instead of re-deriving what the two-element
/// tuple means at every call site.
struct WorkStack {
    frames: Vec<Frame>,
}

impl WorkStack {
    const fn new() -> Self {
        Self { frames: Vec::new() }
    }

    fn push(&mut self, dom_node: dom::NodeId, parent: SnapshotId) {
        self.frames.push(Frame { dom_node, parent });
    }

    /// Pops the most recently pushed frame, or `None` once exhausted.
    fn pop(&mut self) -> Option<(dom::NodeId, SnapshotId)> {
        self.frames
            .pop()
            .map(|frame| (frame.dom_node, frame.parent))
    }
}

/// Maps one `dom` node into the builder and returns its fresh id.
fn add_node(
    tree: &dom::DomTree,
    builder: &mut SnapshotBuilder,
    dom_node: dom::NodeId,
    parent: Option<SnapshotId>,
) -> SnapshotId {
    match tree.node_kind(dom_node) {
        Ok(dom::NodeKind::Element(element)) => {
            builder.add_element(parent, element.tag().clone(), collect_attributes(element))
        }
        Ok(dom::NodeKind::Text(content)) => {
            builder.add_character_data(SnapshotNodeKind::Text, parent, content.as_str().to_owned())
        }
        Ok(dom::NodeKind::Comment(content)) => builder.add_character_data(
            SnapshotNodeKind::Comment,
            parent,
            content.as_str().to_owned(),
        ),
        Ok(dom::NodeKind::Document) | Err(_) => builder.add_document(parent),
    }
}

/// Copies an element's attributes into the projection's own owned form.
fn collect_attributes(element: &dom::ElementData) -> AttributeList {
    AttributeList::from_pairs(element.attributes().iter().map(|(name, value)| {
        (
            AttributeKey::from_dom(name),
            AttributeValue::new(value.as_str()),
        )
    }))
}

/// Pushes `dom_node`'s children onto `stack` last-to-first, so the first child
/// is popped — and numbered — next. Walks the sibling links directly rather
/// than collecting, the same way `core/dom/src/application/serialize.rs:117`
/// does.
fn push_children(
    tree: &dom::DomTree,
    dom_node: dom::NodeId,
    parent: SnapshotId,
    stack: &mut WorkStack,
) {
    let mut cursor = tree.last_child(dom_node).ok().flatten();
    while let Some(child) = cursor {
        stack.push(child, parent);
        cursor = tree.previous_sibling(child).ok().flatten();
    }
}
