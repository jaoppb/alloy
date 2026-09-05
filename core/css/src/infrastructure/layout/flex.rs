//! Flexbox (CSS Flexbox L1 §9, v0.5 B4) — the formatting context a
//! `display: flex` container hands its in-flow children to.
//!
//! Every item is laid out through [`crate::infrastructure::layout::block::layout_box`]
//! exactly like a normal block child — a flex container never grows a second
//! recursive layout algorithm, it only decides **what containing size** to
//! force on each child before asking for its box (CSS Flexbox L1 §9.2's
//! "used main size" and stretch's "used cross size" are both, mechanically,
//! just a forced content width or height).
//!
//! Three simplifications are declared here and belong as lines in
//! `core/css/tests/data/MANIFEST.md`, pre-approved by the v0.5 B4 handoff as an
//! explicit relief valve rather than a silent cut:
//!
//! 1. Wrapping (`flex-wrap: wrap` / `wrap-reverse`) only breaks lines when the
//!    main axis has a definite size to overflow against — an auto-sized
//!    `column` container (no declared/forced height) never wraps, since there
//!    is no limit to overflow. This is a single-line engine for that one case,
//!    not a general multi-line algorithm.
//! 2. `align-items: baseline` / `align-self: baseline` behave like
//!    `flex-start` — no first-baseline-of-line algorithm is implemented.
//! 3. An item with both `flex-basis: auto` **and** an `auto` value for its own
//!    main-size property (`width` for a `row` container, `height` for
//!    `column`) gets a hypothetical main size of zero rather than a real
//!    content-based (shrink-to-fit) measurement — there is no intrinsic-sizing
//!    pass in this engine yet. An item relying on this combination should set
//!    an explicit `flex-basis`, `width`/`height`, or lean on `flex-grow`.
//! 4. For a `column` container, an item with a non-`stretch` effective
//!    alignment but an `auto` own width still fills the cross axis exactly as
//!    `stretch` would — the shrink-to-fit measurement `align-items: flex-start`
//!    would need on the cross axis is the same missing pass as (3). An item
//!    that declares an explicit `width` **is** positioned correctly within the
//!    leftover cross space.

use graphics::{Au, Point, Px};

use crate::domain::computed::flex::{
    AlignContent, AlignItems, FlexDirection, FlexFactor, FlexStyle, FlexWrap, JustifyContent,
};
use crate::domain::computed::sizing::{BoxSizing, Sizing};
use crate::domain::computed::style::ComputedStyle;
use crate::domain::dom_snapshot::SnapshotId;
use crate::domain::error::CssError;
use crate::domain::layout_box_tree::EdgeSizes;
use crate::domain::styled_tree::StyledNode;
use crate::infrastructure::layout::block::layout_box;
use crate::infrastructure::layout::box_model::{self, BoxMetrics};
use crate::infrastructure::layout::context::{BlockInput, BlockResult, ContentFlow, LayoutContext};
use crate::infrastructure::layout::fragment::Fragments;

/// Lays a `display: flex` container's in-flow children out inside a content
/// box `content_width` wide.
pub fn layout(
    context: &LayoutContext<'_>,
    node: &StyledNode,
    content_width: Au,
    font_size: Au,
    input: BlockInput,
) -> Result<ContentFlow, CssError> {
    let style = node.style();
    let flex = style.flex();
    let axis = Axis::new(flex.direction());
    let container_metrics = box_model::resolve(style, font_size, input.containing_width())?;
    let (main_available, cross_available) =
        container_axes(axis, content_width, container_metrics, input);

    let flex_ctx = FlexContext {
        context,
        font_size,
        content_width,
        input,
        axis,
        justify: flex.justify_content(),
        align_items: flex.align_items(),
        main_available,
    };

    let child_ids = in_flow_children(context, node)?;
    if child_ids.is_empty() {
        return Ok(ContentFlow::empty());
    }

    let items = resolve_items(flex_ctx, &child_ids)?;
    let lines = pack_lines(&items, main_available, flex.wrap());
    let mut pass_a = run_pass_a(flex_ctx, &lines)?;
    if flex.wrap().is_reversed() {
        pass_a.reverse();
    }

    let natural_cross: Vec<Au> = pass_a.iter().map(|line| line.cross_size).collect();
    let natural_cross_total = natural_cross
        .iter()
        .fold(Au::ZERO, |sum, size| sum.saturating_add(*size));
    let resolved_cross_total = cross_available.unwrap_or(natural_cross_total);
    // CSS Flexbox L1 §9.4: a single-line container's one line is always
    // exactly the container's own cross size — `align-content` only
    // redistributes leftover space when there are two or more lines.
    let (cross_start, cross_sizes, cross_gaps) = if pass_a.len() <= 1 {
        (Au::ZERO, vec![resolved_cross_total], Vec::new())
    } else {
        distribute_align_content(flex.align_content(), resolved_cross_total, &natural_cross)
    };
    let resolved_main_extent = main_available.unwrap_or_else(|| {
        pass_a
            .iter()
            .fold(Au::ZERO, |acc, line| acc.larger(line.main_extent))
    });

    let fragments = emit_lines(
        flex_ctx,
        pass_a,
        cross_start,
        cross_sizes,
        cross_gaps,
        resolved_main_extent,
    )?;

    let content_flow_height = if axis.is_row() {
        resolved_cross_total
    } else {
        resolved_main_extent
    };
    Ok(ContentFlow::new(content_flow_height, fragments))
}

