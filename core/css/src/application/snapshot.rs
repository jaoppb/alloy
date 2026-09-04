//! [`snapshot`] — the explicit `dom::DomTree → DomSnapshot` mapping
//! (`PRD-007:36`).
//!
//! This is the **only** file in `core/css` that names a `core/dom` type. It is
//! non-recursive: an explicit work stack of `(dom::NodeId, parent SnapshotId)`
//! frames, never a self-call — the same discipline as
//! `core/dom/src/application/serialize.rs`. Nodes are visited pre-order, so the
//! [`SnapshotId`]s come out in document order with every parent's id smaller
//! than its children's.

use crate::domain::dom_snapshot::{
    AttributeList, DomSnapshot, SnapshotBuilder, SnapshotId, SnapshotNodeKind,
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
    let mut stack: Vec<(dom::NodeId, SnapshotId)> = Vec::new();
    push_children(tree, root, root_id, &mut stack);
    while let Some((dom_node, parent)) = stack.pop() {
        let snapshot_id = add_node(tree, &mut builder, dom_node, Some(parent));
        push_children(tree, dom_node, snapshot_id, &mut stack);
    }
    builder.finish(root_id)
}

/// Maps one `dom` node into the builder and returns its fresh id.
fn add_node(
    tree: &dom::DomTree,
    builder: &mut SnapshotBuilder,
    dom_node: dom::NodeId,
    parent: Option<SnapshotId>,
) -> SnapshotId {
    match tree.node_kind(dom_node) {
        Ok(dom::NodeKind::Element(element)) => builder.add_element(
            parent,
            element.tag().as_str().to_owned(),
            collect_attributes(element),
        ),
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
    AttributeList::from_pairs(
        element
            .attributes()
            .iter()
            .map(|(name, value)| (name.as_str().to_owned(), value.as_str().to_owned())),
    )
}

/// Pushes `dom_node`'s children onto `stack` last-to-first, so the first child
/// is popped — and numbered — next. Walks the sibling links directly rather
/// than collecting, the same way `core/dom/src/application/serialize.rs:117`
/// does.
fn push_children(
    tree: &dom::DomTree,
    dom_node: dom::NodeId,
    parent: SnapshotId,
    stack: &mut Vec<(dom::NodeId, SnapshotId)>,
) {
    let mut cursor = tree.last_child(dom_node).ok().flatten();
    while let Some(child) = cursor {
        stack.push((child, parent));
        cursor = tree.previous_sibling(child).ok().flatten();
    }
}
