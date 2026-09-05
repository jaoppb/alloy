//! [`BlockLayout`] — the built-in flow-plus-Flexbox [`LayoutEngine`]
//! (`PRD-007` §3.5), delivered in v0.5 B4.
//!
//! This file owns two things: the adapter itself, and the **block** formatting
//! context. A block container stacks its in-flow children vertically, collapsing
//! their vertical margins (CSS 2.1 §8.3.1); a run of consecutive inline-level
//! children is handed to [`inline`] as one anonymous block, which is how
//! `<div>text<p>x</p>more</div>` keeps all three pieces; a `display: flex`
//! container is handed to [`flex`].
//!
//! Every result is **relative**: `layout_box` answers with fragments positioned
//! against its own border-box origin, and the caller translates. Nothing here
//! holds a page-global cursor, which is what lets the three contexts nest
//! without knowing about each other.

use std::sync::Arc;

use core::fmt;

use graphics::{Au, Point};

use crate::application::ports::{LayoutEngine, TextMeasurer};
use crate::domain::computed::display::Display;
use crate::domain::computed::inline_style::TextAlign;
use crate::domain::computed::intrinsic::IntrinsicSize;
use crate::domain::dom_snapshot::{ChildIds, SnapshotId};
use crate::domain::error::CssError;
use crate::domain::layout_box_tree::{LayoutBoxTree, LayoutBoxTreeBuilder};
use crate::domain::styled_tree::{StyledNode, StyledTree};
use crate::domain::viewport::ViewportConstraints;
use crate::infrastructure::layout::box_model::{self, BoxMetrics, DEFAULT_FONT_SIZE};
use crate::infrastructure::layout::context::{
    BlockInput, BlockResult, BorderBoxSize, ContentFlow, LayoutContext, MAX_LAYOUT_DEPTH,
};
use crate::infrastructure::layout::fragment::{Fragment, Fragments, rect_at};
use crate::infrastructure::layout::margin_collapse::{
    CollapsedMargin, MarginFlow, collapses_at_bottom, collapses_at_top,
};
use crate::infrastructure::layout::{flex, inline};
use crate::infrastructure::text_metrics::MonospaceMetrics;

/// The built-in layout engine: normal flow, an inline formatting context, and
/// Flexbox.
#[derive(Clone)]
pub struct BlockLayout {
    measurer: Arc<dyn TextMeasurer>,
}

impl BlockLayout {
    /// A layout engine measuring text with the deterministic
    /// [`MonospaceMetrics`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            measurer: Arc::new(MonospaceMetrics::new()),
        }
    }

    /// A layout engine measuring text through `measurer` — the seam a real
    /// font-backed measurer enters by, with no font type named here.
    #[must_use]
    pub const fn with_measurer(measurer: Arc<dyn TextMeasurer>) -> Self {
        Self { measurer }
    }
}

impl Default for BlockLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for BlockLayout {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BlockLayout")
    }
}

impl LayoutEngine for BlockLayout {
    fn layout(
        &self,
        styled: &StyledTree,
        constraints: &ViewportConstraints,
    ) -> Result<LayoutBoxTree, CssError> {
        let context = LayoutContext::new(styled, self.measurer.as_ref());
        let root = styled.root();
        let mut builder = LayoutBoxTreeBuilder::new();
        if !generates_box(&context, root) {
            return Ok(builder.finish(None));
        }
        let input = BlockInput::new(constraints.width(), DEFAULT_FONT_SIZE);
        let result = layout_box(&context, root, input)?;
        let placed = place_root(result);
        builder.push_all(placed.into_boxes());
        Ok(builder.finish(Some(root)))
    }
}

/// The root box sits at the viewport origin, offset by its own left margin and
/// by the margin that escaped its top edge.
fn place_root(result: BlockResult) -> Fragments {
    let edges = result.edges();
    let margin = edges.margin();
    let vertical = result.top_margin().resolve();
    let horizontal = margin.left();
    result.into_fragments().translated(horizontal, vertical)
}

fn generates_box(context: &LayoutContext<'_>, node: SnapshotId) -> bool {
    let Ok(styled) = context.node(node) else {
        return false;
    };
    !display_of(styled).is_none()
}

const fn display_of(styled: &StyledNode) -> Display {
    let style = styled.style();
    style.display()
}

