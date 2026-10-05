//! Default adapter implementing [`TreeSink`] over [`dom::DomTree`].

#![cfg(feature = "dom")]

use crate::application::ports::TreeSink;
use crate::domain::attribute::{AttributeEntry, AttributeList};
use crate::domain::diagnostic::{Diagnostics, ParseDiagnostic, ParseErrorCode};
use crate::domain::error::HtmlError;
use crate::domain::handle::NodeHandle;
use crate::domain::tag::TagName;
use crate::domain::text::Text;

/// What a successful parse produces: the tree and every recoverable error met on the way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseOutcome {
    tree: dom::DomTree,
    diagnostics: Diagnostics,
}

impl ParseOutcome {
    /// The built tree.
    #[must_use]
    pub const fn tree(&self) -> &dom::DomTree {
        &self.tree
    }

    /// The recoverable parse errors, in detection order.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// Discards the diagnostics and returns the tree.
    #[must_use]
    pub fn into_tree(self) -> dom::DomTree {
        self.tree
    }

    /// Splits into the tree and the diagnostics.
    #[must_use]
    pub fn into_parts(self) -> (dom::DomTree, Diagnostics) {
        (self.tree, self.diagnostics)
    }
}

/// An adapter that builds a real [`dom::DomTree`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomTreeSink {
    tree: dom::DomTree,
    handles: Vec<dom::NodeId>,
    diagnostics: Diagnostics,
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
            diagnostics: Diagnostics::new(),
        }
    }

    /// Unwrap and return the built tree together with the diagnostics collected.
    #[must_use]
    pub fn into_outcome(self) -> ParseOutcome {
        ParseOutcome {
            tree: self.tree,
            diagnostics: self.diagnostics,
        }
    }

    /// The diagnostics collected so far.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// Attaches `entry` to `node`, or reports why the DOM refuses it (never a silent drop).
    fn attach_attribute(
        &mut self,
        node: dom::NodeId,
        entry: &AttributeEntry,
    ) -> Result<(), HtmlError> {
        let Ok(attr_name) = dom::AttributeName::new(entry.name().as_str()) else {
            self.parse_error(ParseDiagnostic::new(
                ParseErrorCode::UnsupportedAttributeName,
                entry.location(),
            ));
            return Ok(());
        };
        let attr_val = dom::AttributeValue::new(entry.value().as_str());
        self.tree.set_attribute(node, attr_name, attr_val)?;
        Ok(())
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
            self.attach_attribute(node, entry)?;
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
            let already_present = dom::AttributeName::new(entry.name().as_str())
                .is_ok_and(|name| matches!(self.tree.attribute(target_node, &name), Ok(Some(_))));
            if !already_present {
                self.attach_attribute(target_node, entry)?;
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

    fn parse_error(&mut self, diagnostic: ParseDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    fn root_node(&self) -> NodeHandle {
        NodeHandle::root()
    }
}
