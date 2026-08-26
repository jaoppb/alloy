use crate::domain::specificity::Specificity;
use dom::{AttributeName, DomNode, DomTree, NodeData, NodeId, TagName};

/// Attribute matching operators supported in W3C Selectors Level 3 (C-20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeMatcher {
    /// Matches if attribute is present regardless of value: `[attr]`
    Exists,
    /// Matches if attribute value equals string: `[attr=val]`
    Exact(String),
    /// Matches if whitespace-separated list contains word: `[attr~=val]`
    Includes(String),
    /// Matches if value equals or starts with val followed by hyphen: `[attr|=val]`
    DashMatch(String),
    /// Matches if value starts with prefix: `[attr^=val]`
    Prefix(String),
    /// Matches if value ends with suffix: `[attr$=val]`
    Suffix(String),
    /// Matches if value contains substring: `[attr*=val]`
    Substring(String),
}

/// Structural and state pseudo-classes supported in W3C Selectors Level 3 (C-20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PseudoClass {
    /// Matches the root element of the document: `:root`
    Root,
    /// Matches an element that is the first child among its parent's element children: `:first-child`
    FirstChild,
    /// Matches an element that is the last child among its parent's element children: `:last-child`
    LastChild,
    /// Matches an element that is the sole child among its parent's element children: `:only-child`
    OnlyChild,
    /// Matches an element that has no element children and no non-empty text: `:empty`
    Empty,
}

/// CSS selector matching elements in the `DomTree` with Selectors Level 3 combinators (C-20).
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
    /// Attribute selector matching `[attr]`, `[attr=val]`, etc.
    Attribute {
        name: AttributeName,
        matcher: AttributeMatcher,
    },
    /// Pseudo-class selector matching `:first-child`, `:root`, etc.
    PseudoClass(PseudoClass),
    /// Compound selector matching multiple criteria simultaneously (e.g. `div.hero#main`).
    Compound(Vec<Selector>),
    /// Descendant combinator matching `ancestor descendant` (e.g. `div p`).
    Descendant(Box<Selector>, Box<Selector>),
    /// Child combinator matching `parent > child` (e.g. `ul > li`).
    Child(Box<Selector>, Box<Selector>),
    /// Next-sibling combinator matching `prev + next` (e.g. `h1 + p`).
    AdjacentSibling(Box<Selector>, Box<Selector>),
    /// Subsequent-sibling combinator matching `prev ~ next` (e.g. `h1 ~ p`).
    GeneralSibling(Box<Selector>, Box<Selector>),
}

