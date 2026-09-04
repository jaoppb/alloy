//! [`DomSnapshot`] — an immutable, read-only projection of a `dom::DomTree`
//! (`PRD-007:35-36`).
//!
//! Elements, attributes and tree shape, with **no `core/dom` internal type in
//! the public API**: a node is addressed by an opaque [`SnapshotId`], a tag is
//! a `&str`, an attribute is a `(&str, &str)` pair. The only way to build one
//! is [`crate::snapshot`], the explicit mapping function of `PRD-007:36`.

use core::fmt;

/// An opaque handle to a node inside one [`DomSnapshot`].
///
/// A projection-local index, unrelated to `dom::NodeId`. `snapshot()` assigns
/// these in pre-order, so `0..len` is document order and every parent's id is
/// smaller than its children's — which is what lets the cascade run in a
/// single forward pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SnapshotId(u32);

impl SnapshotId {
    pub(crate) fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).unwrap_or(u32::MAX))
    }

    /// The projection index this id addresses.
    #[must_use]
    pub fn index(self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX)
    }
}

impl fmt::Display for SnapshotId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "snapshot node #{}", self.0)
    }
}

/// What a projected node is. No payload — a tag comes from [`NodeRef::tag`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SnapshotNodeKind {
    /// The document root.
    Document,
    /// An element.
    Element,
    /// A text node.
    Text,
    /// A comment node.
    Comment,
}

/// The child ids of one node, in document order. A first-class collection
/// (`ADR-0010` rule 3) — no public `Vec`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChildIds {
    ids: Vec<SnapshotId>,
}

impl ChildIds {
    #[must_use]
    pub const fn new() -> Self {
        Self { ids: Vec::new() }
    }

    /// Collects a run of ids in the order given. Crate-internal — the snapshot
    /// and the styled tree are the only producers.
    pub(crate) fn from_ids(ids: impl IntoIterator<Item = SnapshotId>) -> Self {
        Self {
            ids: ids.into_iter().collect(),
        }
    }

    fn push(&mut self, id: SnapshotId) {
        self.ids.push(id);
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.ids.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = SnapshotId> + '_ {
        self.ids.iter().copied()
    }
}

impl<'ids> IntoIterator for &'ids ChildIds {
    type Item = SnapshotId;
    type IntoIter = core::iter::Copied<core::slice::Iter<'ids, SnapshotId>>;

    fn into_iter(self) -> Self::IntoIter {
        self.ids.iter().copied()
    }
}

/// One element's attributes, in source order. A first-class collection — no
/// public `Vec`, and lookup is by `&str` so `core/dom`'s `AttributeName` never
/// leaks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttributeList {
    entries: Vec<(String, String)>,
}

impl AttributeList {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Builds the list from `(name, value)` pairs already in source order.
    /// Crate-internal — the projection is the only producer.
    pub(crate) fn from_pairs(pairs: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            entries: pairs.into_iter().collect(),
        }
    }

    /// The value of `name`, or `None`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.entries
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// An element's own facts: its lowercased tag and its attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ElementFacts {
    tag: String,
    attributes: AttributeList,
}

/// One projected node.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SnapshotNode {
    kind: SnapshotNodeKind,
    parent: Option<SnapshotId>,
    children: ChildIds,
    element: Option<ElementFacts>,
    text: Option<String>,
}

/// An immutable projection of a document subtree.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DomSnapshot {
    nodes: Vec<SnapshotNode>,
    root: SnapshotId,
}

impl DomSnapshot {
    /// The id of the projected root.
    #[must_use]
    pub const fn root(&self) -> SnapshotId {
        self.root
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// A borrowed view of the node at `id`, or `None` when `id` is foreign.
    #[must_use]
    pub fn node(&self, id: SnapshotId) -> Option<NodeRef<'_>> {
        self.raw(id).map(|_| NodeRef { snapshot: self, id })
    }

    /// Every id in document order — `0..len`, which pre-order construction
    /// guarantees is also parent-before-child order.
    pub fn nodes_in_document_order(&self) -> impl Iterator<Item = SnapshotId> + '_ {
        (0..self.nodes.len()).map(SnapshotId::from_index)
    }

    fn raw(&self, id: SnapshotId) -> Option<&SnapshotNode> {
        self.nodes.get(id.index())
    }
}

