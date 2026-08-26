use crate::domain::node_id::NodeId;

/// First-class collection maintaining the ordered sequence of child nodes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Children {
    nodes: Vec<NodeId>,
}

impl Children {
    /// Creates an empty sequence of children.
    #[must_use]
    pub const fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Appends a child node to the end of the children list.
    pub fn push(&mut self, child: NodeId) {
        self.nodes.push(child);
    }

    /// Inserts a child node at the given index.
    pub fn insert(&mut self, index: usize, child: NodeId) {
        self.nodes.insert(index, child);
    }

    /// Removes a child node from the list, returning true if it was found and removed.
    pub fn remove(&mut self, child: NodeId) -> bool {
        if let Some(pos) = self.nodes.iter().position(|&id| id == child) {
            self.nodes.remove(pos);
            return true;
        }
        false
    }

    /// Finds the index of a child node.
    #[must_use]
    pub fn position(&self, child: NodeId) -> Option<usize> {
        self.nodes.iter().position(|&id| id == child)
    }

    /// Checks if a child node exists in this list.
    #[must_use]
    pub fn contains(&self, child: NodeId) -> bool {
        self.nodes.contains(&child)
    }

    /// Returns a slice of the child `NodeId`s.
    #[must_use]
    pub fn as_slice(&self) -> &[NodeId] {
        &self.nodes
    }

    /// Returns the number of child nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Checks if the children list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns an iterator over the child `NodeId`s.
    pub fn iter(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().copied()
    }
}
