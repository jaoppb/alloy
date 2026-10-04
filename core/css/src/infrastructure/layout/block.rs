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

use core::fmt;

use graphics::{Au, Point};

use crate::application::ports::{LayoutEngine, TextMeasurer};
use crate::domain::computed::display::Display;
use crate::domain::computed::inline_style::TextAlign;
use crate::domain::computed::intrinsic::IntrinsicSize;
use crate::domain::computed::logical::Direction;
use crate::domain::computed::position::PositionType;
use crate::domain::computed::sizing::Sizing;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::dom_snapshot::{ChildIds, SnapshotId};
use crate::domain::error::CssError;
use crate::domain::layout_box_tree::{LayoutBoxTree, LayoutBoxTreeBuilder};
use crate::domain::length::Length;
use crate::domain::styled_tree::{StyledNode, StyledTree};
use crate::domain::viewport::ViewportConstraints;
use crate::infrastructure::layout::box_model::{self, BoxMetrics, DEFAULT_FONT_SIZE};
use crate::infrastructure::layout::context::{
    BlockInput, BlockResult, BorderBoxSize, ContainingBlock, ContentFlow, FittedWidth,
    LayoutContext, MAX_LAYOUT_DEPTH,
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
pub struct BlockLayout<M = MonospaceMetrics> {
    measurer: M,
}

impl<M: TextMeasurer> BlockLayout<M> {
    /// A layout engine measuring text through `measurer` — received from parameter.
    #[must_use]
    pub const fn new(measurer: M) -> Self {
        Self { measurer }
    }
}

impl BlockLayout<MonospaceMetrics> {
    /// A layout engine measuring text with the deterministic
    /// [`MonospaceMetrics`].
    #[must_use]
    pub const fn monospace() -> Self {
        Self::new(MonospaceMetrics::new())
    }
}

impl<M: TextMeasurer + Default> Default for BlockLayout<M> {
    fn default() -> Self {
        Self::new(M::default())
    }
}

impl<M: TextMeasurer> fmt::Debug for BlockLayout<M> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BlockLayout")
    }
}

impl<M: TextMeasurer + Send + Sync> LayoutEngine for BlockLayout<M> {
    fn layout(
        &self,
        styled: &StyledTree,
        constraints: &ViewportConstraints,
    ) -> Result<LayoutBoxTree, CssError> {
        let context = LayoutContext::new(styled, &self.measurer);
        let root = styled.root();
        let mut builder = LayoutBoxTreeBuilder::new();
        if !generates_box(&context, root) {
            return Ok(builder.finish(None));
        }
        let viewport = ContainingBlock::new(constraints.width(), Some(constraints.height()));
        let input = BlockInput::new(viewport, DEFAULT_FONT_SIZE);
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

fn generates_box<M: TextMeasurer>(context: &LayoutContext<'_, M>, node: SnapshotId) -> bool {
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
pub(crate) fn layout_box<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
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
    let inner = ContainingBlock::new(content_width, definite_content_height(metrics, input));
    let children = layout_content(context, node, inner, font_size, input)?;
    let result = assemble(
        context,
        node,
        Resolved::new(metrics, content_width),
        children,
        input,
    )?;
    let direction = containing_direction(context, node);
    Ok(apply_relative_insets(
        style,
        RelativeFrame::new(font_size, direction, input),
        result,
    ))
}

/// Lays an atomic inline-level box (`display: inline-block`) out for a line.
///
/// An explicit `width` is used as-is. An `auto` width is shrink-to-fit
/// (CSS 2.1 §10.3.9), approximated in two passes: lay the box out at the
/// available width, measure the span its contents actually used, and lay it out
/// again at that span. Block-level descendants with an `auto` width fill
/// whatever they are given, so a box containing one stays at the available
/// width — the missing intrinsic-sizing pass `flex.rs` also declares.
///
/// The measured width is remembered per node ([`LayoutContext::remembered_fit`]):
/// without that, every nesting level would lay its whole subtree out twice and
/// `n` nested inline-blocks would cost `2^n` layouts.
pub(crate) fn layout_inline_block<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    input: BlockInput,
) -> Result<BlockResult, CssError> {
    let style = context.node(node_id)?.style();
    if style.width() != Sizing::Auto {
        return layout_box(context, node_id, input);
    }
    let available = available_content_width(style, input)?;
    let Some(fitted) = context.remembered_fit(node_id, available) else {
        return measure_and_fit(context, node_id, input, available);
    };
    layout_box(context, node_id, input.with_forced_content_width(fitted))
}

/// The content width an `auto`-width box would get from its containing block.
fn available_content_width(style: &ComputedStyle, input: BlockInput) -> Result<Au, CssError> {
    let font_size = box_model::font_size_of(style, input.parent_font_size());
    let metrics = box_model::resolve(style, font_size, input.containing_width())?;
    Ok(metrics.content_width_within(input.containing_width()))
}

/// The first, measuring pass of shrink-to-fit — and the final one too when the
/// contents already need the whole available width.
fn measure_and_fit<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    input: BlockInput,
    available: Au,
) -> Result<BlockResult, CssError> {
    let measured = layout_box(context, node_id, input)?;
    let fitted = measured.fitted_content_width();
    context.remember_fit(node_id, FittedWidth::new(available, fitted));
    if fitted == available {
        return Ok(measured);
    }
    layout_box(context, node_id, input.with_forced_content_width(fitted))
}

