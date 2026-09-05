//! Mock adapter implementing [`TreeSink`] for tests and conformance validation (ADR-0011).

use crate::application::ports::TreeSink;
use crate::domain::error::HtmlError;
use crate::domain::token::AttributeList;

/// A recorded operation in the [`MockTreeSink`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MockEvent {
    /// Created element node.
    CreateElement {
        /// Target node id.
        id: dom::NodeId,
        /// Element tag name.
        tag: String,
        /// Number of attributes attached.
        attr_count: usize,
    },
    /// Created text node.
    CreateText {
        /// Target node id.
        id: dom::NodeId,
        /// Content snippet.
        content: String,
    },
    /// Created comment node.
    CreateComment {
        /// Target node id.
        id: dom::NodeId,
        /// Comment snippet.
        content: String,
    },
    /// Appended child to parent.
    AppendChild {
        /// Parent node id.
        parent: dom::NodeId,
        /// Child node id.
        child: dom::NodeId,
    },
}

/// A mock tree sink that logs construction events without allocating a real DOM tree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MockTreeSink {
    arena: dom::DomTree,
    events: Vec<MockEvent>,
}

impl MockTreeSink {
    /// Create a new mock tree sink.
    #[must_use]
    pub fn new() -> Self {
        Self {
            arena: dom::DomTree::new(),
            events: Vec::new(),
        }
    }

    /// Access the recorded events.
    #[must_use]
    pub fn events(&self) -> &[MockEvent] {
        &self.events
    }

    fn allocate_id(&mut self) -> dom::NodeId {
        self.arena.create_element(dom::TagName::Div)
    }
}

impl TreeSink for MockTreeSink {
    fn create_element(
        &mut self,
        tag: &str,
        attributes: &AttributeList,
    ) -> Result<dom::NodeId, HtmlError> {
        let id = self.allocate_id();
        self.events.push(MockEvent::CreateElement {
            id,
            tag: tag.to_string(),
            attr_count: attributes.len(),
        });
        Ok(id)
    }

    fn create_text(&mut self, text: &str) -> Result<dom::NodeId, HtmlError> {
        let id = self.allocate_id();
        self.events.push(MockEvent::CreateText {
            id,
            content: text.to_string(),
        });
        Ok(id)
    }

    fn create_comment(&mut self, text: &str) -> Result<dom::NodeId, HtmlError> {
        let id = self.allocate_id();
        self.events.push(MockEvent::CreateComment {
            id,
            content: text.to_string(),
        });
        Ok(id)
    }

    fn append_child(&mut self, parent: dom::NodeId, child: dom::NodeId) -> Result<(), HtmlError> {
        self.events.push(MockEvent::AppendChild { parent, child });
        Ok(())
    }

    fn root_node(&self) -> dom::NodeId {
        dom::NodeId::root()
    }
}
