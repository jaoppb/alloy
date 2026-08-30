//! [`Descendants`] and [`Ancestors`] — non-recursive tree iterators
//! (v0.2 report §2.3). Each keeps an explicit stack / cursor; neither ever
//! calls itself, so a hostile-depth tree cannot overflow the call stack.

use crate::domain::node::NodeId;
use crate::domain::tree::DomTree;

/// Pre-order (document-order) iterator over the strict descendants of a node.
/// Built by [`DomTree::descendants`].
pub struct Descendants<'tree> {
    tree: &'tree DomTree,
    stack: Vec<NodeId>,
}

impl<'tree> Descendants<'tree> {
    pub(crate) fn new(tree: &'tree DomTree, root: NodeId) -> Self {
        let mut descendants = Self {
            tree,
            stack: Vec::new(),
        };
        descendants.push_children_of(root);
        descendants
    }

    fn push_children_of(&mut self, parent: NodeId) {
        for child in self.tree.child_id_vec(parent).into_iter().rev() {
            self.stack.push(child);
        }
    }
}

impl Iterator for Descendants<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.stack.pop()?;
        self.push_children_of(current);
        Some(current)
    }
}

/// Iterator from a node's parent up to and including the `Document` root. Built
/// by [`DomTree::ancestors`].
pub struct Ancestors<'tree> {
    tree: &'tree DomTree,
    next: Option<NodeId>,
}

impl<'tree> Ancestors<'tree> {
    pub(crate) fn new(tree: &'tree DomTree, node: NodeId) -> Self {
        Self {
            tree,
            next: tree.parent_of(node),
        }
    }
}

impl Iterator for Ancestors<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;
        self.next = self.tree.parent_of(current);
        Some(current)
    }
}