/// The content height this box's children may resolve percentages against: a
/// forced height (a flex item) or a declared one, never the height its
/// content will produce (CSS 2.1 §10.5).
fn definite_content_height(metrics: BoxMetrics, input: BlockInput) -> Option<Au> {
    input.forced_content_height().or_else(|| metrics.height())
}

/// The `direction` of `node`'s containing block — its parent's — which decides
/// the horizontal inset of `position: relative` (CSS 2.1 §9.4.3). The root,
/// with no styled parent, falls back to its own.
fn containing_direction<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node: &StyledNode,
) -> Direction {
    let parent_style = node
        .parent()
        .and_then(|parent| context.node(parent).ok())
        .map_or_else(|| node.style(), StyledNode::style);
    parent_style.logical().context().direction()
}

/// What a relative offset is measured against: the box's font size (for
/// `em`), its containing block's direction and its containing block's size.
#[derive(Clone, Copy)]
struct RelativeFrame {
    font_size: Au,
    direction: Direction,
    input: BlockInput,
}

impl RelativeFrame {
    const fn new(font_size: Au, direction: Direction, input: BlockInput) -> Self {
        Self {
            font_size,
            direction,
            input,
        }
    }

    /// Which horizontal inset wins when both are set: `left` in `ltr`,
    /// `right` in `rtl` (CSS 2.1 §9.4.3).
    const fn horizontal_precedence(self) -> InsetPrecedence {
        match self.direction {
            Direction::Rtl => InsetPrecedence::EndWins,
            Direction::Ltr => InsetPrecedence::StartWins,
        }
    }
}

/// Applies the visual offset of `position: relative` to a laid-out result.
///
/// A relatively-positioned box keeps its normal-flow position for purposes of
/// margin collapse and sibling layout; only its paint rect shifts (CSS 2.1
/// §9.4.3, CSS Positioned Layout L3 §3.4). When every inset is `auto` — the
/// common case — this is a no-op.
fn apply_relative_insets(
    style: &ComputedStyle,
    frame: RelativeFrame,
    result: BlockResult,
) -> BlockResult {
    let position = style.position();
    if position.position() != PositionType::Relative {
        return result;
    }
    let horizontal = Opposed::new(position.left(), position.right());
    let dx = relative_offset(
        horizontal,
        frame.horizontal_precedence(),
        frame.font_size,
        Some(frame.input.containing_width()),
    );
    let vertical = Opposed::new(position.top(), position.bottom());
    let dy = relative_offset(
        vertical,
        InsetPrecedence::StartWins,
        frame.font_size,
        frame.input.containing_height(),
    );
    result.with_relative_offset(dx, dy)
}

/// The two insets of one axis: `left`/`right` or `top`/`bottom`.
#[derive(Clone, Copy)]
struct Opposed {
    start: Sizing,
    end: Sizing,
}

impl Opposed {
    const fn new(start: Sizing, end: Sizing) -> Self {
        Self { start, end }
    }
}

/// Which of two non-`auto` opposing insets decides the offset.
#[derive(Clone, Copy)]
enum InsetPrecedence {
    /// `left` / `top`: always so vertically, and horizontally in `ltr`.
    StartWins,
    /// `right`: horizontally in `rtl`.
    EndWins,
}

/// CSS 2.1 §9.4.3: the start inset (`left` / `top`) moves the box by its own
/// value and the end inset (`right` / `bottom`) by minus its value; when only
/// one is set it decides, when both are set `precedence` does, and both
/// `auto` leaves the box in place.
fn relative_offset(
    insets: Opposed,
    precedence: InsetPrecedence,
    font_size: Au,
    basis: Option<Au>,
) -> Au {
    let start = resolve_inset(insets.start, font_size, basis);
    let end =
        resolve_inset(insets.end, font_size, basis).map(|offset| Au::ZERO.saturating_sub(offset));
    let offset = match precedence {
        InsetPrecedence::StartWins => start.or(end),
        InsetPrecedence::EndWins => end.or(start),
    };
    offset.unwrap_or(Au::ZERO)
}

