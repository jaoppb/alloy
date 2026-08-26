use crate::domain::attribute::AttributeMap;
use crate::domain::error::DomError;
use crate::domain::node::DomNode;
use crate::domain::node_data::NodeData;
use crate::domain::node_id::NodeId;
use crate::domain::slot::Slot;
use crate::domain::tag_name::TagName;

/// Aggregate root managing DOM nodes within a generational slot arena (ADR-0010, ADR-0013, C-27).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DomTree {
    slots: Vec<Slot<DomNode>>,
    free_head: Option<u32>,
    root: Option<NodeId>,
}

impl DomTree {
    /// Creates an empty DOM tree arena.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_head: None,
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
        self.resolve_all(&[root])?;
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

    /// Looks up an immutable reference to a node by its generational `NodeId`.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&DomNode> {
        match self.slots.get(id.as_usize())? {
            Slot::Occupied { data, generation } if *generation == id.generation() => Some(data),
            _ => None,
        }
    }

    /// Looks up a mutable reference to a node by its generational `NodeId`.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut DomNode> {
        match self.slots.get_mut(id.as_usize())? {
            Slot::Occupied { data, generation } if *generation == id.generation() => Some(data),
            _ => None,
        }
    }

    /// Validates that all node identifiers exist and match active generations (C-26).
    ///
    /// # Errors
    /// Returns `DomError::NodeNotFound` on the first missing or stale identifier.
    pub fn resolve_all(&self, ids: &[NodeId]) -> Result<(), DomError> {
        for &id in ids {
            if self.get(id).is_none() {
                return Err(DomError::NodeNotFound(id));
            }
        }
        Ok(())
    }

    /// Returns the total number of active nodes in the arena.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_occupied()).count()
    }

    /// Accesses the underlying generational slots.
    #[must_use]
    pub fn slots(&self) -> &[Slot<DomNode>] {
        &self.slots
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
        self.resolve_all(&[parent, child])?;

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
        self.resolve_all(&[parent, new_child, reference_child])?;

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
        self.resolve_all(&[parent, child])?;

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

    /// Removes a node from the arena, recycling its slot with an incremented generation (ADR-0013, C-27).
    ///
    /// # Errors
    /// - `DomError::NodeNotFound`: If `id` does not exist or has already been recycled.
    pub fn remove_node(&mut self, id: NodeId) -> Result<(), DomError> {
        self.resolve_all(&[id])?;
        self.detach_from_parent(id);

        if self.root == Some(id) {
            self.root = None;
        }

        let idx = id.as_usize();
        if let Some(Slot::Occupied { generation, .. }) = self.slots.get(idx) {
            let next_gen = generation.wrapping_add(1);
            let prev_free = self.free_head;
            self.slots[idx] = Slot::Vacant {
                next_free: prev_free,
                generation: next_gen,
            };
            self.free_head = Some(id.index());
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
        if let Some(free_idx) = self.free_head {
            let slot = &mut self.slots[free_idx as usize];
            let slot_gen = match slot {
                Slot::Vacant {
                    next_free,
                    generation,
                } => {
                    self.free_head = *next_free;
                    *generation
                }
                Slot::Occupied { .. } => unreachable!("corrupt free list in DomTree"),
            };
            let id = NodeId::with_generation(free_idx, slot_gen);
            let node = DomNode::new(id, data);
            self.slots[free_idx as usize] = Slot::Occupied {
                data: node,
                generation: slot_gen,
            };
            id
        } else {
            let index = self.slots.len() as u32;
            let id = NodeId::with_generation(index, 0);
            let node = DomNode::new(id, data);
            self.slots.push(Slot::Occupied {
                data: node,
                generation: 0,
            });
            id
        }
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