/// Lays one box out inside the containing block `input` describes.
pub(crate) fn layout_box(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    input: BlockInput,
) -> Result<BlockResult, CssError> {
    if input.depth() > MAX_LAYOUT_DEPTH {
        return Err(BlockInput::too_deep());
    }
    let node = context.node(node_id)?;
    let style = node.style();
    let font_size = box_model::font_size_of(style, input.parent_font_size());
    let metrics = box_model::resolve(style, font_size, input.containing_width())?;
    let content_width = input
        .forced_content_width()
        .unwrap_or_else(|| metrics.content_width_within(input.containing_width()));
    let children = layout_content(context, node, content_width, font_size, input)?;
    assemble(
        context,
        node,
        Resolved::new(metrics, content_width),
        children,
        input,
    )
}

/// What resolving a node's own box produced, before its children are folded in.
#[derive(Clone, Copy)]
struct Resolved {
    metrics: BoxMetrics,
    content_width: Au,
}

impl Resolved {
    const fn new(metrics: BoxMetrics, content_width: Au) -> Self {
        Self {
            metrics,
            content_width,
        }
    }
}

/// Folds a box's own metrics and its children's flow into one [`BlockResult`],
/// applying the two margin-collapsing decisions only a parent can make.
fn assemble(
    context: &LayoutContext<'_>,
    node: &StyledNode,
    resolved: Resolved,
    children: ContentFlow,
    input: BlockInput,
) -> Result<BlockResult, CssError> {
    let metrics = resolved.metrics;
    let edges = metrics.edges();
    let margin = edges.margin();
    let (top_margin, children_offset) = top_arrangement(
        collapses_at_top(edges),
        CollapsedMargin::from_length(margin.top()),
        children.leading_margin(),
    );
    let (bottom_margin, trailing_extra) = bottom_arrangement(
        collapses_at_bottom(edges, metrics.height()),
        CollapsedMargin::from_length(margin.bottom()),
        children.trailing_margin(),
    );
    let content_height = used_content_height(
        &children,
        metrics,
        input,
        children_offset.saturating_add(trailing_extra),
    );
    let flow = flow_of(&children, metrics, content_height, input);
    let fragments = assemble_fragments(
        context,
        node,
        resolved,
        children,
        children_offset,
        content_height,
    )?;
    let size = BorderBoxSize::new(
        metrics.border_box_width(resolved.content_width),
        metrics.border_box_height(content_height),
    );
    Ok(BlockResult::new(
        size,
        edges,
        collapsed_pair(flow, top_margin, bottom_margin),
        flow,
        fragments,
    ))
}

/// A box that collapses through reports **one** margin set holding all four
/// adjoining margins; a separated box reports its two ends.
const fn collapsed_pair(
    flow: MarginFlow,
    top: CollapsedMargin,
    bottom: CollapsedMargin,
) -> (CollapsedMargin, CollapsedMargin) {
    if flow.collapses_through() {
        return (top.adjoin(bottom), CollapsedMargin::ZERO);
    }
    (top, bottom)
}

/// The margin escaping this box's top edge, and how far its children start
/// below the content-box top.
const fn top_arrangement(
    collapses: bool,
    own: CollapsedMargin,
    leading: CollapsedMargin,
) -> (CollapsedMargin, Au) {
    if collapses {
        return (own.adjoin(leading), Au::ZERO);
    }
    (own, leading.resolve())
}

/// The margin escaping this box's bottom edge, and how much extra content
/// height the last child's margin claims.
const fn bottom_arrangement(
    collapses: bool,
    own: CollapsedMargin,
    trailing: CollapsedMargin,
) -> (CollapsedMargin, Au) {
    if collapses {
        return (own.adjoin(trailing), Au::ZERO);
    }
    (own, trailing.resolve())
}

/// A forced height (a flex item) wins over a declared one, which wins over the
/// height the children produced.
fn used_content_height(
    children: &ContentFlow,
    metrics: BoxMetrics,
    input: BlockInput,
    escaping: Au,
) -> Au {
    let declared = input.forced_content_height().or_else(|| metrics.height());
    declared.unwrap_or_else(|| children.height().saturating_add(escaping))
}

/// Whether this box's own top and bottom margins end up adjoining: nothing of
/// its own may separate them, and its children must have collapsed through too.
const fn flow_of(
    children: &ContentFlow,
    metrics: BoxMetrics,
    content_height: Au,
    input: BlockInput,
) -> MarginFlow {
    let separated = !children.flow().collapses_through()
        || !content_height.is_zero()
        || !metrics.inner_vertical().is_zero()
        || metrics.height().is_some()
        || input.forced_content_height().is_some();
    if separated {
        return MarginFlow::Separated;
    }
    MarginFlow::CollapsesThrough
}