/// The container's own main-axis and cross-axis available sizes: the main
/// axis of a `row` container and the cross axis of a `column` one are always
/// the definite `content_width` this engine already resolved; the other axis
/// is definite only when a height was declared or forced (a flex item being
/// stretched by an ancestor flex container).
fn container_axes(
    axis: Axis,
    content_width: Au,
    container_metrics: BoxMetrics,
    input: BlockInput,
) -> (Option<Au>, Option<Au>) {
    let height = input
        .forced_content_height()
        .or_else(|| container_metrics.height());
    if axis.is_row() {
        (Some(content_width), height)
    } else {
        (height, Some(content_width))
    }
}

fn in_flow_children(
    context: &LayoutContext<'_>,
    node: &StyledNode,
) -> Result<Vec<SnapshotId>, CssError> {
    let mut kept = Vec::new();
    for child in node.children().iter() {
        let styled = context.node(child)?;
        if !styled.style().display().is_none() {
            kept.push(child);
        }
    }
    Ok(kept)
}

// ---- the direction abstraction --------------------------------------------

/// The one place `row` vs. `column` branches: everything else in this file
/// asks `Axis` which physical dimension is "main" and which is "cross".
#[derive(Clone, Copy)]
struct Axis {
    direction: FlexDirection,
}

impl Axis {
    const fn new(direction: FlexDirection) -> Self {
        Self { direction }
    }

    const fn is_row(self) -> bool {
        self.direction.is_horizontal()
    }

    const fn is_reversed(self) -> bool {
        self.direction.is_reversed()
    }

    const fn inner_main(self, metrics: BoxMetrics) -> Au {
        if self.is_row() {
            metrics.inner_horizontal()
        } else {
            metrics.inner_vertical()
        }
    }

    const fn inner_cross(self, metrics: BoxMetrics) -> Au {
        if self.is_row() {
            metrics.inner_vertical()
        } else {
            metrics.inner_horizontal()
        }
    }

    const fn margin_main(self, margin: EdgeSizes) -> Au {
        if self.is_row() {
            margin.horizontal()
        } else {
            margin.vertical()
        }
    }

    /// Overrides an item's main-axis containing size — the mechanism every
    /// flex item's used main size (post flex-basis/grow/shrink) is applied
    /// through, in place of whatever `width`/`height` it declared.
    const fn force_main(self, input: BlockInput, size: Au) -> BlockInput {
        if self.is_row() {
            input.with_forced_content_width(size)
        } else {
            input.with_forced_content_height(size)
        }
    }

    const fn outer_cross(self, result: &BlockResult) -> Au {
        if self.is_row() {
            result.outer_height()
        } else {
            result.outer_width()
        }
    }

    const fn point(self, main: Au, cross: Au) -> Point {
        if self.is_row() {
            Point::new(main, cross)
        } else {
            Point::new(cross, main)
        }
    }
}

/// Everything every step of this file reads and never mutates: the ambient
/// layout inputs plus the three container-level Flexbox properties that apply
/// to every item and every line alike.
#[derive(Clone, Copy)]
struct FlexContext<'tree> {
    context: &'tree LayoutContext<'tree>,
    font_size: Au,
    content_width: Au,
    input: BlockInput,
    axis: Axis,
    justify: JustifyContent,
    align_items: AlignItems,
    main_available: Option<Au>,
}

