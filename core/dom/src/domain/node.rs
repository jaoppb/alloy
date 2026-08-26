use crate::domain::children::Children;
use crate::domain::node_data::NodeData;
use crate::domain::node_id::NodeId;

/// Entity representing a single node in the DOM tree hierarchy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomNode {
    id: NodeId,
    parent: Option<NodeId>,
    children: Children,
    data: NodeData,
}

impl DomNode {
    /// Creates a new unattached `DomNode`.
    #[must_use]
    pub const fn new(id: NodeId, data: NodeData) -> Self {
        Self {
            id,
            parent: None,
            children: Children::new(),
            data,
        }
    }

    /// Returns the unique identity of this node.
    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.id
    }

    /// Returns the parent `NodeId`, if any.
    #[must_use]
    pub const fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Returns the children collection.
    #[must_use]
    pub const fn children(&self) -> &Children {
        &self.children
    }

    /// Returns the node data payload.
    #[must_use]
    pub const fn data(&self) -> &NodeData {
        &self.data
    }

    /// Returns a mutable reference to the node data payload.
    pub fn data_mut(&mut self) -> &mut NodeData {
        &mut self.data
    }

    /// Sets or clears the parent node pointer.
    pub(crate) fn set_parent(&mut self, parent: Option<NodeId>) {
        self.parent = parent;
    }

    /// Appends a child to this node's children list.
    pub(crate) fn add_child(&mut self, child: NodeId) {
        self.children.push(child);
    }

    /// Inserts a child at the given position.
    pub(crate) fn insert_child(&mut self, index: usize, child: NodeId) {
        self.children.insert(index, child);
    }

    /// Removes a child from this node's children list.
    pub(crate) fn remove_child(&mut self, child: NodeId) -> bool {
        self.children.remove(child)
    }
}
