use crate::domain::node_data::NodeData;
use crate::domain::node_id::NodeId;
use crate::domain::tag_name::TagName;
use crate::domain::tree::DomTree;

/// Domain service providing utility operations over a DOM tree.
pub struct DomService;

impl DomService {
    /// Recursively collects all text content inside a subtree rooted at `root`.
    #[must_use]
    pub fn get_text_content(tree: &DomTree, root: NodeId) -> String {
        let mut buffer = String::new();
        let nodes = tree.traverse_pre_order(root);

        for id in nodes {
            if let Some(node) = tree.get(id) {
                if let NodeData::Text(text) = node.data() {
                    buffer.push_str(text);
                }
            }
        }

        buffer
    }

    /// Finds all element node IDs in the subtree that match `tag`.
    #[must_use]
    pub fn find_by_tag_name(tree: &DomTree, root: NodeId, tag: &TagName) -> Vec<NodeId> {
        let mut matched = Vec::new();
        let nodes = tree.traverse_pre_order(root);

        for id in nodes {
            if let Some(node) = tree.get(id) {
                if let Some(node_tag) = node.data().as_element_tag() {
                    if node_tag == tag {
                        matched.push(id);
                    }
                }
            }
        }

        matched
    }

    /// Serializes a node subtree to a compact HTML-like string representation.
    #[must_use]
    pub fn serialize_to_html(tree: &DomTree, root: NodeId) -> String {
        let mut output = String::new();
        Self::serialize_node(tree, root, &mut output);
        output
    }

    fn serialize_node(tree: &DomTree, id: NodeId, out: &mut String) {
        let Some(node) = tree.get(id) else {
            return;
        };

        match node.data() {
            NodeData::Document => {
                for child_id in node.children().iter() {
                    Self::serialize_node(tree, child_id, out);
                }
            }
            NodeData::Element {
                tag_name,
                attributes,
            } => {
                out.push('<');
                out.push_str(tag_name.as_str());
                for (name, val) in attributes.iter() {
                    out.push(' ');
                    out.push_str(name.as_str());
                    out.push_str("=\"");
                    out.push_str(val.as_str());
                    out.push('"');
                }
                out.push('>');

                for child_id in node.children().iter() {
                    Self::serialize_node(tree, child_id, out);
                }

                out.push_str("</");
                out.push_str(tag_name.as_str());
                out.push('>');
            }
            NodeData::Text(text) => {
                out.push_str(text);
            }
            NodeData::Comment(comment) => {
                out.push_str("<!--");
                out.push_str(comment);
                out.push_str("-->");
            }
        }
    }
}
