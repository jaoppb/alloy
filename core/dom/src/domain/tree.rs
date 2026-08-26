use crate::domain::attribute::AttributeMap;
use crate::domain::error::DomError;
use crate::domain::node::DomNode;
use crate::domain::node_data::NodeData;
use crate::domain::node_id::NodeId;
use crate::domain::tag_name::TagName;

/// Aggregate root managing DOM nodes within an indexed vector arena.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DomTree {
    nodes: Vec<Option<DomNode>>,
    root: Option<NodeId>,
}

impl DomTree {
    /// Creates an empty DOM tree arena.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
        }
    }

    /// Returns the root node identifier, if set.
    #[must_use]
    pub const fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Sets the root node of this tree.
    ///
    /// # Errors
    /// Returns `DomError::NodeNotFound` if the node ID does not exist in the arena.
    pub fn set_root(&mut self, root: NodeId) -> Result<(), DomError> {
        if self.get(root).is_none() {
            return Err(DomError::NodeNotFound(root));
        }

        self.root = Some(root);
        Ok(())
    }

    /// Allocates an element node in the arena.
    pub fn create_element(&mut self, tag: TagName, attributes: AttributeMap) -> NodeId {
        self.allocate_node(NodeData::Element {
            tag_name: tag,
            attributes,
        })
    }

    /// Allocates a text leaf node in the arena.
    pub fn create_text(&mut self, text: impl Into<String>) -> NodeId {
        self.allocate_node(NodeData::Text(text.into()))
    }

    /// Allocates a root document node in the arena.
    pub fn create_document(&mut self) -> NodeId {
        let id = self.allocate_node(NodeData::Document);
        if self.root.is_none() {
            self.root = Some(id);
        }
        id
    }

    /// Allocates a comment node in the arena.
    pub fn create_comment(&mut self, text: impl Into<String>) -> NodeId {
        self.allocate_node(NodeData::Comment(text.into()))
    }

    /// Looks up an immutable reference to a node by its `NodeId`.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&DomNode> {
        self.nodes.get(id.as_usize())?.as_ref()
    }

    /// Looks up a mutable reference to a node by its `NodeId`.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut DomNode> {
        self.nodes.get_mut(id.as_usize())?.as_mut()
    }

    /// Returns the total number of nodes in the arena.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.iter().flatten().count()
    }

    /// Checks whether `node` is a descendant of `potential_ancestor`.
    #[must_use]
    pub fn is_descendant_of(&self, node: NodeId, potential_ancestor: NodeId) -> bool {
        let mut current = self.get(node).and_then(DomNode::parent);
        while let Some(parent_id) = current {
            if parent_id == potential_ancestor {
                return true;
            }
            current = self.get(parent_id).and_then(DomNode::parent);
        }
        false
    }

    /// Appends `child` as the last child of `parent`, enforcing acyclicity and single-parent linkage.
    ///
    /// # Errors
    /// - `DomError::NodeNotFound`: If `parent` or `child` does not exist.
    /// - `DomError::CycleDetected`: If appending would make an ancestor a child of its descendant.
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), DomError> {
        self.validate_exists(parent)?;
        self.validate_exists(child)?;

        if child == parent || self.is_descendant_of(parent, child) {
            return Err(DomError::CycleDetected {
                node: child,
                parent,
            });
        }

        self.detach_from_parent(child);

        if let Some(parent_node) = self.get_mut(parent) {
            parent_node.add_child(child);
        }
        if let Some(child_node) = self.get_mut(child) {
            child_node.set_parent(Some(parent));
        }

        Ok(())
    }

    /// Inserts `new_child` into `parent`'s children sequence right before `reference_child`.
    ///
    /// # Errors
    /// - `DomError::NodeNotFound`: If any referenced node does not exist.
    /// - `DomError::InvalidHierarchy`: If `reference_child` is not a direct child of `parent`.
    /// - `DomError::CycleDetected`: If insertion would create a cyclical hierarchy.
    pub fn insert_before(
        &mut self,
        parent: NodeId,
        new_child: NodeId,
        reference_child: NodeId,
    ) -> Result<(), DomError> {
        self.validate_exists(parent)?;
        self.validate_exists(new_child)?;
        self.validate_exists(reference_child)?;

        let insert_idx = {
            let parent_node = self.get(parent).ok_or(DomError::NodeNotFound(parent))?;
            parent_node
                .children()
                .position(reference_child)
                .ok_or_else(|| {
                    DomError::InvalidHierarchy(format!(
                        "Reference child {reference_child} is not a child of parent {parent}"
                    ))
                })?
        };

        if new_child == parent || self.is_descendant_of(parent, new_child) {
            return Err(DomError::CycleDetected {
                node: new_child,
                parent,
            });
        }

        self.detach_from_parent(new_child);

        if let Some(parent_node) = self.get_mut(parent) {
            parent_node.insert_child(insert_idx, new_child);
        }
        if let Some(child_node) = self.get_mut(new_child) {
            child_node.set_parent(Some(parent));
        }

        Ok(())
    }

    /// Removes `child` from `parent`'s children list.
    ///
    /// # Errors
    /// - `DomError::NodeNotFound`: If `parent` or `child` does not exist.
    /// - `DomError::InvalidHierarchy`: If `child` is not a direct child of `parent`.
    pub fn remove_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), DomError> {
        self.validate_exists(parent)?;
        self.validate_exists(child)?;

        let was_removed = {
            let parent_node = self.get_mut(parent).ok_or(DomError::NodeNotFound(parent))?;
            parent_node.remove_child(child)
        };

        if !was_removed {
            return Err(DomError::InvalidHierarchy(format!(
                "Node {child} is not a child of {parent}"
            )));
        }

        if let Some(child_node) = self.get_mut(child) {
            child_node.set_parent(None);
        }

        Ok(())
    }

    /// Performs pre-order depth-first traversal starting from `start`.
    #[must_use]
    pub fn traverse_pre_order(&self, start: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut stack = vec![start];

        while let Some(current) = stack.pop() {
            if let Some(node) = self.get(current) {
                result.push(current);
                // Push children in reverse order so leftmost child is processed first
                for &child_id in node.children().as_slice().iter().rev() {
                    stack.push(child_id);
                }
            }
        }

        result
    }

    fn allocate_node(&mut self, data: NodeData) -> NodeId {
        let index = self.nodes.len() as u32;
        let id = NodeId::new(index);
        let node = DomNode::new(id, data);
        self.nodes.push(Some(node));
        id
    }

    fn validate_exists(&self, id: NodeId) -> Result<(), DomError> {
        if self.get(id).is_some() {
            return Ok(());
        }
        Err(DomError::NodeNotFound(id))
    }

    fn detach_from_parent(&mut self, child: NodeId) {
        let old_parent = self.get(child).and_then(DomNode::parent);
        if let Some(parent_id) = old_parent {
            if let Some(parent_node) = self.get_mut(parent_id) {
                parent_node.remove_child(child);
            }
            if let Some(child_node) = self.get_mut(child) {
                child_node.set_parent(None);
            }
        }
    }

    /// Serializes a node subtree to a compact HTML string (C-24).
    #[must_use]
    pub fn serialize_to_html(&self, root: NodeId) -> String {
        crate::domain::service::DomService::serialize_to_html(self, root)
    }
}