/// This box's own fragment first (document order), then its children's, moved
/// into the content box. `content_height` is exactly what `assemble` already
/// resolved via `used_content_height` — computed once, so the fragment drawn
/// here and the border-box height `assemble` reports can never disagree.
fn assemble_fragments(
    context: &LayoutContext<'_>,
    node: &StyledNode,
    resolved: Resolved,
    children: ContentFlow,
    children_offset: Au,
    content_height: Au,
) -> Result<Fragments, CssError> {
    let metrics = resolved.metrics;
    let edges = metrics.edges();
    let inset = content_inset(metrics);
    let own = Fragment::new(
        node.node(),
        rect_at(inset, resolved.content_width, content_height),
        edges,
        marker_for(node, metrics),
        box_generating_children(context, node)?,
    );
    let mut fragments = Fragments::one(own);
    let vertical = inset.vertical().saturating_add(children_offset);
    fragments.absorb(
        children
            .into_fragments()
            .translated(inset.horizontal(), vertical),
    );
    Ok(fragments)
}

/// The offset from the border-box origin to the content-box origin.
const fn content_inset(metrics: BoxMetrics) -> Point {
    let edges = metrics.edges();
    let border = edges.border();
    let padding = edges.padding();
    Point::new(
        border.left().saturating_add(padding.left()),
        border.top().saturating_add(padding.top()),
    )
}

/// A replaced element keeps its `Pending` marker only while the cascade left it
/// without both axes — once an author pins `width` **and** `height`, the box's
/// geometry no longer depends on the resource.
const fn marker_for(node: &StyledNode, metrics: BoxMetrics) -> IntrinsicSize {
    let pinned = metrics.width().is_some() && metrics.height().is_some();
    if node.intrinsic_size().is_pending() && !pinned {
        return IntrinsicSize::Pending;
    }
    IntrinsicSize::Resolved
}

fn box_generating_children(
    context: &LayoutContext<'_>,
    node: &StyledNode,
) -> Result<ChildIds, CssError> {
    let mut kept = Vec::new();
    for child in node.children().iter() {
        let styled = context.node(child)?;
        keep_if_boxed(&mut kept, styled, child);
    }
    Ok(ChildIds::from_ids(kept))
}

fn keep_if_boxed(kept: &mut Vec<SnapshotId>, styled: &StyledNode, child: SnapshotId) {
    if display_of(styled).is_none() {
        return;
    }
    kept.push(child);
}

// ---- the block formatting context ----------------------------------------

/// One stretch of a block container's children: either a run of inline-level
/// boxes forming an anonymous block, or one block-level box.
enum Segment {
    Inline(Vec<SnapshotId>),
    Block(SnapshotId),
}

/// The children of `node`, laid out inside a content box `content_width` wide.
fn layout_content(
    context: &LayoutContext<'_>,
    node: &StyledNode,
    content_width: Au,
    font_size: Au,
    input: BlockInput,
) -> Result<ContentFlow, CssError> {
    if display_of(node) == Display::Flex {
        return flex::layout(context, node, content_width, font_size, input);
    }
    let segments = segments_of(context, node)?;
    if segments.is_empty() {
        return Ok(ContentFlow::empty());
    }
    let align = node.style().text_align();
    stack_segments(context, &segments, content_width, font_size, align, input)
}

/// Splits the in-flow children into runs of inline-level boxes and single
/// block-level boxes, in document order. A node that is itself a text node
/// contributes itself — that is how a text node blockified by a flex container
/// still gets a line box.
fn segments_of(context: &LayoutContext<'_>, node: &StyledNode) -> Result<Vec<Segment>, CssError> {
    let mut segments = Vec::new();
    for child in node.children().iter() {
        push_child(context, &mut segments, child)?;
    }
    push_own_text(node, &mut segments);
    Ok(segments)
}

fn push_child(
    context: &LayoutContext<'_>,
    segments: &mut Vec<Segment>,
    child: SnapshotId,
) -> Result<(), CssError> {
    let styled = context.node(child)?;
    let display = display_of(styled);
    if display.is_none() {
        return Ok(());
    }
    if display == Display::Inline {
        push_inline(segments, child);
        return Ok(());
    }
    segments.push(Segment::Block(child));
    Ok(())
}

fn push_inline(segments: &mut Vec<Segment>, child: SnapshotId) {
    if let Some(Segment::Inline(items)) = segments.last_mut() {
        items.push(child);
        return;
    }
    segments.push(Segment::Inline(vec![child]));
}

