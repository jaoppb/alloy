use crate::domain::command::RenderCommand;
use crate::domain::display_list::DisplayList;
use crate::domain::geometry::Rect;
use css::{Color, DisplayType, StyledNode, StyledTree};
use dom::{DomTree, NodeData};

/// Translates a `StyledTree` and `DomTree` into a positioned 2D `DisplayList`.
pub struct LayoutEngine;

impl LayoutEngine {
    /// Translates a styled DOM hierarchy into a declarative `DisplayList`.
    #[must_use]
    pub fn layout(
        tree: &DomTree,
        styled_tree: &StyledTree,
        viewport_w: f32,
        _viewport_h: f32,
    ) -> DisplayList {
        let mut list = DisplayList::new();
        // Clear background with white
        list.push(RenderCommand::Clear(Color::WHITE));

        let Some(root) = styled_tree.root() else {
            return list;
        };

        let mut current_y = 10.0;
        Self::layout_node(
            tree,
            root,
            10.0,
            &mut current_y,
            viewport_w - 20.0,
            &mut list,
        );

        list
    }

    fn layout_node(
        tree: &DomTree,
        styled: &StyledNode,
        x: f32,
        current_y: &mut f32,
        available_w: f32,
        list: &mut DisplayList,
    ) {
        let style = styled.style();
        if style.display == DisplayType::None {
            return;
        }

        let node_x = x + style.margin_left.value();
        let node_y = *current_y + style.margin_top.value();
        let node_w = style
            .width
            .map(|w| w.value())
            .unwrap_or(available_w - style.margin_left.value() - style.margin_right.value())
            .max(10.0);

        let default_line_h = style.font_size.value() * 1.4;
        let mut node_h = style.height.map(|h| h.value()).unwrap_or(default_line_h);

        // If background is not transparent, emit background rect
        if style.background_color != Color::TRANSPARENT {
            list.push(RenderCommand::DrawRect {
                rect: Rect::new(node_x, node_y, node_w, node_h),
                color: style.background_color,
            });
        }

        // If this DOM node has text content, emit DrawText
        if let Some(dom_node) = tree.get(styled.node_id()) {
            if let NodeData::Text(content) = dom_node.data() {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    list.push(RenderCommand::DrawText {
                        text: trimmed.to_string(),
                        rect: Rect::new(node_x, node_y, node_w, node_h),
                        color: style.color,
                        font_size: style.font_size.value(),
                    });
                }
            }
        }

        *current_y = node_y + style.padding_top.value();

        // Layout children
        let inner_w = node_w - style.padding_left.value() - style.padding_right.value();
        let inner_x = node_x + style.padding_left.value();

        for child in styled.children() {
            Self::layout_node(tree, child, inner_x, current_y, inner_w, list);
        }

        let total_child_h = *current_y - node_y;
        if style.height.is_none() && total_child_h > node_h {
            node_h = total_child_h;
        }

        *current_y = node_y + node_h + style.margin_bottom.value();
    }
}