/// A borrowed, read-only view of one projected node.
#[derive(Clone, Copy)]
pub struct NodeRef<'snapshot> {
    snapshot: &'snapshot DomSnapshot,
    id: SnapshotId,
}

impl<'snapshot> NodeRef<'snapshot> {
    #[must_use]
    pub const fn id(self) -> SnapshotId {
        self.id
    }

    #[must_use]
    pub fn kind(self) -> SnapshotNodeKind {
        self.node()
            .map_or(SnapshotNodeKind::Document, |node| node.kind)
    }

    /// The lowercased tag name, or `None` for a non-element.
    #[must_use]
    pub fn tag(self) -> Option<&'snapshot str> {
        self.element().map(|element| element.tag.as_str())
    }

    /// The value of attribute `name`, or `None`.
    #[must_use]
    pub fn attribute(self, name: &str) -> Option<&'snapshot str> {
        self.element()
            .and_then(|element| element.attributes.get(name))
    }

    /// Every attribute of this node in source order.
    pub fn attributes(self) -> impl Iterator<Item = (&'snapshot str, &'snapshot str)> + 'snapshot {
        self.element()
            .into_iter()
            .flat_map(|element| element.attributes.iter())
    }

    /// The character data of a `Text` or `Comment` node.
    #[must_use]
    pub fn text(self) -> Option<&'snapshot str> {
        self.node().and_then(|node| node.text.as_deref())
    }

    /// The parent id, or `None` for the projected root.
    #[must_use]
    pub fn parent(self) -> Option<SnapshotId> {
        self.node().and_then(|node| node.parent)
    }

    /// The direct children in document order.
    pub fn children(self) -> impl Iterator<Item = SnapshotId> + 'snapshot {
        self.node()
            .into_iter()
            .flat_map(|node| node.children.iter())
    }

    fn node(self) -> Option<&'snapshot SnapshotNode> {
        self.snapshot.raw(self.id)
    }

    fn element(self) -> Option<&'snapshot ElementFacts> {
        self.node().and_then(|node| node.element.as_ref())
    }
}

/// Accumulates projected nodes during [`crate::snapshot`]. Crate-internal:
/// outside callers reach a [`DomSnapshot`] only through the mapping function.
pub(crate) struct SnapshotBuilder {
    nodes: Vec<SnapshotNode>,
}

impl SnapshotBuilder {
    pub(crate) const fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Appends a node, links it into its parent's child list, and returns its
    /// fresh id.
    pub(crate) fn add_element(
        &mut self,
        parent: Option<SnapshotId>,
        tag: String,
        attributes: AttributeList,
    ) -> SnapshotId {
        let facts = ElementFacts { tag, attributes };
        self.add(SnapshotNodeKind::Element, parent, Some(facts), None)
    }

    pub(crate) fn add_character_data(
        &mut self,
        kind: SnapshotNodeKind,
        parent: Option<SnapshotId>,
        text: String,
    ) -> SnapshotId {
        self.add(kind, parent, None, Some(text))
    }

    pub(crate) fn add_document(&mut self, parent: Option<SnapshotId>) -> SnapshotId {
        self.add(SnapshotNodeKind::Document, parent, None, None)
    }

    fn add(
        &mut self,
        kind: SnapshotNodeKind,
        parent: Option<SnapshotId>,
        element: Option<ElementFacts>,
        text: Option<String>,
    ) -> SnapshotId {
        let id = SnapshotId::from_index(self.nodes.len());
        self.nodes.push(SnapshotNode {
            kind,
            parent,
            children: ChildIds::new(),
            element,
            text,
        });
        Self::link_child(&mut self.nodes, parent, id);
        id
    }

    fn link_child(nodes: &mut [SnapshotNode], parent: Option<SnapshotId>, child: SnapshotId) {
        if let Some(parent_id) = parent {
            Self::push_child(nodes, parent_id, child);
        }
    }

    fn push_child(nodes: &mut [SnapshotNode], parent_id: SnapshotId, child: SnapshotId) {
        if let Some(parent_node) = nodes.get_mut(parent_id.index()) {
            parent_node.children.push(child);
        }
    }

    pub(crate) fn finish(self, root: SnapshotId) -> DomSnapshot {
        DomSnapshot {
            nodes: self.nodes,
            root,
        }
    }
}
