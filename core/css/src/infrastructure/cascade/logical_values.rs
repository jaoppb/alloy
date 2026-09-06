//! Cascade adapter for CSS Logical Properties and Values L1.

use crate::domain::computed::edges::LengthEdges;
use crate::domain::computed::logical::{
    Direction, LogicalAxis, LogicalEdges, LogicalInsets, LogicalSide, LogicalStyle, PhysicalAxis,
    WritingContext, WritingMode,
};
use crate::domain::computed::sizing::Sizing;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;

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
    let c = style.constraints();
    match (property, context.map_axis(LogicalAxis::Inline)) {
        ("min-inline-size", PhysicalAxis::Horizontal)
        | ("min-block-size", PhysicalAxis::Vertical) => {
            Some(style.with_constraints(c.with_min_width(size)))
        }
        ("min-inline-size", PhysicalAxis::Vertical)
        | ("min-block-size", PhysicalAxis::Horizontal) => {
            Some(style.with_constraints(c.with_min_height(size)))
        }
        ("max-inline-size", PhysicalAxis::Horizontal)
        | ("max-block-size", PhysicalAxis::Vertical) => {
            Some(style.with_constraints(c.with_max_width(size)))
        }
        ("max-inline-size", PhysicalAxis::Vertical)
        | ("max-block-size", PhysicalAxis::Horizontal) => {
            Some(style.with_constraints(c.with_max_height(size)))
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
    let len = parse_length(tokens)?;
    match property {
        "border-block-start-width" => Some(LogicalEdges::apply_to_physical(
            context,
            border,
            LogicalSide::BlockStart,
            len,
        )),
        "border-block-end-width" => Some(LogicalEdges::apply_to_physical(
            context,
            border,
            LogicalSide::BlockEnd,
            len,
        )),
        "border-inline-start-width" => Some(LogicalEdges::apply_to_physical(
            context,
            border,
            LogicalSide::InlineStart,
            len,
        )),
        "border-inline-end-width" => Some(LogicalEdges::apply_to_physical(
            context,
            border,
            LogicalSide::InlineEnd,
            len,
        )),
        _ => None,
    }
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
        "block-start" => parse_length(tokens).map(|len| {
            LogicalEdges::apply_to_physical(context, edges, LogicalSide::BlockStart, len)
        }),
        "block-end" => parse_length(tokens)
            .map(|len| LogicalEdges::apply_to_physical(context, edges, LogicalSide::BlockEnd, len)),
        "inline-start" => parse_length(tokens).map(|len| {
            LogicalEdges::apply_to_physical(context, edges, LogicalSide::InlineStart, len)
        }),
        "inline-end" => parse_length(tokens).map(|len| {
            LogicalEdges::apply_to_physical(context, edges, LogicalSide::InlineEnd, len)
        }),
        "block" => parse_one_or_two_lengths(tokens).map(|(s, e)| {
            apply_pair(
                edges,
                context,
                LogicalSide::BlockStart,
                LogicalSide::BlockEnd,
                s,
                e,
            )
        }),
        "inline" => parse_one_or_two_lengths(tokens).map(|(s, e)| {
            apply_pair(
                edges,
                context,
                LogicalSide::InlineStart,
                LogicalSide::InlineEnd,
                s,
                e,
            )
        }),
        _ => None,
    }
}

const fn apply_pair(
    edges: LengthEdges,
    context: WritingContext,
    start_side: LogicalSide,
    end_side: LogicalSide,
    start_val: Length,
    end_val: Length,
) -> LengthEdges {
    let updated = LogicalEdges::apply_to_physical(context, edges, start_side, start_val);
    LogicalEdges::apply_to_physical(context, updated, end_side, end_val)
}

