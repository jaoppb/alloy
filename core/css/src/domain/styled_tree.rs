//! [`StyledTree`] — the computed value of every node after the cascade
//! (`PRD-007:39`).
//!
//! It mirrors the DOM shape (`parent` + `children` per node) because
//! [`crate::LayoutEngine::layout`] is handed **only** a `&StyledTree`
//! (`PRD-007:56-60`) — the structure has to travel inside the aggregate.
//! Building one goes through [`StyledTree::recompute_in_document_order`], a
//! single parent-before-child pass so an inheriting resolver always sees its
//! parent's finished style.

use crate::domain::computed::intrinsic::{self, IntrinsicSize};
use crate::domain::computed::style::ComputedStyle;
use crate::domain::dom_snapshot::{ChildIds, DomSnapshot, NodeRef, SnapshotId, SnapshotNodeKind};
use crate::domain::text::TextRun;

/// One node's computed style, plus its place in the tree, the character data it
/// carries, and whether its own size is still provisional.
///
/// The last two are what v0.5 B4 added: [`crate::LayoutEngine::layout`] is
/// handed **only** a `&StyledTree` (`PRD-007:56-60`), so an inline formatting
/// context can reach the text only if the text travels inside the aggregate,
/// and Phase X can find the boxes waiting on a resource only if the marker
/// does too.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StyledNode {
    node: SnapshotId,
    parent: Option<SnapshotId>,
    children: ChildIds,
    style: ComputedStyle,
    text: Option<TextRun>,
    intrinsic_size: IntrinsicSize,
}

impl StyledNode {
    #[must_use]
    pub const fn node(&self) -> SnapshotId {
        self.node
    }

    /// The character data of a text node, or `None` for anything else.
    #[must_use]
    pub const fn text(&self) -> Option<&TextRun> {
        self.text.as_ref()
    }

    /// Whether this node's own size still depends on an unloaded resource.
    #[must_use]
    pub const fn intrinsic_size(&self) -> IntrinsicSize {
        self.intrinsic_size
    }

    #[must_use]
    pub const fn parent(&self) -> Option<SnapshotId> {
        self.parent
    }

    #[must_use]
    pub const fn children(&self) -> &ChildIds {
        &self.children
    }

    #[must_use]
    pub const fn style(&self) -> &ComputedStyle {
        &self.style
    }
}

/// The whole tree of computed styles.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StyledTree {
    nodes: Vec<StyledNode>,
    root: SnapshotId,
}

impl StyledTree {
    /// The id of the root node.
    #[must_use]
    pub const fn root(&self) -> SnapshotId {
        self.root
    }

    /// The styled node for `id`, or `None`.
    #[must_use]
    pub fn node(&self, id: SnapshotId) -> Option<&StyledNode> {
        self.nodes.get(id.index())
    }

    /// Every styled node in document order.
    pub fn nodes_in_document_order(&self) -> impl Iterator<Item = &StyledNode> + '_ {
        self.nodes.iter()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Builds a styled tree by calling `compute` once per node, in document
    /// order, passing the parent's already-computed style (or `None` at the
    /// root). Pre-order construction of the snapshot guarantees the parent is
    /// finished before the child is visited.
    #[must_use]
    pub fn recompute_in_document_order(
        snapshot: &DomSnapshot,
        mut compute: impl FnMut(NodeRef<'_>, Option<&ComputedStyle>) -> ComputedStyle,
    ) -> Self {
        let mut nodes: Vec<StyledNode> = Vec::with_capacity(snapshot.len());
        for id in snapshot.nodes_in_document_order() {
            let Some(node_ref) = snapshot.node(id) else {
                continue;
            };
            let style = compute(node_ref, parent_style_of(&nodes, node_ref));
            nodes.push(StyledNode {
                node: id,
                parent: node_ref.parent(),
                children: ChildIds::from_ids(node_ref.children()),
                style,
                text: character_data_of(node_ref),
                intrinsic_size: intrinsic::for_tag(node_ref.tag()),
            });
        }
        Self {
            nodes,
            root: snapshot.root(),
        }
    }
}

/// The text a node contributes to an inline formatting context. Only a `Text`
/// node has any: a comment's character data is markup, never rendered content.
fn character_data_of(node_ref: NodeRef<'_>) -> Option<TextRun> {
    if node_ref.kind() != SnapshotNodeKind::Text {
        return None;
    }
    node_ref.text().map(TextRun::new)
}

/// The already-computed style of `node_ref`'s parent, looked up by index — safe
/// because a parent's id is always smaller than its child's.
fn parent_style_of<'nodes>(
    nodes: &'nodes [StyledNode],
    node_ref: NodeRef<'_>,
) -> Option<&'nodes ComputedStyle> {
    node_ref
        .parent()
        .and_then(|parent_id| nodes.get(parent_id.index()))
        .map(StyledNode::style)
}
