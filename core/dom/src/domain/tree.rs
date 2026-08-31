//! [`DomTree`] — the arena aggregate.
//!
//! It owns every node and is the *only* way
//! to mutate the tree; the five invariants of the v0.2 report §2.2 (acyclicity,
//! single parent, no self-parent, an irremovable `Document` root, `Children` ⇄
//! `parent` coherence) are enforced here and nowhere else (`ADR-0010:136`
//! rule 8: no public mutable field).
//!
//! Traversal ([`DomTree::descendants`], [`DomTree::ancestors`]) and
//! serialization ([`crate::serialize_html`]) are read-only and never recurse.

use crate::domain::attributes::{AttributeName, AttributeValue};
use crate::domain::children::Children;
use crate::domain::error::DomError;
use crate::domain::node::{ElementData, NodeData, NodeId, NodeKind, Slot};
use crate::domain::tag_name::TagName;
use crate::domain::text::{CommentContent, TextContent};
use crate::domain::traversal::{Ancestors, Descendants};

/// A whole document tree: an arena of slots plus the id of the single
/// `Document` root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomTree {
    slots: Vec<Slot>,
    document: NodeId,
}

/// Where in a parent's child list [`DomTree::attach`] places a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Attachment {
    End,
    Before(NodeId),
}

impl DomTree {
    /// A fresh tree holding only an empty `Document` root.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: vec![Slot::Occupied(NodeData::new(NodeKind::Document))],
            document: NodeId::from_index(0),
        }
    }

    /// The id of the `Document` root — always valid, never removable.
    #[must_use]
    pub const fn document(&self) -> NodeId {
        self.document
    }

    // ---- creation (nodes start detached) -------------------------------

    pub fn create_element(&mut self, tag: TagName) -> NodeId {
        self.push_node(NodeData::new(NodeKind::Element(ElementData::new(tag))))
    }

    pub fn create_text(&mut self, content: TextContent) -> NodeId {
        self.push_node(NodeData::new(NodeKind::Text(content)))
    }

    pub fn create_comment(&mut self, content: CommentContent) -> NodeId {
        self.push_node(NodeData::new(NodeKind::Comment(content)))
    }

    // ---- structural mutation (invariant-checked) ----------------------

    /// Append `child` as the last child of `parent`, moving it from any current
    /// parent. Enforces invariants 1–3 (report §2.2).
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), DomError> {
        self.attach(parent, child, Attachment::End)
    }

    /// Insert `new_child` immediately before `anchor` in `parent`'s child list.
    pub fn insert_before(
        &mut self,
        parent: NodeId,
        new_child: NodeId,
        anchor: NodeId,
    ) -> Result<(), DomError> {
        self.attach(parent, new_child, Attachment::Before(anchor))
    }

    /// Unlink `node` from its parent, leaving it detached but still in the arena.
    pub fn detach(&mut self, node: NodeId) -> Result<(), DomError> {
        self.reject_document(node)?;
        self.node(node)?;
        self.detach_from_parent(node)
    }

    /// Unlink `node` and tombstone it together with its whole subtree. Ids into
    /// the removed subtree then resolve to [`DomError::NodeNotFound`].
    pub fn remove(&mut self, node: NodeId) -> Result<(), DomError> {
        self.reject_document(node)?;
        self.detach_from_parent(node)?;
        for doomed in self.collect_subtree(node)? {
            self.tombstone(doomed);
        }
        Ok(())
    }

    /// Replace the content of a `Text` node.
    pub fn set_text(&mut self, node: NodeId, content: TextContent) -> Result<(), DomError> {
        match self.node_mut(node)?.kind_mut() {
            NodeKind::Text(existing) => {
                *existing = content;
                Ok(())
            }
            _ => Err(DomError::NotCharacterData(node)),
        }
    }

    /// Set (or update in place) an attribute on an `Element`.
    pub fn set_attribute(
        &mut self,
        node: NodeId,
        name: AttributeName,
        value: AttributeValue,
    ) -> Result<(), DomError> {
        self.element_mut(node)?.attributes_mut().set(name, value);
        Ok(())
    }

    /// Remove an attribute from an `Element` if present.
    pub fn remove_attribute(&mut self, node: NodeId, name: &AttributeName) -> Result<(), DomError> {
        self.element_mut(node)?.attributes_mut().remove(name);
        Ok(())
    }

    // ---- reads ------------------------------------------------------

    pub fn node_kind(&self, node: NodeId) -> Result<&NodeKind, DomError> {
        self.node(node).map(NodeData::kind)
    }

    pub fn parent(&self, node: NodeId) -> Result<Option<NodeId>, DomError> {
        self.node(node).map(NodeData::parent)
    }

    /// A snapshot of `node`'s children in document order.
    pub fn child_ids(&self, node: NodeId) -> Result<Children, DomError> {
        self.node(node).map(|data| data.children().clone())
    }

    pub fn tag(&self, node: NodeId) -> Result<&TagName, DomError> {
        match self.node(node)?.kind() {
            NodeKind::Element(element) => Ok(element.tag()),
            _ => Err(DomError::NotAnElement(node)),
        }
    }

    pub fn attribute(
        &self,
        node: NodeId,
        name: &AttributeName,
    ) -> Result<Option<&AttributeValue>, DomError> {
        match self.node(node)?.kind() {
            NodeKind::Element(element) => Ok(element.attributes().get(name)),
            _ => Err(DomError::NotAnElement(node)),
        }
    }

    pub fn text(&self, node: NodeId) -> Result<&TextContent, DomError> {
        match self.node(node)?.kind() {
            NodeKind::Text(content) => Ok(content),
            _ => Err(DomError::NotCharacterData(node)),
        }
    }

    /// Pre-order (document-order) iterator over the strict descendants of `root`.
    #[must_use]
    pub fn descendants(&self, root: NodeId) -> Descendants<'_> {
        Descendants::new(self, root)
    }

    /// Iterator from `node`'s parent up to and including the `Document` root.
    #[must_use]
    pub fn ancestors(&self, node: NodeId) -> Ancestors<'_> {
        Ancestors::new(self, node)
    }

    // ---- crate-internal helpers for traversal / serialize ----------

    pub(crate) fn child_id_vec(&self, node: NodeId) -> Vec<NodeId> {
        self.node(node)
            .map(|data| data.children().iter().collect())
            .unwrap_or_default()
    }

    pub(crate) fn parent_of(&self, node: NodeId) -> Option<NodeId> {
        self.node(node).ok().and_then(NodeData::parent)
    }

    // ---- private ------------------------------------------------

    fn push_node(&mut self, data: NodeData) -> NodeId {
        let id = NodeId::from_index(self.slots.len());
        self.slots.push(Slot::Occupied(data));
        id
    }

    fn node(&self, node: NodeId) -> Result<&NodeData, DomError> {
        match self.slots.get(node.index()) {
            Some(Slot::Occupied(data)) => Ok(data),
            _ => Err(DomError::NodeNotFound(node)),
        }
    }

    fn node_mut(&mut self, node: NodeId) -> Result<&mut NodeData, DomError> {
        match self.slots.get_mut(node.index()) {
            Some(Slot::Occupied(data)) => Ok(data),
            _ => Err(DomError::NodeNotFound(node)),
        }
    }

    fn element_mut(&mut self, node: NodeId) -> Result<&mut ElementData, DomError> {
        match self.node_mut(node)?.kind_mut() {
            NodeKind::Element(element) => Ok(element),
            _ => Err(DomError::NotAnElement(node)),
        }
    }

    fn attach(
        &mut self,
        parent: NodeId,
        child: NodeId,
        position: Attachment,
    ) -> Result<(), DomError> {
        Self::reject_self_parent(parent, child)?;
        self.ensure_container(parent)?;
        self.node(child)?;
        self.reject_cycle(parent, child)?;
        self.detach_from_parent(child)?;
        self.place(parent, child, position)?;
        self.node_mut(child)?.set_parent(Some(parent));
        Ok(())
    }

    fn reject_self_parent(parent: NodeId, child: NodeId) -> Result<(), DomError> {
        if parent == child {
            return Err(DomError::SelfParent);
        }
        Ok(())
    }

    fn ensure_container(&self, parent: NodeId) -> Result<(), DomError> {
        match self.node(parent)?.kind() {
            NodeKind::Document | NodeKind::Element(_) => Ok(()),
            _ => Err(DomError::CannotHaveChildren(parent)),
        }
    }

    fn reject_cycle(&self, parent: NodeId, child: NodeId) -> Result<(), DomError> {
        let would_cycle =
            parent == child || self.ancestors(parent).any(|ancestor| ancestor == child);
        if would_cycle {
            return Err(DomError::WouldCycle);
        }
        Ok(())
    }

    fn reject_document(&self, node: NodeId) -> Result<(), DomError> {
        if node == self.document {
            return Err(DomError::CannotDetachDocument);
        }
        Ok(())
    }

    fn detach_from_parent(&mut self, child: NodeId) -> Result<(), DomError> {
        match self.node(child)?.parent() {
            None => Ok(()),
            Some(parent) => {
                self.node_mut(parent)?.children_mut().remove_value(child);
                self.node_mut(child)?.set_parent(None);
                Ok(())
            }
        }
    }

    fn place(
        &mut self,
        parent: NodeId,
        child: NodeId,
        position: Attachment,
    ) -> Result<(), DomError> {
        match position {
            Attachment::End => {
                self.node_mut(parent)?.children_mut().push(child);
                Ok(())
            }
            Attachment::Before(anchor) => self
                .node_mut(parent)?
                .children_mut()
                .insert_before_value(anchor, child)
                .ok_or(DomError::NodeNotFound(anchor)),
        }
    }

    fn collect_subtree(&self, root: NodeId) -> Result<Vec<NodeId>, DomError> {
        self.node(root)?;
        let mut collected = vec![root];
        let mut cursor = 0;
        while cursor < collected.len() {
            let Some(&parent) = collected.get(cursor) else {
                break;
            };
            collected.extend(self.node(parent)?.children().iter());
            cursor = cursor.saturating_add(1);
        }
        Ok(collected)
    }

    fn tombstone(&mut self, node: NodeId) {
        if let Some(slot) = self.slots.get_mut(node.index()) {
            *slot = Slot::Tombstone;
        }
    }
}

impl Default for DomTree {
    fn default() -> Self {
        Self::new()
    }
}
