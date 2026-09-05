//! Default adapter implementing [`TreeSink`] over [`dom::DomTree`].

use crate::application::ports::TreeSink;
use crate::domain::error::HtmlError;
use crate::domain::token::AttributeList;

/// An adapter that builds a real [`dom::DomTree`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomTreeSink {
    tree: dom::DomTree,
}

impl DomTreeSink {
    /// Create a new sink wrapping a fresh [`dom::DomTree`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            tree: dom::DomTree::new(),
        }
    }

    /// Unwrap and return the built DOM tree.
    #[must_use]
    pub fn into_tree(self) -> dom::DomTree {
        self.tree
    }

    /// Access the underlying DOM tree.
    #[must_use]
    pub const fn tree(&self) -> &dom::DomTree {
        &self.tree
    }
}

impl Default for DomTreeSink {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeSink for DomTreeSink {
    fn create_element(
        &mut self,
        tag: &str,
        attributes: &AttributeList,
    ) -> Result<dom::NodeId, HtmlError> {
        let tag_name = dom::TagName::new(tag)?;
        let node = self.tree.create_element(tag_name);

        for entry in attributes {
            let attr_name = dom::AttributeName::new(entry.name())?;
            let attr_val = dom::AttributeValue::new(entry.value());
            self.tree.set_attribute(node, attr_name, attr_val)?;
        }

        Ok(node)
    }

    fn create_text(&mut self, text: &str) -> Result<dom::NodeId, HtmlError> {
        let content = dom::TextContent::new(text);
        Ok(self.tree.create_text(content))
    }

    fn create_comment(&mut self, text: &str) -> Result<dom::NodeId, HtmlError> {
        let content = dom::CommentContent::new(text);
        Ok(self.tree.create_comment(content))
    }

    fn append_child(&mut self, parent: dom::NodeId, child: dom::NodeId) -> Result<(), HtmlError> {
        self.tree.append_child(parent, child)?;
        Ok(())
    }

    fn root_node(&self) -> dom::NodeId {
        self.tree.document()
    }
}