fn apply_insets(
    style: ComputedStyle,
    context: WritingContext,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let pos = style.position();
    match property {
        "inset-block-start" => parse_single_sizing(tokens).map(|sz| {
            style.with_position(LogicalInsets::apply_to_position(
                context,
                pos,
                LogicalSide::BlockStart,
                sz,
            ))
        }),
        "inset-block-end" => parse_single_sizing(tokens).map(|sz| {
            style.with_position(LogicalInsets::apply_to_position(
                context,
                pos,
                LogicalSide::BlockEnd,
                sz,
            ))
        }),
        "inset-inline-start" => parse_single_sizing(tokens).map(|sz| {
            style.with_position(LogicalInsets::apply_to_position(
                context,
                pos,
                LogicalSide::InlineStart,
                sz,
            ))
        }),
        "inset-inline-end" => parse_single_sizing(tokens).map(|sz| {
            style.with_position(LogicalInsets::apply_to_position(
                context,
                pos,
                LogicalSide::InlineEnd,
                sz,
            ))
        }),
        "inset-block" => parse_one_or_two_sizings(tokens).map(|(s, e)| {
            let p1 = LogicalInsets::apply_to_position(context, pos, LogicalSide::BlockStart, s);
            let p2 = LogicalInsets::apply_to_position(context, p1, LogicalSide::BlockEnd, e);
            style.with_position(p2)
        }),
        "inset-inline" => parse_one_or_two_sizings(tokens).map(|(s, e)| {
            let p1 = LogicalInsets::apply_to_position(context, pos, LogicalSide::InlineStart, s);
            let p2 = LogicalInsets::apply_to_position(context, p1, LogicalSide::InlineEnd, e);
            style.with_position(p2)
        }),
        _ => None,
    }
}

fn length_from_token(token: &Token) -> Option<Length> {
    match token {
        Token::Dimension(mag, unit) => match unit.to_ascii_lowercase().as_str() {
            "px" => Some(Length::Pixels(*mag)),
            "em" => Some(Length::Em(*mag)),
            "rem" => Some(Length::Rem(*mag)),
            "pt" => Some(Length::Points(*mag)),
            _ => None,
        },
        Token::Percentage(mag) => Some(Length::Percent(*mag)),
        Token::Number(mag) if *mag == 0.0 => Some(Length::ZERO),
        _ => None,
    }
}

fn parse_length(tokens: &[Token]) -> Option<Length> {
    match tokens {
        [only] => length_from_token(only),
        _ => None,
    }
}

fn parse_border_shorthand(tokens: &[Token]) -> Option<Length> {
    if tokens
        .iter()
        .any(|t| matches!(t, Token::Ident(name) if name.eq_ignore_ascii_case("none")))
    {
        return Some(Length::ZERO);
    }
    tokens.iter().find_map(length_from_token)
}

fn parse_sizing(tokens: &[Token]) -> Option<Sizing> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("auto") => Some(Sizing::Auto),
        [only] => length_from_token(only).map(Sizing::Fixed),
        _ => None,
    }
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

fn parse_single_sizing(tokens: &[Token]) -> Option<Sizing> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("auto") => Some(Sizing::Auto),
        [only] => length_from_token(only).map(Sizing::Fixed),
        _ => None,
    }
}

