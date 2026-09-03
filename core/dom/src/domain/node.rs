//! Node identity and payload.
//!
//! [`NodeId`], the [`NodeKind`] payloads, and the arena slot. `NodeData` and
//! `Slot` are crate-internal — the outside world only ever names a [`NodeId`]
//! and reads through [`crate::DomTree`] (Object Calisthenics rule 8,
//! `ADR-0010:136`).

use core::fmt;

use crate::domain::attributes::AttributeMap;
use crate::domain::tag_name::TagName;
use crate::domain::text::{CommentContent, TextContent};

/// A handle to a node inside one [`crate::DomTree`].
///
/// `Copy`; the raw `u32` is the arena index (v0.2 report §2.2; `ADR-0010:131`
/// and `CLAUDE.md` write this newtype verbatim). A `NodeId` into a removed node
/// stays valid syntactically but resolves to [`crate::DomError::NodeNotFound`] —
/// v0.2 keeps no generational tag (deferred to C-13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u32);

impl NodeId {
    /// The root `Document` node id (`node #0`).
    #[must_use]
    pub const fn root() -> Self {
        Self(0)
    }

    pub(crate) fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).unwrap_or(u32::MAX))
    }

    /// The arena index this id addresses.
    #[must_use]
    pub fn index(self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "node #{}", self.0)
    }
}

/// An element's own data: its tag and its insertion-ordered attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementData {
    tag: TagName,
    attributes: AttributeMap,
}

impl ElementData {
    pub(crate) const fn new(tag: TagName) -> Self {
        Self {
            tag,
            attributes: AttributeMap::new(),
        }
    }

    #[must_use]
    pub const fn tag(&self) -> &TagName {
        &self.tag
    }

    #[must_use]
    pub const fn attributes(&self) -> &AttributeMap {
        &self.attributes
    }

    pub(crate) const fn attributes_mut(&mut self) -> &mut AttributeMap {
        &mut self.attributes
    }
}

/// What a node *is*. The four kinds Alloy's v0.2 tree recognises (report §2.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// The single, irremovable root.
    Document,
    Element(ElementData),
    Text(TextContent),
    Comment(CommentContent),
}

/// The payload of an occupied arena slot: kind + structural links. Every field
/// is reached through [`crate::DomTree`] methods, never mutated from outside.
///
/// Implemented as an intrusive doubly-linked tree with 5 pointers (NodeId/Option<NodeId>),
/// eliminating dynamic Vec allocations for child tracking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NodeData {
    kind: NodeKind,
    parent: Option<NodeId>,
    first_child: Option<NodeId>,
    last_child: Option<NodeId>,
    previous_sibling: Option<NodeId>,
    next_sibling: Option<NodeId>,
}

impl NodeData {
    pub(crate) const fn new(kind: NodeKind) -> Self {
        Self {
            kind,
            parent: None,
            first_child: None,
            last_child: None,
            previous_sibling: None,
            next_sibling: None,
        }
    }

    pub(crate) const fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub(crate) const fn kind_mut(&mut self) -> &mut NodeKind {
        &mut self.kind
    }

    pub(crate) const fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    pub(crate) const fn set_parent(&mut self, parent: Option<NodeId>) {
        self.parent = parent;
    }

    pub(crate) const fn first_child(&self) -> Option<NodeId> {
        self.first_child
    }

    pub(crate) const fn set_first_child(&mut self, first_child: Option<NodeId>) {
        self.first_child = first_child;
    }

    pub(crate) const fn last_child(&self) -> Option<NodeId> {
        self.last_child
    }

    pub(crate) const fn set_last_child(&mut self, last_child: Option<NodeId>) {
        self.last_child = last_child;
    }

    pub(crate) const fn previous_sibling(&self) -> Option<NodeId> {
        self.previous_sibling
    }

    pub(crate) const fn set_previous_sibling(&mut self, previous_sibling: Option<NodeId>) {
        self.previous_sibling = previous_sibling;
    }

    pub(crate) const fn next_sibling(&self) -> Option<NodeId> {
        self.next_sibling
    }

    pub(crate) const fn set_next_sibling(&mut self, next_sibling: Option<NodeId>) {
        self.next_sibling = next_sibling;
    }
}

/// One cell of the arena. A removed node's cell becomes [`Slot::Tombstone`] and
/// its index is never reissued in v0.2 (report §2.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    Occupied(NodeData),
    Tombstone,
}