// ---- item resolution: flex-basis (CSS Flexbox L1 §9.2) --------------------

/// One item's own style and its hypothetical (pre-grow/shrink) main size.
#[derive(Clone, Copy)]
struct ResolvedItem {
    id: SnapshotId,
    flex: FlexStyle,
    metrics: BoxMetrics,
    basis: Au,
    outer_basis: Au,
}

fn resolve_items(
    flex_ctx: FlexContext<'_>,
    child_ids: &[SnapshotId],
) -> Result<Vec<ResolvedItem>, CssError> {
    let mut items = Vec::with_capacity(child_ids.len());
    for id in child_ids {
        items.push(resolve_item(flex_ctx, *id)?);
    }
    Ok(items)
}

fn resolve_item(flex_ctx: FlexContext<'_>, id: SnapshotId) -> Result<ResolvedItem, CssError> {
    let styled = flex_ctx.context.node(id)?;
    let style = styled.style();
    let font_size = box_model::font_size_of(style, flex_ctx.font_size);
    let metrics = box_model::resolve(style, font_size, flex_ctx.content_width)?;
    let basis = hypothetical_main_size(
        style,
        metrics,
        font_size,
        flex_ctx.axis,
        flex_ctx.main_available,
    );
    let inner_main = flex_ctx.axis.inner_main(metrics);
    let margin_main = flex_ctx.axis.margin_main(metrics.edges().margin());
    let outer_basis = basis.saturating_add(inner_main).saturating_add(margin_main);
    Ok(ResolvedItem {
        id,
        flex: style.flex(),
        metrics,
        basis,
        outer_basis,
    })
}

/// CSS Flexbox L1 §9.2 step 3: a fixed `flex-basis` wins; otherwise the
/// item's own main-size property; otherwise (simplification 3, module doc)
/// zero.
fn hypothetical_main_size(
    style: &ComputedStyle,
    metrics: BoxMetrics,
    font_size: Au,
    axis: Axis,
    main_available: Option<Au>,
) -> Au {
    let flex = style.flex();
    if let Some(declared) = resolve_basis_length(flex.basis(), font_size, main_available) {
        return box_sizing_adjust(style, metrics, declared, axis);
    }
    let own_main = if axis.is_row() {
        metrics.width()
    } else {
        metrics.height()
    };
    own_main.unwrap_or(Au::ZERO)
}

fn resolve_basis_length(basis: Sizing, font_size: Au, main_available: Option<Au>) -> Option<Au> {
    let Sizing::Fixed(length) = basis else {
        return None;
    };
    if length.is_percentage() {
        let reference = main_available?;
        return length.resolve_to_au(font_size, reference);
    }
    length.resolve_to_au(font_size, Au::ZERO)
}

/// A `border-box`-sized `flex-basis` measures the border box, same as `width`
/// (CSS Box Sizing L3 §5) — the content size is what the rest of this file
/// works in.
fn box_sizing_adjust(style: &ComputedStyle, metrics: BoxMetrics, declared: Au, axis: Axis) -> Au {
    if style.box_sizing() == BoxSizing::ContentBox {
        return declared;
    }
    declared
        .saturating_sub(axis.inner_main(metrics))
        .larger(Au::ZERO)
}

// ---- line packing (CSS Flexbox L1 §9.3) ------------------------------------

