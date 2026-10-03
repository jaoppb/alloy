//! Cascade adapter for CSS Logical Properties and Values L1.
//!
//! A flow-relative property and its physical counterpart share one computed
//! value (CSS Logical L1 §4), so every logical declaration is written straight
//! into the physical field its writing context maps it to. That mapping needs
//! the element's **final** `writing-mode` and `direction`, which is why the
//! cascade (`author_rules.rs`) settles those two properties in a pass of their
//! own — see [`sets_writing_context`] — before any other declaration applies.
//!
//! [`reset`] and [`inherit`] are the `initial` / `inherit` arms for every
//! logical property: they read the source value through the source's own
//! writing context and write it through the element's.

use crate::domain::computed::edges::LengthEdges;
use crate::domain::computed::logical::{
    Direction, LogicalAxis, LogicalEdges, LogicalInsets, LogicalSide, LogicalSizing, LogicalStyle,
    PhysicalAxis, WritingContext, WritingMode,
};
use crate::domain::computed::sizing::Sizing;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::{
    length_from_token, parse_border_shorthand, parse_length, parse_sizing,
};

/// The two properties that make up an element's [`WritingContext`].
const WRITING_MODE: &str = "writing-mode";
const DIRECTION: &str = "direction";

/// Whether `property` is one of the two inherited properties that decide how
/// every other logical property maps to a physical one (CSS Writing Modes L3
/// §2.1, §3.1).
#[must_use]
pub(crate) fn sets_writing_context(property: &str) -> bool {
    matches!(property, WRITING_MODE | DIRECTION)
}

/// Applies a logical property declaration using default writing context (horizontal-tb, ltr).
#[must_use]
pub fn apply(style: ComputedStyle, property: &str, tokens: &[Token]) -> Option<ComputedStyle> {
    apply_with_context(style, WritingContext::default(), property, tokens)
}

/// Applies a logical property declaration using the specified writing context.
#[must_use]
pub fn apply_with_context(
    style: ComputedStyle,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    apply_sizing(style, context, property, tokens)
        .or_else(|| apply_margin(style, context, property, tokens))
        .or_else(|| apply_padding(style, context, property, tokens))
        .or_else(|| apply_border(style, context, property, tokens))
        .or_else(|| apply_insets(style, context, property, tokens))
}

fn apply_sizing(
    style: ComputedStyle,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let size = parse_sizing(tokens)?;
    match (property, context.map_axis(LogicalAxis::Inline)) {
        ("inline-size", PhysicalAxis::Horizontal) | ("block-size", PhysicalAxis::Vertical) => {
            Some(style.with_width(size))
        }
        ("inline-size", PhysicalAxis::Vertical) | ("block-size", PhysicalAxis::Horizontal) => {
            Some(style.with_height(size))
        }
        _ => apply_sizing_constraint(style, context, property, size),
    }
}

fn apply_sizing_constraint(
    style: ComputedStyle,
    context: WritingContext,
    property: &str,
    size: Sizing,
) -> Option<ComputedStyle> {
    let constraints = style.constraints();
    match (property, context.map_axis(LogicalAxis::Inline)) {
        ("min-inline-size", PhysicalAxis::Horizontal)
        | ("min-block-size", PhysicalAxis::Vertical) => {
            Some(style.with_constraints(constraints.with_min_width(size)))
        }
        ("min-inline-size", PhysicalAxis::Vertical)
        | ("min-block-size", PhysicalAxis::Horizontal) => {
            Some(style.with_constraints(constraints.with_min_height(size)))
        }
        ("max-inline-size", PhysicalAxis::Horizontal)
        | ("max-block-size", PhysicalAxis::Vertical) => {
            Some(style.with_constraints(constraints.with_max_width(size)))
        }
        ("max-inline-size", PhysicalAxis::Vertical)
        | ("max-block-size", PhysicalAxis::Horizontal) => {
            Some(style.with_constraints(constraints.with_max_height(size)))
        }
        _ => None,
    }
}

