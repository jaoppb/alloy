//! Default adapter implementing [`html::TreeSink`] over [`DomTree`], and [`parse`].

use html::{
    AttributeEntry, AttributeList, Diagnostics, HtmlError, NodeHandle, ParseDiagnostic, TagName,
    Text, TreeSink,
};

use crate::domain::error::DomError;
use crate::domain::node::NodeId;
use crate::domain::text::{CommentContent, TextContent};
use crate::domain::tree::DomTree;

/// Parses `markup` into a [`DomTree`] plus the recoverable errors met on the way.
pub fn parse(markup: &str) -> Result<ParseOutcome, HtmlError> {
    let mut sink = DomTreeSink::new();
    html::parse_with_sink(markup, &mut sink)?;
    Ok(sink.into_outcome())
}

impl From<DomError> for HtmlError {
    fn from(error: DomError) -> Self {
        Self::tree_construction(error)
    }
}

/// What a successful parse produces: the tree and every recoverable error met on the way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseOutcome {
    tree: DomTree,
    diagnostics: Diagnostics,
}

impl ParseOutcome {
    /// The built tree.
    #[must_use]
    pub const fn tree(&self) -> &DomTree {
        &self.tree
    }

    /// The recoverable parse errors, in detection order.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// Discards the diagnostics and returns the tree.
    #[must_use]
    pub fn into_tree(self) -> DomTree {
        self.tree
    }

    /// Splits into the tree and the diagnostics.
    #[must_use]
    pub fn into_parts(self) -> (DomTree, Diagnostics) {
        (self.tree, self.diagnostics)
    }
}

/// An adapter that builds a real [`DomTree`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomTreeSink {
    tree: DomTree,
    handles: Vec<NodeId>,
    diagnostics: Diagnostics,
}

impl DomTreeSink {
    /// Create a new sink wrapping a fresh [`DomTree`].
    #[must_use]
    pub fn new() -> Self {
        let tree = DomTree::new();
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

    fn attach_attribute(&mut self, node: NodeId, entry: &AttributeEntry) -> Result<(), HtmlError> {
        let name = entry.name().clone();
        let value = entry.value().clone();
        self.tree.set_attribute(node, name, value)?;
        Ok(())
    }

    /// Unwrap and return the built DOM tree.
    #[must_use]
    pub fn into_tree(self) -> DomTree {
        self.tree
    }

    /// Access the underlying DOM tree.
    #[must_use]
    pub const fn tree(&self) -> &DomTree {
        &self.tree
    }

    fn register_node(&mut self, node: NodeId) -> NodeHandle {
        let index = u32::try_from(self.handles.len()).unwrap_or(0);
        self.handles.push(node);
        NodeHandle::new(index)
    }

    fn resolve_node(&self, handle: NodeHandle) -> Result<NodeId, HtmlError> {
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
        let node = self.tree.create_element(tag);

        for entry in attributes {
            self.attach_attribute(node, entry)?;
        }

        Ok(self.register_node(node))
    }

    fn create_text(&mut self, text: &Text) -> Result<NodeHandle, HtmlError> {
        let content = TextContent::new(text.as_str());
        let node = self.tree.create_text(content);
        Ok(self.register_node(node))
    }

    fn create_comment(&mut self, text: &Text) -> Result<NodeHandle, HtmlError> {
        let content = CommentContent::new(text.as_str());
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
            let already_present =
                matches!(self.tree.attribute(target_node, entry.name()), Ok(Some(_)));
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
        let children: Vec<NodeId> = self.tree.children(from_node).collect();
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