/// Greedily packs items into lines when wrapping is on **and** the main axis
/// has a definite size to overflow against (module doc, simplification 1);
/// otherwise every item shares one line.
fn pack_lines(
    items: &[ResolvedItem],
    main_available: Option<Au>,
    wrap: FlexWrap,
) -> Vec<Vec<ResolvedItem>> {
    let limit = main_available.filter(|_| wrap.wraps());
    let Some(limit) = limit else {
        return vec![items.to_vec()];
    };
    let mut lines: Vec<Vec<ResolvedItem>> = Vec::new();
    let mut current: Vec<ResolvedItem> = Vec::new();
    let mut current_main = Au::ZERO;
    for item in items {
        if !current.is_empty() && current_main.saturating_add(item.outer_basis) > limit {
            lines.push(core::mem::take(&mut current));
            current_main = Au::ZERO;
        }
        current_main = current_main.saturating_add(item.outer_basis);
        current.push(*item);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

// ---- pass A: resolve each line's items and lay them out at their natural
// (unstretched) cross size, to learn every line's natural cross size --------

struct Entry {
    item: ResolvedItem,
    main_size: Au,
    border_main: Au,
    main_origin: Au,
    result: BlockResult,
}

struct LineLayout {
    entries: Vec<Entry>,
    cross_size: Au,
    main_extent: Au,
}

fn run_pass_a(
    flex_ctx: FlexContext<'_>,
    lines: &[Vec<ResolvedItem>],
) -> Result<Vec<LineLayout>, CssError> {
    let mut laid_out = Vec::with_capacity(lines.len());
    for line in lines {
        laid_out.push(layout_line_pass_a(flex_ctx, line)?);
    }
    Ok(laid_out)
}

fn layout_line_pass_a(
    flex_ctx: FlexContext<'_>,
    line: &[ResolvedItem],
) -> Result<LineLayout, CssError> {
    let (resolved_mains, leftover) = resolve_main_sizes(line, flex_ctx.main_available);
    let positions = abstract_positions(
        line,
        &resolved_mains,
        flex_ctx.axis,
        flex_ctx.justify,
        leftover,
    );
    let combined: Vec<(ResolvedItem, Au, Au, Au)> = line
        .iter()
        .copied()
        .zip(resolved_mains)
        .zip(positions)
        .map(|((item, main_size), (main_origin, border_main))| {
            (item, main_size, main_origin, border_main)
        })
        .collect();

    let mut entries = Vec::with_capacity(combined.len());
    let mut cross_size = Au::ZERO;
    let mut main_extent = Au::ZERO;
    for (item, main_size, main_origin, border_main) in combined {
        let child_input = flex_ctx.axis.force_main(
            flex_ctx
                .input
                .nested(flex_ctx.content_width, flex_ctx.font_size),
            main_size,
        );
        let result = layout_box(flex_ctx.context, item.id, child_input)?;
        cross_size = cross_size.larger(flex_ctx.axis.outer_cross(&result));
        main_extent = main_extent.larger(main_origin.saturating_add(border_main));
        entries.push(Entry {
            item,
            main_size,
            border_main,
            main_origin,
            result,
        });
    }

    Ok(LineLayout {
        entries,
        cross_size: if flex_ctx.axis.is_row() {
            cross_size
        } else {
            flex_ctx.content_width
        },
        main_extent,
    })
}

// ---- main-axis distribution: flex-grow / flex-shrink (CSS Flexbox L1 §9.7) -

/// Resolves every item's used main size, and reports the main-axis space
/// `justify-content` still has to place (zero unless every flex factor in the
/// line is zero).
fn resolve_main_sizes(line: &[ResolvedItem], main_available: Option<Au>) -> (Vec<Au>, Au) {
    let bases: Vec<Au> = line.iter().map(|item| item.basis).collect();
    let Some(avail) = main_available else {
        return (bases, Au::ZERO);
    };
    let used = line
        .iter()
        .fold(Au::ZERO, |sum, item| sum.saturating_add(item.outer_basis));
    let free = avail.saturating_sub(used);
    if free.is_zero() {
        return (bases, Au::ZERO);
    }
    if free.is_negative() {
        let weights: Vec<i64> = line.iter().map(shrink_weight).collect();
        let total = weights
            .iter()
            .fold(0_i64, |acc, weight| acc.saturating_add(*weight));
        if total <= 0 {
            return (bases, free);
        }
        return (apply_deltas(&bases, &distribute(free, &weights)), Au::ZERO);
    }
    let weights: Vec<i64> = line
        .iter()
        .map(|item| factor_weight_raw(item.flex.grow()))
        .collect();
    let total = weights
        .iter()
        .fold(0_i64, |acc, weight| acc.saturating_add(*weight));
    if total <= 0 {
        return (bases, free);
    }
    (apply_deltas(&bases, &distribute(free, &weights)), Au::ZERO)
}

fn apply_deltas(bases: &[Au], deltas: &[Au]) -> Vec<Au> {
    bases
        .iter()
        .zip(deltas)
        .map(|(base, delta)| base.saturating_add(*delta).larger(Au::ZERO))
        .collect()
}

/// `flex-shrink`'s weight is scaled by the item's own basis (CSS Flexbox L1
/// §9.7.4b) — a bigger item shrinks more for the same factor.
fn shrink_weight(item: &ResolvedItem) -> i64 {
    let factor = factor_weight_raw(item.flex.shrink());
    let basis = i64::from(item.basis.raw().max(0));
    factor.saturating_mul(basis)
}

/// The scale a `FlexFactor` is turned into an integer weight at, before any
/// further arithmetic touches an [`Au`] (`ADR-0016`).
const WEIGHT_SCALE: f32 = 1000.0;

/// The one place a dimensionless `FlexFactor` crosses into `Au`-flavoured
/// integer arithmetic — through [`Au::from_px`], the crate's one sanctioned
/// float-to-fixed-point crossing, the same way [`crate::domain::length::Length::resolve_to_au`]
/// turns a declared magnitude into geometry. Everything downstream of this
/// call (`distribute`) works in plain integer numerator/denominator division.
fn factor_weight_raw(factor: FlexFactor) -> i64 {
    let scaled = Au::from_px(Px::new(factor.value() * WEIGHT_SCALE)).unwrap_or(Au::ZERO);
    i64::from(scaled.raw())
}

// ---- justify-content: main-axis placement within a line (CSS Flexbox L1 §9.4) --

/// Where every item's border box starts along the main axis, in **abstract**
/// start-to-end coordinates (main-start is always `0`, regardless of
/// direction) — [`mirror_main_origin`] flips this into physical space for a
/// `*-reverse` direction once the container's main extent is known.
fn abstract_positions(
    line: &[ResolvedItem],
    resolved_mains: &[Au],
    axis: Axis,
    justify: JustifyContent,
    leftover: Au,
) -> Vec<(Au, Au)> {
    let count = line.len();
    let (start, gaps) = justify_offsets(justify, leftover, count);
    let mut cursor = start;
    let mut gap_iter = gaps.into_iter();
    let mut placed = Vec::with_capacity(count);
    for (item, &main_size) in line.iter().zip(resolved_mains) {
        let margin = item.metrics.edges().margin();
        let (leading, trailing) = if axis.is_row() {
            (margin.left(), margin.right())
        } else {
            (margin.top(), margin.bottom())
        };
        let border_origin = cursor.saturating_add(leading);
        let border_main = main_size.saturating_add(axis.inner_main(item.metrics));
        placed.push((border_origin, border_main));
        let outer = leading.saturating_add(border_main).saturating_add(trailing);
        cursor = cursor.saturating_add(outer);
        if let Some(gap) = gap_iter.next() {
            cursor = cursor.saturating_add(gap);
        }
    }
    placed
}

fn justify_offsets(justify: JustifyContent, leftover: Au, count: usize) -> (Au, Vec<Au>) {
    let no_gaps = vec![Au::ZERO; count.saturating_sub(1)];
    if leftover.is_zero() || count == 0 {
        return (Au::ZERO, no_gaps);
    }
    match justify {
        JustifyContent::FlexStart => (Au::ZERO, no_gaps),
        JustifyContent::FlexEnd => (leftover, no_gaps),
        JustifyContent::Center => (center_share(leftover), no_gaps),
        JustifyContent::SpaceBetween => space_between(leftover, count, no_gaps),
        JustifyContent::SpaceAround => {
            slots_to_start_and_gaps(leftover, &around_weights(count), count)
        }
        JustifyContent::SpaceEvenly => {
            slots_to_start_and_gaps(leftover, &vec![1_i64; count.saturating_add(1)], count)
        }
    }
}

fn space_between(leftover: Au, count: usize, no_gaps: Vec<Au>) -> (Au, Vec<Au>) {
    if count < 2 {
        return (Au::ZERO, no_gaps);
    }
    let weights = vec![1_i64; count.saturating_sub(1)];
    (Au::ZERO, distribute(leftover, &weights))
}

fn center_share(leftover: Au) -> Au {
    distribute(leftover, &[1, 1])
        .first()
        .copied()
        .unwrap_or(Au::ZERO)
}

fn around_weights(count: usize) -> Vec<i64> {
    let mut weights = vec![2_i64; count.saturating_add(1)];
    set_ends(&mut weights, 1);
    weights
}

const fn set_ends(weights: &mut [i64], value: i64) {
    if let Some(first) = weights.first_mut() {
        *first = value;
    }
    if let Some(last) = weights.last_mut() {
        *last = value;
    }
}

/// Splits `leftover` over `count + 1` slots (the space before the first item,
/// between every pair, and after the last) and reports the first slot plus the
/// `count - 1` between-item ones — the trailing slot is empty space nothing
/// needs to know the size of.
fn slots_to_start_and_gaps(leftover: Au, weights: &[i64], count: usize) -> (Au, Vec<Au>) {
    let shares = distribute(leftover, weights);
    let start = shares.first().copied().unwrap_or(Au::ZERO);
    let gaps = shares.get(1..count).map(<[Au]>::to_vec).unwrap_or_default();
    (start, gaps)
}

/// Flips an abstract (always start-to-end) main-axis origin into physical
/// space for `row-reverse` / `column-reverse`.
const fn mirror_main_origin(axis: Axis, main_extent: Au, origin: Au, border_size: Au) -> Au {
    if !axis.is_reversed() {
        return origin;
    }
    main_extent
        .saturating_sub(origin)
        .saturating_sub(border_size)
}

// ---- cross-axis placement: align-items / align-self (CSS Flexbox L1 §9.6) -

fn emit_lines(
    flex_ctx: FlexContext<'_>,
    lines: Vec<LineLayout>,
    cross_start: Au,
    cross_sizes: Vec<Au>,
    cross_gaps: Vec<Au>,
    main_extent: Au,
) -> Result<Fragments, CssError> {
    let mut fragments = Fragments::new();
    let mut cursor = cross_start;
    let mut gaps = cross_gaps.into_iter();
    for (line, cross_size) in lines.into_iter().zip(cross_sizes) {
        place_line(
            flex_ctx,
            &mut fragments,
            line,
            cursor,
            cross_size,
            main_extent,
        )?;
        cursor = cursor.saturating_add(cross_size);
        if let Some(gap) = gaps.next() {
            cursor = cursor.saturating_add(gap);
        }
    }
    Ok(fragments)
}

fn place_line(
    flex_ctx: FlexContext<'_>,
    fragments: &mut Fragments,
    line: LineLayout,
    cross_start: Au,
    cross_size: Au,
    main_extent: Au,
) -> Result<(), CssError> {
    let axis = flex_ctx.axis;
    for entry in line.entries {
        let effective_align = entry.item.flex.align_self().resolve(flex_ctx.align_items);
        let margin = entry.item.metrics.edges().margin();
        let (leading_cross, trailing_cross) = if axis.is_row() {
            (margin.top(), margin.bottom())
        } else {
            (margin.left(), margin.right())
        };
        // Stretch only overrides an item's cross size when that item's own
        // cross-size property (`height`, on a row) is `auto` — an explicit
        // height wins over stretch (CSS Flexbox L1 §8.3).
        let stretch = axis.is_row()
            && matches!(effective_align, AlignItems::Stretch)
            && entry.item.metrics.height().is_none();
        let (result, cross_offset_value) = if stretch {
            let target = cross_size
                .saturating_sub(leading_cross)
                .saturating_sub(trailing_cross)
                .saturating_sub(axis.inner_cross(entry.item.metrics))
                .larger(Au::ZERO);
            let child_input = flex_ctx
                .input
                .nested(flex_ctx.content_width, flex_ctx.font_size)
                .with_forced_content_width(entry.main_size)
                .with_forced_content_height(target);
            (
                layout_box(flex_ctx.context, entry.item.id, child_input)?,
                Au::ZERO,
            )
        } else {
            let free_cross = cross_size.saturating_sub(axis.outer_cross(&entry.result));
            (entry.result, cross_offset(effective_align, free_cross))
        };
        let main_origin =
            mirror_main_origin(axis, main_extent, entry.main_origin, entry.border_main);
        let cross_origin = cross_start
            .saturating_add(leading_cross)
            .saturating_add(cross_offset_value);
        let point = axis.point(main_origin, cross_origin);
        fragments.absorb(
            result
                .into_fragments()
                .translated(point.horizontal(), point.vertical()),
        );
    }
    Ok(())
}

fn cross_offset(align: AlignItems, free_cross: Au) -> Au {
    match align {
        AlignItems::FlexEnd => free_cross,
        AlignItems::Center => halve(free_cross),
        AlignItems::FlexStart | AlignItems::Stretch | AlignItems::Baseline => Au::ZERO,
    }
}

fn halve(value: Au) -> Au {
    Au::from_raw(value.raw().checked_div(2).unwrap_or(0))
}

// ---- align-content: cross-axis placement of the lines themselves (CSS Flexbox L1 §9.6) --

/// Distributes the container's cross size over its lines. Per spec this only
/// ever has an effect with two or more lines — the early return below is that
/// rule, not an extra simplification.
fn distribute_align_content(
    align_content: AlignContent,
    resolved_cross_total: Au,
    natural_sizes: &[Au],
) -> (Au, Vec<Au>, Vec<Au>) {
    let count = natural_sizes.len();
    let natural_sum = natural_sizes
        .iter()
        .fold(Au::ZERO, |sum, size| sum.saturating_add(*size));
    let leftover = resolved_cross_total.saturating_sub(natural_sum);
    let no_gaps = vec![Au::ZERO; count.saturating_sub(1)];
    if count <= 1 || leftover.is_zero() {
        return (Au::ZERO, natural_sizes.to_vec(), no_gaps);
    }
    match align_content {
        AlignContent::Stretch => (Au::ZERO, stretch_lines(natural_sizes, leftover), no_gaps),
        AlignContent::FlexStart => (Au::ZERO, natural_sizes.to_vec(), no_gaps),
        AlignContent::FlexEnd => (leftover, natural_sizes.to_vec(), no_gaps),
        AlignContent::Center => (center_share(leftover), natural_sizes.to_vec(), no_gaps),
        AlignContent::SpaceBetween => {
            let weights = vec![1_i64; count.saturating_sub(1)];
            (
                Au::ZERO,
                natural_sizes.to_vec(),
                distribute(leftover, &weights),
            )
        }
        AlignContent::SpaceAround => {
            let (start, gaps) = slots_to_start_and_gaps(leftover, &around_weights(count), count);
            (start, natural_sizes.to_vec(), gaps)
        }
    }
}

fn stretch_lines(natural_sizes: &[Au], leftover: Au) -> Vec<Au> {
    let weights = vec![1_i64; natural_sizes.len()];
    let extra = distribute(leftover, &weights);
    natural_sizes
        .iter()
        .zip(extra)
        .map(|(size, share)| size.saturating_add(share))
        .collect()
}

// ---- generic integer proportional distribution -----------------------------

/// Splits `total` proportionally over `weights`, in pure integer arithmetic
/// (`ADR-0016`): each share is `total * weight / sum(weights)`, and the
/// rounding remainder is handed out one raw unit at a time, in order, so the
/// shares always sum back to exactly `total`.
fn distribute(total: Au, weights: &[i64]) -> Vec<Au> {
    let sum = weights
        .iter()
        .fold(0_i64, |acc, weight| acc.saturating_add(*weight));
    if sum == 0 {
        return weights.iter().map(|_| Au::ZERO).collect();
    }
    let raw = i64::from(total.raw());
    let mut shares: Vec<i64> = weights
        .iter()
        .map(|weight| raw.saturating_mul(*weight).checked_div(sum).unwrap_or(0))
        .collect();
    let allocated = shares
        .iter()
        .fold(0_i64, |acc, share| acc.saturating_add(*share));
    let mut leftover = raw.saturating_sub(allocated);
    distribute_leftover(&mut shares, weights, &mut leftover);
    shares
        .into_iter()
        .map(|share| i32::try_from(share).map_or(Au::ZERO, Au::from_raw))
        .collect()
}

/// Hands out the rounding remainder from [`distribute`] one raw unit per
/// weighted slot, in order — deterministic, and bounded (the remainder is
/// always smaller in magnitude than `weights.len()`, so this always
/// terminates within a handful of passes).
fn distribute_leftover(shares: &mut [i64], weights: &[i64], leftover: &mut i64) {
    if *leftover == 0 {
        return;
    }
    let step: i64 = if *leftover > 0 { 1 } else { -1 };
    loop {
        let mut progressed = false;
        for (share, weight) in shares.iter_mut().zip(weights.iter()) {
            if *leftover == 0 {
                return;
            }
            if *weight > 0 {
                *share = share.saturating_add(step);
                *leftover = leftover.saturating_sub(step);
                progressed = true;
            }
        }
        if !progressed {
            return;
        }
    }
}