fn push_own_text(node: &StyledNode, segments: &mut Vec<Segment>) {
    if !segments.is_empty() || node.text().is_none() {
        return;
    }
    segments.push(Segment::Inline(vec![node.node()]));
}

fn stack_segments(
    context: &LayoutContext<'_>,
    segments: &[Segment],
    content_width: Au,
    font_size: Au,
    align: TextAlign,
    input: BlockInput,
) -> Result<ContentFlow, CssError> {
    let mut stack = BlockStack::new();
    for segment in segments {
        stack.absorb(
            context,
            segment,
            Flowing::new(content_width, font_size, align, input),
        )?;
    }
    Ok(stack.finish())
}

/// The four values every segment of one block formatting context shares.
#[derive(Clone, Copy)]
struct Flowing {
    content_width: Au,
    font_size: Au,
    align: TextAlign,
    input: BlockInput,
}

impl Flowing {
    const fn new(content_width: Au, font_size: Au, align: TextAlign, input: BlockInput) -> Self {
        Self {
            content_width,
            font_size,
            align,
            input,
        }
    }

    const fn nested(self) -> BlockInput {
        self.input.nested(self.content_width, self.font_size)
    }
}

/// The running state of one block formatting context: where the next box goes,
/// which margins are still adjoining, and what has been placed so far.
struct BlockStack {
    cursor: Au,
    pending: CollapsedMargin,
    leading: Option<CollapsedMargin>,
    fragments: Fragments,
    flow: MarginFlow,
}

impl BlockStack {
    const fn new() -> Self {
        Self {
            cursor: Au::ZERO,
            pending: CollapsedMargin::ZERO,
            leading: None,
            fragments: Fragments::new(),
            flow: MarginFlow::CollapsesThrough,
        }
    }

    fn absorb(
        &mut self,
        context: &LayoutContext<'_>,
        segment: &Segment,
        flowing: Flowing,
    ) -> Result<(), CssError> {
        match segment {
            Segment::Block(child) => self.absorb_block(context, *child, flowing),
            Segment::Inline(items) => self.absorb_inline(context, items, flowing),
        }
    }

    fn absorb_block(
        &mut self,
        context: &LayoutContext<'_>,
        child: SnapshotId,
        flowing: Flowing,
    ) -> Result<(), CssError> {
        let result = layout_box(context, child, flowing.nested())?;
        self.pending = self.pending.adjoin(result.top_margin());
        self.hoist_leading();
        let vertical = self.cursor.saturating_add(self.pending.resolve());
        let edges = result.edges();
        let margin = edges.margin();
        self.place(result, margin.left(), vertical);
        Ok(())
    }

    fn absorb_inline(
        &mut self,
        context: &LayoutContext<'_>,
        items: &[SnapshotId],
        flowing: Flowing,
    ) -> Result<(), CssError> {
        let flow = inline::layout(
            context,
            items,
            flowing.content_width,
            flowing.font_size,
            flowing.align,
        )?;
        self.leading.get_or_insert(CollapsedMargin::ZERO);
        let vertical = self.cursor.saturating_add(self.pending.resolve());
        let height = flow.height();
        self.fragments
            .absorb(flow.into_fragments().translated(Au::ZERO, vertical));
        self.cursor = vertical.saturating_add(height);
        self.pending = CollapsedMargin::ZERO;
        self.flow = MarginFlow::Separated;
        Ok(())
    }

    /// The first in-flow child's top margin does not push it down inside this
    /// container: it escapes upward, and the parent decides whether it collapses
    /// with the container's own top margin.
    const fn hoist_leading(&mut self) {
        if self.leading.is_some() {
            return;
        }
        self.leading = Some(self.pending);
        self.pending = CollapsedMargin::ZERO;
    }

    fn place(&mut self, result: BlockResult, horizontal: Au, vertical: Au) {
        let height = result.height();
        let flow = result.flow();
        let bottom = result.bottom_margin();
        self.fragments
            .absorb(result.into_fragments().translated(horizontal, vertical));
        if flow.collapses_through() {
            return;
        }
        self.cursor = vertical.saturating_add(height);
        self.pending = bottom;
        self.flow = MarginFlow::Separated;
    }

    fn finish(self) -> ContentFlow {
        let leading = self.leading.unwrap_or(CollapsedMargin::ZERO);
        ContentFlow::new(self.cursor, self.fragments)
            .with_margins(leading, self.pending)
            .with_flow(self.flow)
    }
}
