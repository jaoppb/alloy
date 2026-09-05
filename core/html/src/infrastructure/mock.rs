//! Mock adapter implementing [`TreeSink`] for tests and conformance validation (ADR-0011).

use crate::application::ports::TreeSink;
use crate::domain::attribute::AttributeList;
use crate::domain::error::HtmlError;
use crate::domain::handle::NodeHandle;
use crate::domain::tag_name::TagName;
use crate::domain::text::Text;

/// A recorded operation in the [`MockTreeSink`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MockEvent {
    /// Created element node.
    CreateElement {
        /// Target node handle.
        id: NodeHandle,
        /// Element tag name.
        tag: String,
        /// Number of attributes attached.
        attribute_count: usize,
    },
    /// Created text node.
    CreateText {
        /// Target node handle.
        id: NodeHandle,
        /// Content snippet.
        content: String,
    },
    /// Created comment node.
    CreateComment {
        /// Target node handle.
        id: NodeHandle,
        /// Comment snippet.
        content: String,
    },
    /// Appended child to parent.
    AppendChild {
        /// Parent node handle.
        parent: NodeHandle,
        /// Child node handle.
        child: NodeHandle,
    },
    /// Appended before sibling.
    AppendBeforeSibling {
        /// Sibling node handle.
        sibling: NodeHandle,
        /// Child node handle.
        child: NodeHandle,
    },
    /// Added attributes if missing.
    AddAttributes {
        /// Target node handle.
        target: NodeHandle,
        /// Number of attributes added.
        attribute_count: usize,
    },
    /// Removed node from parent.
    RemoveFromParent {
        /// Target node handle.
        target: NodeHandle,
    },
    /// Reparented children.
    ReparentChildren {
        /// Origin node handle.
        from: NodeHandle,
        /// Destination node handle.
        to: NodeHandle,
    },
}

/// A node stored in the mock tree.
#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredNode {
    handle: NodeHandle,
    parent: Option<NodeHandle>,
    children: Vec<NodeHandle>,
}

/// A mock tree sink that logs construction events without allocating a real DOM tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockTreeSink {
    next_identifier: u32,
    nodes: Vec<StoredNode>,
    events: Vec<MockEvent>,
}

impl MockTreeSink {
    /// Create a new mock tree sink with root node initialized.
    #[must_use]
    pub fn new() -> Self {
        let root = StoredNode {
            handle: NodeHandle::root(),
            parent: None,
            children: Vec::new(),
        };
        Self {
            next_identifier: 1,
            nodes: vec![root],
            events: Vec::new(),
        }
    }

    /// Access the recorded events.
    #[must_use]
    pub fn events(&self) -> &[MockEvent] {
        &self.events
    }

    fn allocate_handle(&mut self) -> NodeHandle {
        let handle = NodeHandle::new(self.next_identifier);
        self.next_identifier = self.next_identifier.saturating_add(1);
        self.nodes.push(StoredNode {
            handle,
            parent: None,
            children: Vec::new(),
        });
        handle
    }
}

impl Default for MockTreeSink {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeSink for MockTreeSink {
    fn create_element(
        &mut self,
        tag: TagName,
        attributes: &AttributeList,
    ) -> Result<NodeHandle, HtmlError> {
        let handle = self.allocate_handle();
        self.events.push(MockEvent::CreateElement {
            id: handle,
            tag: tag.into_string(),
            attribute_count: attributes.len(),
        });
        Ok(handle)
    }

    fn create_text(&mut self, text: &Text) -> Result<NodeHandle, HtmlError> {
        let handle = self.allocate_handle();
        self.events.push(MockEvent::CreateText {
            id: handle,
            content: text.as_str().to_string(),
        });
        Ok(handle)
    }

    fn create_comment(&mut self, text: &Text) -> Result<NodeHandle, HtmlError> {
        let handle = self.allocate_handle();
        self.events.push(MockEvent::CreateComment {
            id: handle,
            content: text.as_str().to_string(),
        });
        Ok(handle)
    }

    fn append_child(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), HtmlError> {
        if let Some(parent_node) = self.nodes.iter_mut().find(|n| n.handle == parent) {
            parent_node.children.push(child);
        }
        if let Some(child_node) = self.nodes.iter_mut().find(|n| n.handle == child) {
            child_node.parent = Some(parent);
        }
        self.events.push(MockEvent::AppendChild { parent, child });
        Ok(())
    }

    fn append_before_sibling(
        &mut self,
        sibling: NodeHandle,
        child: NodeHandle,
    ) -> Result<(), HtmlError> {
        let parent_handle = self
            .nodes
            .iter()
            .find(|n| n.handle == sibling)
            .and_then(|n| n.parent);

        let Some(parent_node) = self
            .nodes
            .iter_mut()
            .find(|node| parent_handle == Some(node.handle))
        else {
            self.events
                .push(MockEvent::AppendBeforeSibling { sibling, child });
            return Ok(());
        };

        if let Some(index) = parent_node.children.iter().position(|&c| c == sibling) {
            parent_node.children.insert(index, child);
        }
        self.events
            .push(MockEvent::AppendBeforeSibling { sibling, child });
        Ok(())
    }

    fn add_attributes_if_missing(
        &mut self,
        target: NodeHandle,
        attributes: &AttributeList,
    ) -> Result<(), HtmlError> {
        self.events.push(MockEvent::AddAttributes {
            target,
            attribute_count: attributes.len(),
        });
        Ok(())
    }

    fn remove_from_parent(&mut self, target: NodeHandle) -> Result<(), HtmlError> {
        for node in &mut self.nodes {
            node.children.retain(|&c| c != target);
        }
        self.events.push(MockEvent::RemoveFromParent { target });
        Ok(())
    }

    fn reparent_children(&mut self, from: NodeHandle, to: NodeHandle) -> Result<(), HtmlError> {
        let children_to_move: Vec<NodeHandle> = self
            .nodes
            .iter()
            .find(|n| n.handle == from)
            .map_or_else(Vec::new, |n| n.children.clone());

        if let Some(from_node) = self.nodes.iter_mut().find(|n| n.handle == from) {
            from_node.children.clear();
        }
        if let Some(to_node) = self.nodes.iter_mut().find(|n| n.handle == to) {
            to_node.children.extend(children_to_move);
        }
        self.events.push(MockEvent::ReparentChildren { from, to });
        Ok(())
    }

    fn root_node(&self) -> NodeHandle {
        NodeHandle::root()
    }
}