/// Resolves one inset to an [`Au`] offset, or `None` when it behaves as `auto`.
///
/// A percentage resolves against `basis` — the containing block's width for
/// `left` / `right`, its height for `top` / `bottom` (CSS 2.1 §9.3.2) — and
/// against an indefinite height it computes to `auto`.
fn resolve_inset(sizing: Sizing, font_size: Au, basis: Option<Au>) -> Option<Au> {
    let Sizing::Fixed(length) = sizing else {
        return None;
    };
    let reference = percentage_basis(length, basis)?;
    length.resolve_to_au(font_size, reference)
}

/// What a length's percentage is measured against: `basis` itself, which a
/// percentage needs to be definite; any other length ignores it.
fn percentage_basis(length: Length, basis: Option<Au>) -> Option<Au> {
    if length.is_percentage() {
        return basis;
    }
    Some(basis.unwrap_or(Au::ZERO))
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
fn assemble<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
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
    let declared = definite_content_height(metrics, input);
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
fn assemble_fragments<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
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

fn box_generating_children<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
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

/// One stretch of a block container's children: a run of inline-level boxes
/// forming an anonymous block, one block-level box, or — for a childless node
/// carrying text of its own (a text node blockified by a flex container, an
/// `<input>` label) — that text as the container's only line content.
enum Segment {
    Inline(Vec<SnapshotId>),
    Block(SnapshotId),
    OwnText(SnapshotId),
}

/// The children of `node`, laid out inside its content box `inner`.
fn layout_content<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node: &StyledNode,
    inner: ContainingBlock,
    font_size: Au,
    input: BlockInput,
) -> Result<ContentFlow, CssError> {
    if display_of(node) == Display::Flex {
        return flex::layout(context, node, inner, font_size, input);
    }
    let segments = segments_of(context, node)?;
    if segments.is_empty() {
        return Ok(ContentFlow::empty());
    }
    let align = node.style().text_align();
    stack_segments(
        context,
        &segments,
        Flowing::new(inner, font_size, align, input),
    )
}

/// Splits the in-flow children into runs of inline-level boxes and single
/// block-level boxes, in document order. A node that is itself a text node
/// contributes itself — that is how a text node blockified by a flex container
/// still gets a line box.
fn segments_of<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node: &StyledNode,
) -> Result<Vec<Segment>, CssError> {
    let mut segments = Vec::new();
    for child in node.children().iter() {
        push_child(context, &mut segments, child)?;
    }
    push_own_text(node, &mut segments);
    Ok(segments)
}

fn push_child<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    segments: &mut Vec<Segment>,
    child: SnapshotId,
) -> Result<(), CssError> {
    let styled = context.node(child)?;
    let display = display_of(styled);
    if display.is_none() {
        return Ok(());
    }
    if display.is_inline_level() {
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
    segments.push(Segment::OwnText(node.node()));
}

fn stack_segments<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    segments: &[Segment],
    flowing: Flowing,
) -> Result<ContentFlow, CssError> {
    let mut stack = BlockStack::new();
    for segment in segments {
        stack.absorb(context, segment, flowing)?;
    }
    Ok(stack.finish())
}

/// The four values every segment of one block formatting context shares.
#[derive(Clone, Copy)]
struct Flowing {
    inner: ContainingBlock,
    font_size: Au,
    align: TextAlign,
    input: BlockInput,
}

impl Flowing {
    const fn new(
        inner: ContainingBlock,
        font_size: Au,
        align: TextAlign,
        input: BlockInput,
    ) -> Self {
        Self {
            inner,
            font_size,
            align,
            input,
        }
    }

    /// The input every child of this container is laid out with: this
    /// container's content box as its containing block, one level deeper.
    const fn nested(self) -> BlockInput {
        self.input.nested(self.inner, self.font_size)
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

    fn absorb<M: TextMeasurer>(
        &mut self,
        context: &LayoutContext<'_, M>,
        segment: &Segment,
        flowing: Flowing,
    ) -> Result<(), CssError> {
        let lines = match segment {
            Segment::Block(child) => return self.absorb_block(context, *child, flowing),
            Segment::Inline(items) => {
                inline::layout(context, items, flowing.nested(), flowing.align)?
            }
            Segment::OwnText(node) => {
                inline::layout_own_text(context, *node, flowing.nested(), flowing.align)?
            }
        };
        self.absorb_lines(lines);
        Ok(())
    }

    fn absorb_block<M: TextMeasurer>(
        &mut self,
        context: &LayoutContext<'_, M>,
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

    /// Places one anonymous block of line boxes: it separates the margins on
    /// either side of it, so nothing collapses through.
    fn absorb_lines(&mut self, flow: ContentFlow) {
        self.leading.get_or_insert(CollapsedMargin::ZERO);
        let vertical = self.cursor.saturating_add(self.pending.resolve());
        let height = flow.height();
        self.fragments
            .absorb(flow.into_fragments().translated(Au::ZERO, vertical));
        self.cursor = vertical.saturating_add(height);
        self.pending = CollapsedMargin::ZERO;
        self.flow = MarginFlow::Separated;
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
