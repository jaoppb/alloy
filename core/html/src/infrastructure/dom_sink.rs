//! Default adapter implementing [`TreeSink`] over [`dom::DomTree`].

#![cfg(feature = "dom")]

use crate::application::ports::TreeSink;
use crate::domain::attribute::AttributeList;
use crate::domain::error::HtmlError;
use crate::domain::handle::NodeHandle;
use crate::domain::tag_name::TagName;
use crate::domain::text::Text;

/// An adapter that builds a real [`dom::DomTree`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomTreeSink {
    tree: dom::DomTree,
    handles: Vec<dom::NodeId>,
}

impl DomTreeSink {
    /// Create a new sink wrapping a fresh [`dom::DomTree`].
    #[must_use]
    pub fn new() -> Self {
        let tree = dom::DomTree::new();
        let document = tree.document();
        Self {
            tree,
            handles: vec![document],
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

    fn register_node(&mut self, node: dom::NodeId) -> NodeHandle {
        let index = u32::try_from(self.handles.len()).unwrap_or(0);
        self.handles.push(node);
        NodeHandle::new(index)
    }

    fn resolve_node(&self, handle: NodeHandle) -> Result<dom::NodeId, HtmlError> {
        let index = usize::try_from(handle.index()).unwrap_or(usize::MAX);
        self.handles
            .get(index)
            .copied()
            .ok_or_else(|| HtmlError::tree_construction(format!("unknown node handle {handle}")))
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
        tag: TagName,
        attributes: &AttributeList,
    ) -> Result<NodeHandle, HtmlError> {
        let dom_tag = dom::TagName::new(tag.as_str())?;
        let node = self.tree.create_element(dom_tag);

        for entry in attributes {
            let attr_name = dom::AttributeName::new(entry.name().as_str())?;
            let attr_val = dom::AttributeValue::new(entry.value().as_str());
            self.tree.set_attribute(node, attr_name, attr_val)?;
        }

        Ok(self.register_node(node))
    }

    fn create_text(&mut self, text: &Text) -> Result<NodeHandle, HtmlError> {
        let content = dom::TextContent::new(text.as_str());
        let node = self.tree.create_text(content);
        Ok(self.register_node(node))
    }

    fn create_comment(&mut self, text: &Text) -> Result<NodeHandle, HtmlError> {
        let content = dom::CommentContent::new(text.as_str());
        let node = self.tree.create_comment(content);
        Ok(self.register_node(node))
    }

    fn append_child(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), HtmlError> {
        let parent_node = self.resolve_node(parent)?;
        let child_node = self.resolve_node(child)?;
        self.tree.append_child(parent_node, child_node)?;
        Ok(())
    }

    fn append_before_sibling(
        &mut self,
        sibling: NodeHandle,
        child: NodeHandle,
    ) -> Result<(), HtmlError> {
        let sibling_node = self.resolve_node(sibling)?;
        let child_node = self.resolve_node(child)?;
        let parent_node = self.tree.parent(sibling_node)?.ok_or_else(|| {
            HtmlError::tree_construction("sibling has no parent to insert before")
        })?;
        self.tree
            .insert_before(parent_node, child_node, sibling_node)?;
        Ok(())
    }

    fn add_attributes_if_missing(
        &mut self,
        target: NodeHandle,
        attributes: &AttributeList,
    ) -> Result<(), HtmlError> {
        let target_node = self.resolve_node(target)?;
        for entry in attributes {
            let attr_name = dom::AttributeName::new(entry.name().as_str())?;
            if self.tree.attribute(target_node, &attr_name).is_err() {
                let attr_val = dom::AttributeValue::new(entry.value().as_str());
                self.tree.set_attribute(target_node, attr_name, attr_val)?;
            }
        }
        Ok(())
    }

    fn remove_from_parent(&mut self, target: NodeHandle) -> Result<(), HtmlError> {
        let target_node = self.resolve_node(target)?;
        self.tree.detach(target_node)?;
        Ok(())
    }

    fn reparent_children(&mut self, from: NodeHandle, to: NodeHandle) -> Result<(), HtmlError> {
        let from_node = self.resolve_node(from)?;
        let to_node = self.resolve_node(to)?;
        let children: Vec<dom::NodeId> = self.tree.children(from_node).collect();
        for child in children {
            self.tree.append_child(to_node, child)?;
        }
        Ok(())
    }

    fn root_node(&self) -> NodeHandle {
        NodeHandle::root()
    }
}
