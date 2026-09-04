//! [`BlockLayout`] — a minimal block-flow [`LayoutEngine`] placeholder.
//!
//! It resolves `margin` / `padding` / `width` / `height` per box and stacks
//! block boxes vertically in document order, with **no margin collapse** and
//! no true nesting geometry — every box is sized against the viewport width and
//! given a single-line content height. B4 replaces this with the real box
//! model, the inline formatting context and Flexbox; B0 only needs a
//! deterministic engine that produces sane, non-overlapping rectangles so the
//! port and its swap can be proven.

use graphics::{Au, Point, Rect, Size};

use crate::application::ports::LayoutEngine;
use crate::domain::computed::display::Display;
use crate::domain::computed::edges::LengthEdges;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::dom_snapshot::{ChildIds, SnapshotId};
use crate::domain::error::{CssError, CssStage};
use crate::domain::layout_box_tree::{EdgeSizes, LayoutBox, LayoutBoxTree, LayoutBoxTreeBuilder};
use crate::domain::styled_tree::{StyledNode, StyledTree};
use crate::domain::viewport::ViewportConstraints;

/// The CSS `initial` computed `font-size`, `16px`, as an [`Au`].
const DEFAULT_FONT_SIZE: Au = match Au::from_whole_px(16) {
    Some(size) => size,
    None => Au::ZERO,
};

/// The block-flow layout engine.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlockLayout;

impl BlockLayout {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LayoutEngine for BlockLayout {
    fn layout(
        &self,
        styled: &StyledTree,
        constraints: &ViewportConstraints,
    ) -> Result<LayoutBoxTree, CssError> {
        let width = constraints.width();
        let mut builder = LayoutBoxTreeBuilder::new();
        let mut pruned: Vec<SnapshotId> = Vec::new();
        let mut cursor_y = Au::ZERO;
        let mut root = None;
        for styled_node in styled.nodes_in_document_order() {
            if is_pruned(&pruned, styled_node) {
                pruned.push(styled_node.node());
                continue;
            }
            if styled_node.style().display() != Display::Block {
                continue;
            }
            let laid_out = block_box(styled, styled_node, width, cursor_y, &pruned)?;
            cursor_y = bottom_edge(&laid_out);
            if root.is_none() {
                root = Some(styled_node.node());
            }
            builder.push(laid_out);
        }
        Ok(builder.finish(root))
    }
}

/// Whether this node's subtree is cut because it — or an ancestor — is
/// `display: none`. Forward document order means a pruned ancestor is already
/// recorded when its descendant is visited.
fn is_pruned(pruned: &[SnapshotId], node: &StyledNode) -> bool {
    node.style().display().is_none() || node.parent().is_some_and(|parent| pruned.contains(&parent))
}

/// Builds one block box: resolved edges, viewport-relative width, single-line
/// content height, stacked below `cursor_y`.
fn block_box(
    styled: &StyledTree,
    node: &StyledNode,
    viewport_width: Au,
    cursor_y: Au,
    pruned: &[SnapshotId],
) -> Result<LayoutBox, CssError> {
    let style = node.style();
    let font_size = resolve_font_size(style, viewport_width);
    let margin = resolve_edges(style.margin(), font_size, viewport_width)
        .ok_or_else(|| CssError::unsupported(CssStage::Layout, "non-finite length in margin"))?;
    let padding = resolve_edges(style.padding(), font_size, viewport_width)
        .ok_or_else(|| CssError::unsupported(CssStage::Layout, "non-finite length in padding"))?;
    let width = viewport_width
        .saturating_sub(margin.horizontal())
        .saturating_sub(padding.horizontal())
        .larger(Au::ZERO);
    let origin = Point::new(
        margin.left().saturating_add(padding.left()),
        cursor_y
            .saturating_add(margin.top())
            .saturating_add(padding.top()),
    );
    let content = Rect::new(origin, Size::new(width, font_size).unwrap_or(Size::EMPTY));
    let children = block_child_ids(styled, node, pruned);
    Ok(LayoutBox::new(
        node.node(),
        content,
        margin,
        padding,
        children,
    ))
}

/// The `y` a following sibling box starts at: below this box's margin, padding
/// and content.
const fn bottom_edge(laid_out: &LayoutBox) -> Au {
    laid_out
        .content()
        .max_y()
        .saturating_add(laid_out.padding().bottom())
        .saturating_add(laid_out.margin().bottom())
}

/// The child ids that will themselves generate a block box.
fn block_child_ids(styled: &StyledTree, node: &StyledNode, pruned: &[SnapshotId]) -> ChildIds {
    ChildIds::from_ids(
        node.children()
            .iter()
            .filter(|child| !pruned.contains(child))
            .filter(|child| generates_block_box(styled, *child)),
    )
}

fn generates_block_box(styled: &StyledTree, child: SnapshotId) -> bool {
    styled
        .node(child)
        .is_some_and(|styled_child| styled_child.style().display() == Display::Block)
}

fn resolve_font_size(style: &ComputedStyle, container: Au) -> Au {
    style
        .font_size()
        .resolve_to_au(DEFAULT_FONT_SIZE, container)
        .unwrap_or(DEFAULT_FONT_SIZE)
}

fn resolve_edges(edges: LengthEdges, font_size: Au, container: Au) -> Option<EdgeSizes> {
    Some(EdgeSizes::new(
        edges.top().resolve_to_au(font_size, container)?,
        edges.right().resolve_to_au(font_size, container)?,
        edges.bottom().resolve_to_au(font_size, container)?,
        edges.left().resolve_to_au(font_size, container)?,
    ))
}
