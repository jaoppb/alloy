use crate::domain::node_id::NodeId;
use crate::domain::tag_name::TagName;
use crate::domain::tree::DomTree;

/// Domain service utility delegating directly to `DomTree` entity methods (C-48).
pub struct DomService;

impl DomService {
    /// Recursively collects all text content inside a subtree rooted at `root`.
    #[must_use]
    pub fn get_text_content(tree: &DomTree, root: NodeId) -> String {
        tree.get_text_content(root)
    }

    /// Finds all element node IDs in the subtree that match `tag`.
    #[must_use]
    pub fn find_by_tag_name(tree: &DomTree, root: NodeId, tag: &TagName) -> Vec<NodeId> {
        tree.find_by_tag_name(root, tag)
    }

    /// Serializes a node subtree to a compact HTML-like string representation.
    #[must_use]
    pub fn serialize_to_html(tree: &DomTree, root: NodeId) -> String {
        tree.serialize_to_html(root)
    }
}