fn apply_margin(
    style: ComputedStyle,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let edges = apply_box_edge(style.margin(), context, "margin-", property, tokens)?;
    Some(style.with_margin(edges))
}

fn apply_padding(
    style: ComputedStyle,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let edges = apply_box_edge(style.padding(), context, "padding-", property, tokens)?;
    Some(style.with_padding(edges))
}

fn apply_border(
    style: ComputedStyle,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    apply_border_shorthand(style.border(), context, property, tokens)
        .or_else(|| apply_border_width_shorthand(style.border(), context, property, tokens))
        .or_else(|| apply_border_longhand(style.border(), context, property, tokens))
        .map(|edges| style.with_border(edges))
}

fn apply_border_shorthand(
    border: LengthEdges,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<LengthEdges> {
    let width = parse_border_shorthand(tokens)?;
    match property {
        "border-block" => Some(apply_pair(
            border,
            context,
            LogicalSide::BlockStart,
            LogicalSide::BlockEnd,
            width,
            width,
        )),
        "border-inline" => Some(apply_pair(
            border,
            context,
            LogicalSide::InlineStart,
            LogicalSide::InlineEnd,
            width,
            width,
        )),
        _ => None,
    }
}

fn apply_border_width_shorthand(
    border: LengthEdges,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<LengthEdges> {
    let (start, end) = parse_one_or_two_lengths(tokens)?;
    match property {
        "border-block-width" => Some(apply_pair(
            border,
            context,
            LogicalSide::BlockStart,
            LogicalSide::BlockEnd,
            start,
            end,
        )),
        "border-inline-width" => Some(apply_pair(
            border,
            context,
            LogicalSide::InlineStart,
            LogicalSide::InlineEnd,
            start,
            end,
        )),
        _ => None,
    }
}

fn apply_border_longhand(
    border: LengthEdges,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<LengthEdges> {
    let length = parse_length(tokens)?;
    let side = match property {
        "border-block-start-width" => LogicalSide::BlockStart,
        "border-block-end-width" => LogicalSide::BlockEnd,
        "border-inline-start-width" => LogicalSide::InlineStart,
        "border-inline-end-width" => LogicalSide::InlineEnd,
        _ => return None,
    };
    Some(LogicalEdges::apply_to_physical(
        context, border, side, length,
    ))
}

fn apply_box_edge(
    edges: LengthEdges,
    context: WritingContext,
    prefix: &str,
    property: &str,
    tokens: &[Token],
) -> Option<LengthEdges> {
    let suffix = property.strip_prefix(prefix)?;
    match suffix {
        "block" => parse_one_or_two_lengths(tokens).map(|(start, end)| {
            apply_pair(
                edges,
                context,
                LogicalSide::BlockStart,
                LogicalSide::BlockEnd,
                start,
                end,
            )
        }),
        "inline" => parse_one_or_two_lengths(tokens).map(|(start, end)| {
            apply_pair(
                edges,
                context,
                LogicalSide::InlineStart,
                LogicalSide::InlineEnd,
                start,
                end,
            )
        }),
        _ => apply_box_edge_side(edges, context, suffix, tokens),
    }
}

fn apply_box_edge_side(
    edges: LengthEdges,
    context: WritingContext,
    suffix: &str,
    tokens: &[Token],
) -> Option<LengthEdges> {
    let side = logical_side_named(suffix)?;
    let length = parse_length(tokens)?;
    Some(LogicalEdges::apply_to_physical(
        context, edges, side, length,
    ))
}

/// `block-start` / `block-end` / `inline-start` / `inline-end` → the side.
fn logical_side_named(name: &str) -> Option<LogicalSide> {
    match name {
        "block-start" => Some(LogicalSide::BlockStart),
        "block-end" => Some(LogicalSide::BlockEnd),
        "inline-start" => Some(LogicalSide::InlineStart),
        "inline-end" => Some(LogicalSide::InlineEnd),
        _ => None,
    }
}

const fn apply_pair(
    edges: LengthEdges,
    context: WritingContext,
    start_side: LogicalSide,
    end_side: LogicalSide,
    start_length: Length,
    end_length: Length,
) -> LengthEdges {
    let updated = LogicalEdges::apply_to_physical(context, edges, start_side, start_length);
    LogicalEdges::apply_to_physical(context, updated, end_side, end_length)
}

fn apply_insets(
    style: ComputedStyle,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let position = style.position();
    let (start_side, end_side) = match property {
        "inset-block" => (LogicalSide::BlockStart, LogicalSide::BlockEnd),
        "inset-inline" => (LogicalSide::InlineStart, LogicalSide::InlineEnd),
        _ => return apply_inset_longhand(style, context, property, tokens),
    };
    let (start, end) = parse_one_or_two_sizings(tokens)?;
    let with_start = LogicalInsets::apply_to_position(context, position, start_side, start);
    let with_both = LogicalInsets::apply_to_position(context, with_start, end_side, end);
    Some(style.with_position(with_both))
}

fn apply_inset_longhand(
    style: ComputedStyle,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let side = logical_side_named(property.strip_prefix("inset-")?)?;
    let offset = parse_sizing(tokens)?;
    Some(style.with_position(LogicalInsets::apply_to_position(
        context,
        style.position(),
        side,
        offset,
    )))
}

fn parse_one_or_two_lengths(tokens: &[Token]) -> Option<(Length, Length)> {
    match tokens {
        [single] => {
            let length = length_from_token(single)?;
            Some((length, length))
        }
        [first, second] => {
            let start = length_from_token(first)?;
            let end = length_from_token(second)?;
            Some((start, end))
        }
        _ => None,
    }
}

fn parse_one_or_two_sizings(tokens: &[Token]) -> Option<(Sizing, Sizing)> {
    match tokens {
        [single] => {
            let sizing = parse_sizing(core::slice::from_ref(single))?;
            Some((sizing, sizing))
        }
        [first, second] => {
            let start = parse_sizing(core::slice::from_ref(first))?;
            let end = parse_sizing(core::slice::from_ref(second))?;
            Some((start, end))
        }
        _ => None,
    }
}

/// Parses the `writing-mode` property value.
#[must_use]
pub fn parse_writing_mode(tokens: &[Token]) -> Option<WritingMode> {
    let [Token::Ident(name)] = tokens else {
        return None;
    };
    match name.to_ascii_lowercase().as_str() {
        "horizontal-tb" => Some(WritingMode::HorizontalTb),
        "vertical-rl" => Some(WritingMode::VerticalRl),
        "vertical-lr" => Some(WritingMode::VerticalLr),
        _ => None,
    }
}

/// Parses the `direction` property value.
#[must_use]
pub fn parse_direction(tokens: &[Token]) -> Option<Direction> {
    let [Token::Ident(name)] = tokens else {
        return None;
    };
    match name.to_ascii_lowercase().as_str() {
        "ltr" => Some(Direction::Ltr),
        "rtl" => Some(Direction::Rtl),
        _ => None,
    }
}

/// Applies a logical property to a [`LogicalStyle`] accumulator.
pub fn apply_to_logical_style(
    logical: &mut LogicalStyle,
    property: &str,
    tokens: &[Token],
) -> bool {
    apply_mode_or_direction_to_logical(logical, property, tokens)
        || apply_sizing_to_logical(logical, property, tokens)
        || apply_edges_to_logical(logical, property, tokens)
        || apply_insets_to_logical(logical, property, tokens)
}

fn apply_mode_or_direction_to_logical(
    logical: &mut LogicalStyle,
    property: &str,
    tokens: &[Token],
) -> bool {
    let context = logical.context();
    let updated = match property {
        WRITING_MODE => parse_writing_mode(tokens)
            .map(|writing_mode| WritingContext::new(writing_mode, context.direction())),
        DIRECTION => parse_direction(tokens)
            .map(|direction| WritingContext::new(context.writing_mode(), direction)),
        _ => None,
    };
    let Some(updated_context) = updated else {
        return false;
    };
    *logical = logical.with_context(updated_context);
    true
}

fn apply_sizing_to_logical(logical: &mut LogicalStyle, property: &str, tokens: &[Token]) -> bool {
    let Some(size) = parse_sizing(tokens) else {
        return false;
    };
    let sizing = logical.sizing();
    let updated = match property {
        "inline-size" => sizing.with_inline_size(size),
        "block-size" => sizing.with_block_size(size),
        "min-inline-size" => sizing.with_min_inline_size(size),
        "min-block-size" => sizing.with_min_block_size(size),
        "max-inline-size" => sizing.with_max_inline_size(size),
        "max-block-size" => sizing.with_max_block_size(size),
        _ => return false,
    };
    *logical = logical.with_sizing(updated);
    true
}

fn apply_edges_to_logical(logical: &mut LogicalStyle, property: &str, tokens: &[Token]) -> bool {
    apply_margin_to_logical(logical, property, tokens)
        || apply_padding_to_logical(logical, property, tokens)
        || apply_border_to_logical(logical, property, tokens)
}

fn apply_margin_to_logical(logical: &mut LogicalStyle, property: &str, tokens: &[Token]) -> bool {
    let Some(suffix) = property.strip_prefix("margin-") else {
        return false;
    };
    let Some(margin) = logical_edges_with(logical.margin(), suffix, tokens) else {
        return false;
    };
    *logical = logical.with_margin(margin);
    true
}

fn apply_padding_to_logical(logical: &mut LogicalStyle, property: &str, tokens: &[Token]) -> bool {
    let Some(suffix) = property.strip_prefix("padding-") else {
        return false;
    };
    let Some(padding) = logical_edges_with(logical.padding(), suffix, tokens) else {
        return false;
    };
    *logical = logical.with_padding(padding);
    true
}

/// `edges` with the `-block` / `-inline` shorthand or the `-<side>` longhand
/// named by `suffix` applied, or `None` when `suffix` names neither or the
/// value does not parse.
fn logical_edges_with(edges: LogicalEdges, suffix: &str, tokens: &[Token]) -> Option<LogicalEdges> {
    match suffix {
        "block" => parse_one_or_two_lengths(tokens)
            .map(|(start, end)| edges.with_block_start(start).with_block_end(end)),
        "inline" => parse_one_or_two_lengths(tokens)
            .map(|(start, end)| edges.with_inline_start(start).with_inline_end(end)),
        _ => {
            let side = logical_side_named(suffix)?;
            parse_length(tokens).map(|length| edges.with_side(side, length))
        }
    }
}

fn apply_border_to_logical(logical: &mut LogicalStyle, property: &str, tokens: &[Token]) -> bool {
    let border = logical.border();
    let updated = match property {
        "border-block" => parse_border_shorthand(tokens)
            .map(|width| border.with_block_start(width).with_block_end(width)),
        "border-inline" => parse_border_shorthand(tokens)
            .map(|width| border.with_inline_start(width).with_inline_end(width)),
        _ => border_width_suffix(property)
            .and_then(|suffix| logical_edges_with(border, suffix, tokens)),
    };
    let Some(border) = updated else {
        return false;
    };
    *logical = logical.with_border(border);
    true
}

/// `border-block-start-width` → `block-start`, `border-inline-width` →
/// `inline`: the part [`logical_edges_with`] reads.
fn border_width_suffix(property: &str) -> Option<&str> {
    property
        .strip_prefix("border-")
        .and_then(|rest| rest.strip_suffix("-width"))
}

fn apply_insets_to_logical(logical: &mut LogicalStyle, property: &str, tokens: &[Token]) -> bool {
    let Some(suffix) = property.strip_prefix("inset-") else {
        return false;
    };
    let insets = logical.insets();
    let updated = match suffix {
        "block" => parse_one_or_two_sizings(tokens)
            .map(|(start, end)| insets.with_block_start(start).with_block_end(end)),
        "inline" => parse_one_or_two_sizings(tokens)
            .map(|(start, end)| insets.with_inline_start(start).with_inline_end(end)),
        _ => logical_side_named(suffix)
            .and_then(|side| parse_sizing(tokens).map(|offset| insets.with_side(side, offset))),
    };
    let Some(insets) = updated else {
        return false;
    };
    *logical = logical.with_insets(insets);
    true
}

// ---- CSS-wide keywords (CSS Cascade L4 §7.1) ------------------------------

/// Which box an edge longhand edits.
#[derive(Clone, Copy)]
enum EdgeBox {
    Margin,
    Padding,
    BorderWidth,
}

/// Which of the three sizing properties of one axis a longhand edits.
#[derive(Clone, Copy)]
enum SizeBound {
    Preferred,
    Minimum,
    Maximum,
}

/// One flow-relative longhand — the unit `initial` and `inherit` act on. A
/// shorthand such as `margin-inline` is the pair of its longhands.
#[derive(Clone, Copy)]
enum LogicalLonghand {
    Edge(EdgeBox, LogicalSide),
    Inset(LogicalSide),
    Size(SizeBound, LogicalAxis),
}

/// `style` with `property` at its CSS `initial` value, or `None` when
/// `property` is not a logical property this module owns.
#[must_use]
pub(crate) fn reset(style: ComputedStyle, property: &str) -> Option<ComputedStyle> {
    copy_from(style, &ComputedStyle::initial(), property)
}

/// `style` with `property` copied from `parent` — the `inherit` keyword — or
/// `None` when `property` is not a logical property this module owns. A
/// logical longhand reads the parent's value through the **parent's** writing
/// context: `inherit` hands down the parent's `margin-inline-start`, wherever
/// that side lies physically for the child.
#[must_use]
pub(crate) fn inherit(
    style: ComputedStyle,
    parent: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    copy_from(style, parent, property)
}

fn copy_from(
    style: ComputedStyle,
    source: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    copy_writing_context(style, source, property).or_else(|| {
        let longhands = logical_longhands(property)?;
        Some(longhands.iter().fold(style, |copied, longhand| {
            copy_longhand(copied, source, *longhand)
        }))
    })
}

fn copy_writing_context(
    style: ComputedStyle,
    source: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    let context = style.logical().context();
    let source_context = source.logical().context();
    let copied = match property {
        WRITING_MODE => WritingContext::new(source_context.writing_mode(), context.direction()),
        DIRECTION => WritingContext::new(context.writing_mode(), source_context.direction()),
        _ => return None,
    };
    Some(style.with_logical(style.logical().with_context(copied)))
}

/// The longhands `property` stands for, or `None` when it is not a logical
/// box, inset or sizing property.
fn logical_longhands(property: &str) -> Option<Vec<LogicalLonghand>> {
    edge_longhands(property, "margin-", EdgeBox::Margin)
        .or_else(|| edge_longhands(property, "padding-", EdgeBox::Padding))
        .or_else(|| border_longhands(property))
        .or_else(|| inset_longhands(property))
        .or_else(|| size_longhand(property))
}

fn edge_longhands(property: &str, prefix: &str, edge_box: EdgeBox) -> Option<Vec<LogicalLonghand>> {
    let sides = logical_sides(property.strip_prefix(prefix)?)?;
    Some(
        sides
            .into_iter()
            .map(|side| LogicalLonghand::Edge(edge_box, side))
            .collect(),
    )
}

fn border_longhands(property: &str) -> Option<Vec<LogicalLonghand>> {
    let suffix = match property {
        "border-block" => "block",
        "border-inline" => "inline",
        _ => border_width_suffix(property)?,
    };
    let sides = logical_sides(suffix)?;
    Some(
        sides
            .into_iter()
            .map(|side| LogicalLonghand::Edge(EdgeBox::BorderWidth, side))
            .collect(),
    )
}

fn inset_longhands(property: &str) -> Option<Vec<LogicalLonghand>> {
    let sides = logical_sides(property.strip_prefix("inset-")?)?;
    Some(sides.into_iter().map(LogicalLonghand::Inset).collect())
}

/// `block` / `inline` (a two-sided shorthand) → both sides of that axis; a
/// side name → that one side.
fn logical_sides(suffix: &str) -> Option<Vec<LogicalSide>> {
    match suffix {
        "block" => Some(vec![LogicalSide::BlockStart, LogicalSide::BlockEnd]),
        "inline" => Some(vec![LogicalSide::InlineStart, LogicalSide::InlineEnd]),
        _ => logical_side_named(suffix).map(|side| vec![side]),
    }
}

fn size_longhand(property: &str) -> Option<Vec<LogicalLonghand>> {
    let (bound, dimension) = size_bound_of(property);
    let axis = match dimension {
        "inline-size" => LogicalAxis::Inline,
        "block-size" => LogicalAxis::Block,
        _ => return None,
    };
    Some(vec![LogicalLonghand::Size(bound, axis)])
}

/// `min-inline-size` → (`Minimum`, `inline-size`), `block-size` →
/// (`Preferred`, `block-size`).
fn size_bound_of(property: &str) -> (SizeBound, &str) {
    if let Some(dimension) = property.strip_prefix("min-") {
        return (SizeBound::Minimum, dimension);
    }
    if let Some(dimension) = property.strip_prefix("max-") {
        return (SizeBound::Maximum, dimension);
    }
    (SizeBound::Preferred, property)
}

const fn copy_longhand(
    style: ComputedStyle,
    source: &ComputedStyle,
    longhand: LogicalLonghand,
) -> ComputedStyle {
    match longhand {
        LogicalLonghand::Edge(edge_box, side) => copy_edge(style, source, edge_box, side),
        LogicalLonghand::Inset(side) => copy_inset(style, source, side),
        LogicalLonghand::Size(bound, axis) => copy_size(style, source, bound, axis),
    }
}

/// One logical edge: read on `source` through its context, written on
/// `style` through the element's, and recorded in the element's
/// [`LogicalStyle`].
const fn copy_edge(
    style: ComputedStyle,
    source: &ComputedStyle,
    edge_box: EdgeBox,
    side: LogicalSide,
) -> ComputedStyle {
    let length = LogicalEdges::read_from_physical(
        source.logical().context(),
        physical_edges(source, edge_box),
        side,
    );
    let edges = LogicalEdges::apply_to_physical(
        style.logical().context(),
        physical_edges(&style, edge_box),
        side,
        length,
    );
    let recorded = recorded_edge(style.logical(), edge_box, side, length);
    with_physical_edges(style, edge_box, edges).with_logical(recorded)
}

const fn physical_edges(style: &ComputedStyle, edge_box: EdgeBox) -> LengthEdges {
    match edge_box {
        EdgeBox::Margin => style.margin(),
        EdgeBox::Padding => style.padding(),
        EdgeBox::BorderWidth => style.border(),
    }
}

const fn with_physical_edges(
    style: ComputedStyle,
    edge_box: EdgeBox,
    edges: LengthEdges,
) -> ComputedStyle {
    match edge_box {
        EdgeBox::Margin => style.with_margin(edges),
        EdgeBox::Padding => style.with_padding(edges),
        EdgeBox::BorderWidth => style.with_border(edges),
    }
}

const fn recorded_edge(
    logical: LogicalStyle,
    edge_box: EdgeBox,
    side: LogicalSide,
    length: Length,
) -> LogicalStyle {
    match edge_box {
        EdgeBox::Margin => logical.with_margin(logical.margin().with_side(side, length)),
        EdgeBox::Padding => logical.with_padding(logical.padding().with_side(side, length)),
        EdgeBox::BorderWidth => logical.with_border(logical.border().with_side(side, length)),
    }
}

const fn copy_inset(
    style: ComputedStyle,
    source: &ComputedStyle,
    side: LogicalSide,
) -> ComputedStyle {
    let offset =
        LogicalInsets::read_from_position(source.logical().context(), source.position(), side);
    let position =
        LogicalInsets::apply_to_position(style.logical().context(), style.position(), side, offset);
    let logical = style.logical();
    let recorded = logical.with_insets(logical.insets().with_side(side, offset));
    style.with_position(position).with_logical(recorded)
}

const fn copy_size(
    style: ComputedStyle,
    source: &ComputedStyle,
    bound: SizeBound,
    axis: LogicalAxis,
) -> ComputedStyle {
    let source_axis = source.logical().context().map_axis(axis);
    let size = physical_size(source, bound, source_axis);
    let target_axis = style.logical().context().map_axis(axis);
    let logical = style.logical();
    let recorded = logical.with_sizing(recorded_size(logical.sizing(), bound, axis, size));
    with_physical_size(style, bound, target_axis, size).with_logical(recorded)
}

const fn physical_size(style: &ComputedStyle, bound: SizeBound, axis: PhysicalAxis) -> Sizing {
    let constraints = style.constraints();
    match (bound, axis) {
        (SizeBound::Preferred, PhysicalAxis::Horizontal) => style.width(),
        (SizeBound::Preferred, PhysicalAxis::Vertical) => style.height(),
        (SizeBound::Minimum, PhysicalAxis::Horizontal) => constraints.min_width(),
        (SizeBound::Minimum, PhysicalAxis::Vertical) => constraints.min_height(),
        (SizeBound::Maximum, PhysicalAxis::Horizontal) => constraints.max_width(),
        (SizeBound::Maximum, PhysicalAxis::Vertical) => constraints.max_height(),
    }
}

const fn with_physical_size(
    style: ComputedStyle,
    bound: SizeBound,
    axis: PhysicalAxis,
    size: Sizing,
) -> ComputedStyle {
    let constraints = style.constraints();
    match (bound, axis) {
        (SizeBound::Preferred, PhysicalAxis::Horizontal) => style.with_width(size),
        (SizeBound::Preferred, PhysicalAxis::Vertical) => style.with_height(size),
        (SizeBound::Minimum, PhysicalAxis::Horizontal) => {
            style.with_constraints(constraints.with_min_width(size))
        }
        (SizeBound::Minimum, PhysicalAxis::Vertical) => {
            style.with_constraints(constraints.with_min_height(size))
        }
        (SizeBound::Maximum, PhysicalAxis::Horizontal) => {
            style.with_constraints(constraints.with_max_width(size))
        }
        (SizeBound::Maximum, PhysicalAxis::Vertical) => {
            style.with_constraints(constraints.with_max_height(size))
        }
    }
}

const fn recorded_size(
    sizing: LogicalSizing,
    bound: SizeBound,
    axis: LogicalAxis,
    size: Sizing,
) -> LogicalSizing {
    match (bound, axis) {
        (SizeBound::Preferred, LogicalAxis::Inline) => sizing.with_inline_size(size),
        (SizeBound::Preferred, LogicalAxis::Block) => sizing.with_block_size(size),
        (SizeBound::Minimum, LogicalAxis::Inline) => sizing.with_min_inline_size(size),
        (SizeBound::Minimum, LogicalAxis::Block) => sizing.with_min_block_size(size),
        (SizeBound::Maximum, LogicalAxis::Inline) => sizing.with_max_inline_size(size),
        (SizeBound::Maximum, LogicalAxis::Block) => sizing.with_max_block_size(size),
    }
}