impl Selector {
    /// Calculates the specificity of this selector according to W3C Selectors Level 3.
    #[must_use]
    pub fn specificity(&self) -> Specificity {
        match self {
            Self::Universal => Specificity::new(0, 0, 0),
            Self::Tag(_) => Specificity::new(0, 0, 1),
            Self::Class(_) | Self::Attribute { .. } | Self::PseudoClass(_) => {
                Specificity::new(0, 1, 0)
            }
            Self::Id(_) => Specificity::new(1, 0, 0),
            Self::Compound(parts) => parts
                .iter()
                .fold(Specificity::new(0, 0, 0), |acc, s| acc + s.specificity()),
            Self::Descendant(a, b)
            | Self::Child(a, b)
            | Self::AdjacentSibling(a, b)
            | Self::GeneralSibling(a, b) => a.specificity() + b.specificity(),
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
            Self::Attribute { name, matcher } => {
                let Some(val) = attributes.get(name) else {
                    return false;
                };
                let val_str = val.as_str();
                match matcher {
                    AttributeMatcher::Exists => true,
                    AttributeMatcher::Exact(expected) => val_str == expected,
                    AttributeMatcher::Includes(expected) => {
                        val_str.split_ascii_whitespace().any(|w| w == expected)
                    }
                    AttributeMatcher::DashMatch(expected) => {
                        val_str == expected || val_str.starts_with(&format!("{expected}-"))
                    }
                    AttributeMatcher::Prefix(prefix) => val_str.starts_with(prefix),
                    AttributeMatcher::Suffix(suffix) => val_str.ends_with(suffix),
                    AttributeMatcher::Substring(sub) => val_str.contains(sub),
                }
            }
            Self::PseudoClass(pseudo) => match pseudo {
                PseudoClass::Root => tree.root() == Some(node_id) || tag_name == &TagName::Html,
                PseudoClass::FirstChild => {
                    if let Some(parent_id) = node.parent() {
                        if let Some(parent_node) = tree.get(parent_id) {
                            let first_el =
                                parent_node.children().as_slice().iter().find(|&&child_id| {
                                    tree.get(child_id).is_some_and(|n| n.data().is_element())
                                });
                            return first_el == Some(&node_id);
                        }
                    }
                    false
                }
                PseudoClass::LastChild => {
                    if let Some(parent_id) = node.parent() {
                        if let Some(parent_node) = tree.get(parent_id) {
                            let last_el =
                                parent_node
                                    .children()
                                    .as_slice()
                                    .iter()
                                    .rfind(|&&child_id| {
                                        tree.get(child_id).is_some_and(|n| n.data().is_element())
                                    });
                            return last_el == Some(&node_id);
                        }
                    }
                    false
                }
                PseudoClass::OnlyChild => {
                    if let Some(parent_id) = node.parent() {
                        if let Some(parent_node) = tree.get(parent_id) {
                            let el_children: Vec<NodeId> = parent_node
                                .children()
                                .as_slice()
                                .iter()
                                .copied()
                                .filter(|&child_id| {
                                    tree.get(child_id).is_some_and(|n| n.data().is_element())
                                })
                                .collect();
                            return el_children.len() == 1 && el_children[0] == node_id;
                        }
                    }
                    false
                }
                PseudoClass::Empty => node.children().as_slice().iter().all(|&child_id| {
                    tree.get(child_id)
                        .is_none_or(|c| c.data().as_text().is_some_and(|t| t.trim().is_empty()))
                }),
            },
            Self::Compound(parts) => parts.iter().all(|part| part.matches(node_id, tree)),
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
            Self::Child(parent_sel, child_sel) => {
                if !child_sel.matches(node_id, tree) {
                    return false;
                }

                let Some(parent_id) = node.parent() else {
                    return false;
                };

                parent_sel.matches(parent_id, tree)
            }
            Self::AdjacentSibling(prev_sel, next_sel) => {
                if !next_sel.matches(node_id, tree) {
                    return false;
                }

                let Some(parent_id) = node.parent() else {
                    return false;
                };
                let Some(parent_node) = tree.get(parent_id) else {
                    return false;
                };

                let children = parent_node.children().as_slice();
                let Some(current_pos) = children.iter().position(|&id| id == node_id) else {
                    return false;
                };

                // Look backwards for the immediately preceding element sibling
                for &sibling_id in children[..current_pos].iter().rev() {
                    if let Some(sibling_node) = tree.get(sibling_id) {
                        if sibling_node.data().is_element() {
                            return prev_sel.matches(sibling_id, tree);
                        }
                    }
                }

                false
            }
            Self::GeneralSibling(prev_sel, next_sel) => {
                if !next_sel.matches(node_id, tree) {
                    return false;
                }

                let Some(parent_id) = node.parent() else {
                    return false;
                };
                let Some(parent_node) = tree.get(parent_id) else {
                    return false;
                };

                let children = parent_node.children().as_slice();
                let Some(current_pos) = children.iter().position(|&id| id == node_id) else {
                    return false;
                };

                // Check if any earlier element sibling matches
                for &sibling_id in &children[..current_pos] {
                    if let Some(sibling_node) = tree.get(sibling_id) {
                        if sibling_node.data().is_element() && prev_sel.matches(sibling_id, tree) {
                            return true;
                        }
                    }
                }

                false
            }
        }
    }
}
