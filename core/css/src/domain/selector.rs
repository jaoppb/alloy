use crate::domain::specificity::Specificity;
use dom::{AttributeName, DomNode, DomTree, NodeData, NodeId, TagName};

/// CSS selector matching elements in the `DomTree`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// Universal selector `*`.
    Universal,
    /// Type selector matching an HTML tag (e.g. `h1`, `div`).
    Tag(TagName),
    /// Class selector matching `.class-name`.
    Class(String),
    /// ID selector matching `#element-id`.
    Id(String),
    /// Descendant combinator matching `ancestor descendant` (e.g. `div p`).
    Descendant(Box<Selector>, Box<Selector>),
}

impl Selector {
    /// Calculates the specificity of this selector.
    #[must_use]
    pub fn specificity(&self) -> Specificity {
        match self {
            Self::Universal => Specificity::new(0, 0, 0),
            Self::Tag(_) => Specificity::new(0, 0, 1),
            Self::Class(_) => Specificity::new(0, 1, 0),
            Self::Id(_) => Specificity::new(1, 0, 0),
            Self::Descendant(ancestor, descendant) => {
                ancestor.specificity() + descendant.specificity()
            }
        }
    }

    /// Checks if this selector matches a specific node in the given DOM tree.
    #[must_use]
    pub fn matches(&self, node_id: NodeId, tree: &DomTree) -> bool {
        let Some(node) = tree.get(node_id) else {
            return false;
        };

        // Selectors only match element nodes
        let NodeData::Element {
            tag_name,
            attributes,
        } = node.data()
        else {
            return false;
        };

        match self {
            Self::Universal => true,
            Self::Tag(expected_tag) => tag_name == expected_tag,
            Self::Class(expected_class) => {
                let class_attr = AttributeName::new("class");
                if let Some(val) = attributes.get(&class_attr) {
                    return val
                        .as_str()
                        .split_ascii_whitespace()
                        .any(|c| c == expected_class);
                }
                false
            }
            Self::Id(expected_id) => {
                let id_attr = AttributeName::new("id");
                if let Some(val) = attributes.get(&id_attr) {
                    return val.as_str().trim() == expected_id;
                }
                false
            }
            Self::Descendant(ancestor_sel, descendant_sel) => {
                if !descendant_sel.matches(node_id, tree) {
                    return false;
                }

                let mut current_parent = node.parent();
                while let Some(parent_id) = current_parent {
                    if ancestor_sel.matches(parent_id, tree) {
                        return true;
                    }
                    current_parent = tree.get(parent_id).and_then(DomNode::parent);
                }

                false
            }
        }
    }
}
