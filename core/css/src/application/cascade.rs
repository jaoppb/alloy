use crate::domain::computed::ComputedStyle;
use crate::domain::declaration::Declaration;
use crate::domain::property::{DisplayType, PropertyValue};
use crate::domain::specificity::Specificity;
use crate::domain::styled_node::{StyledNode, StyledTree};
use crate::domain::stylesheet::StyleSheet;
use dom::{DomTree, NodeData, NodeId};

/// Service computing cascaded styles across a `DomTree` and generating a `StyledTree`.
pub struct StyleCascade;

impl StyleCascade {
    /// Builds a `StyledTree` by matching stylesheet rules against a `DomTree`.
    #[must_use]
    pub fn build_styled_tree(tree: &DomTree, stylesheet: &StyleSheet) -> StyledTree {
        let Some(root_id) = tree.root() else {
            return StyledTree::new(None);
        };

        let root_styled = Self::build_styled_node(tree, stylesheet, root_id, None);
        StyledTree::new(root_styled)
    }

    fn build_styled_node(
        tree: &DomTree,
        stylesheet: &StyleSheet,
        node_id: NodeId,
        parent_style: Option<&ComputedStyle>,
    ) -> Option<StyledNode> {
        let node = tree.get(node_id)?;

        let mut computed = ComputedStyle::default();
        if let Some(parent) = parent_style {
            computed.inherit_from(parent);
        }

        // Apply matching CSS declarations if element
        if let NodeData::Element { tag_name, .. } = node.data() {
            // Set element default display
            computed.set_display(DisplayType::from_tag(tag_name));

            let mut matched_decls: Vec<(Specificity, usize, Declaration)> = Vec::new();
            for (rule_idx, rule) in stylesheet.rules().iter().enumerate() {
                for selector in rule.selectors() {
                    if selector.matches(node_id, tree) {
                        let spec = selector.specificity();
                        for decl in rule.declarations().iter() {
                            matched_decls.push((spec, rule_idx, decl.clone()));
                        }
                    }
                }
            }

            // Sort declarations by specificity and source order
            matched_decls.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

            for (_, _, decl) in matched_decls {
                apply_declaration(&mut computed, &decl);
            }
        }

        let mut children = Vec::new();
        for child_id in node.children().iter() {
            if let Some(child_styled) =
                Self::build_styled_node(tree, stylesheet, child_id, Some(&computed))
            {
                children.push(child_styled);
            }
        }

        Some(StyledNode::new(node_id, computed, children))
    }
}

fn apply_declaration(style: &mut ComputedStyle, decl: &Declaration) {
    let name = decl.name().as_str();

    match name {
        "display" => {
            if let PropertyValue::Keyword(kw) = decl.value() {
                let disp = match kw.as_str() {
                    "inline" => DisplayType::Inline,
                    "none" => DisplayType::None,
                    "flex" => DisplayType::Flex,
                    _ => DisplayType::Block,
                };
                style.set_display(disp);
            }
        }
        "color" => {
            if let PropertyValue::Color(c) = decl.value() {
                style.set_color(*c);
            }
        }
        "background-color" | "background" => {
            if let PropertyValue::Color(c) = decl.value() {
                style.set_background_color(*c);
            }
        }
        "font-size" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_font_size(*px);
            }
        }
        "width" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_width(Some(*px));
            }
        }
        "height" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_height(Some(*px));
            }
        }
        "margin" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_margin_top(*px);
                style.set_margin_right(*px);
                style.set_margin_bottom(*px);
                style.set_margin_left(*px);
            }
        }
        "margin-top" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_margin_top(*px);
            }
        }
        "margin-right" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_margin_right(*px);
            }
        }
        "margin-bottom" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_margin_bottom(*px);
            }
        }
        "margin-left" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_margin_left(*px);
            }
        }
        "padding" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_padding_top(*px);
                style.set_padding_right(*px);
                style.set_padding_bottom(*px);
                style.set_padding_left(*px);
            }
        }
        "padding-top" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_padding_top(*px);
            }
        }
        "padding-right" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_padding_right(*px);
            }
        }
        "padding-bottom" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_padding_bottom(*px);
            }
        }
        "padding-left" => {
            if let PropertyValue::Length(px) = decl.value() {
                style.set_padding_left(*px);
            }
        }
        _ => {}
    }
}