fn parse_one_or_two_sizings(tokens: &[Token]) -> Option<(Sizing, Sizing)> {
    match tokens {
        [single] => {
            let sizing = parse_single_sizing(core::slice::from_ref(single))?;
            Some((sizing, sizing))
        }
        [first, second] => {
            let start = parse_single_sizing(core::slice::from_ref(first))?;
            let end = parse_single_sizing(core::slice::from_ref(second))?;
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
    match property {
        "writing-mode" => parse_writing_mode(tokens)
            .map(|m| {
                *logical =
                    logical.with_context(WritingContext::new(m, logical.context().direction()));
            })
            .is_some(),
        "direction" => parse_direction(tokens)
            .map(|d| {
                *logical =
                    logical.with_context(WritingContext::new(logical.context().writing_mode(), d));
            })
            .is_some(),
        _ => false,
    }
}

fn apply_sizing_to_logical(logical: &mut LogicalStyle, property: &str, tokens: &[Token]) -> bool {
    let Some(size) = parse_sizing(tokens) else {
        return false;
    };
    let s = logical.sizing();
    let updated = match property {
        "inline-size" => s.with_inline_size(size),
        "block-size" => s.with_block_size(size),
        "min-inline-size" => s.with_min_inline_size(size),
        "min-block-size" => s.with_min_block_size(size),
        "max-inline-size" => s.with_max_inline_size(size),
        "max-block-size" => s.with_max_block_size(size),
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
    let m = logical.margin();
    let updated =
        match property {
            "margin-block-start" => parse_length(tokens).map(|l| m.with_block_start(l)),
            "margin-block-end" => parse_length(tokens).map(|l| m.with_block_end(l)),
            "margin-inline-start" => parse_length(tokens).map(|l| m.with_inline_start(l)),
            "margin-inline-end" => parse_length(tokens).map(|l| m.with_inline_end(l)),
            "margin-block" => parse_one_or_two_lengths(tokens)
                .map(|(s, e)| m.with_block_start(s).with_block_end(e)),
            "margin-inline" => parse_one_or_two_lengths(tokens)
                .map(|(s, e)| m.with_inline_start(s).with_inline_end(e)),
            _ => return false,
        };
    if let Some(val) = updated {
        *logical = logical.with_margin(val);
        return true;
    }
    false
}

fn apply_padding_to_logical(logical: &mut LogicalStyle, property: &str, tokens: &[Token]) -> bool {
    let p = logical.padding();
    let updated =
        match property {
            "padding-block-start" => parse_length(tokens).map(|l| p.with_block_start(l)),
            "padding-block-end" => parse_length(tokens).map(|l| p.with_block_end(l)),
            "padding-inline-start" => parse_length(tokens).map(|l| p.with_inline_start(l)),
            "padding-inline-end" => parse_length(tokens).map(|l| p.with_inline_end(l)),
            "padding-block" => parse_one_or_two_lengths(tokens)
                .map(|(s, e)| p.with_block_start(s).with_block_end(e)),
            "padding-inline" => parse_one_or_two_lengths(tokens)
                .map(|(s, e)| p.with_inline_start(s).with_inline_end(e)),
            _ => return false,
        };
    if let Some(val) = updated {
        *logical = logical.with_padding(val);
        return true;
    }
    false
}

fn apply_border_to_logical(logical: &mut LogicalStyle, property: &str, tokens: &[Token]) -> bool {
    let b = logical.border();
    let updated =
        match property {
            "border-block-start-width" => parse_length(tokens).map(|l| b.with_block_start(l)),
            "border-block-end-width" => parse_length(tokens).map(|l| b.with_block_end(l)),
            "border-inline-start-width" => parse_length(tokens).map(|l| b.with_inline_start(l)),
            "border-inline-end-width" => parse_length(tokens).map(|l| b.with_inline_end(l)),
            "border-block-width" => parse_one_or_two_lengths(tokens)
                .map(|(s, e)| b.with_block_start(s).with_block_end(e)),
            "border-inline-width" => parse_one_or_two_lengths(tokens)
                .map(|(s, e)| b.with_inline_start(s).with_inline_end(e)),
            "border-block" => {
                parse_border_shorthand(tokens).map(|w| b.with_block_start(w).with_block_end(w))
            }
            "border-inline" => {
                parse_border_shorthand(tokens).map(|w| b.with_inline_start(w).with_inline_end(w))
            }
            _ => return false,
        };
    if let Some(val) = updated {
        *logical = logical.with_border(val);
        return true;
    }
    false
}

fn apply_insets_to_logical(logical: &mut LogicalStyle, property: &str, tokens: &[Token]) -> bool {
    let ins = logical.insets();
    let updated = match property {
        "inset-block-start" => parse_single_sizing(tokens).map(|s| ins.with_block_start(s)),
        "inset-block-end" => parse_single_sizing(tokens).map(|s| ins.with_block_end(s)),
        "inset-inline-start" => parse_single_sizing(tokens).map(|s| ins.with_inline_start(s)),
        "inset-inline-end" => parse_single_sizing(tokens).map(|s| ins.with_inline_end(s)),
        "inset-block" => {
            parse_one_or_two_sizings(tokens).map(|(s, e)| ins.with_block_start(s).with_block_end(e))
        }
        "inset-inline" => parse_one_or_two_sizings(tokens)
            .map(|(s, e)| ins.with_inline_start(s).with_inline_end(e)),
        _ => return false,
    };
    if let Some(val) = updated {
        *logical = logical.with_insets(val);
        return true;
    }
    false
}
