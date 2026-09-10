//! The Flexbox half of the cascade (v0.5 B4): the nine properties of
//! [`FlexStyle`], applied, reset to `initial` and copied for `inherit`.
//!
//! Split out of `values.rs` so neither file grows a third page of `match`
//! arms — nine properties times three operations is exactly the kind of growth
//! `ADR-0010` rule 7 asks to be given its own home.

use crate::domain::computed::flex::FlexStyle;
use crate::domain::computed::style::ComputedStyle;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::{
    parse_align_content, parse_align_items, parse_align_self, parse_flex_direction,
    parse_flex_factor, parse_flex_wrap, parse_justify_content, parse_sizing,
};

/// `style` with one Flexbox property set from `tokens`, or `None` when the
/// property is not a Flexbox one or its value is outside the cut.
pub(crate) fn apply(
    style: ComputedStyle,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let flex = style.flex();
    updated(flex, property, tokens).map(|changed| style.with_flex(changed))
}

fn updated(flex: FlexStyle, property: &str, tokens: &[Token]) -> Option<FlexStyle> {
    match property {
        "flex-direction" => parse_flex_direction(tokens).map(|value| flex.with_direction(value)),
        "flex-wrap" => parse_flex_wrap(tokens).map(|value| flex.with_wrap(value)),
        "justify-content" => {
            parse_justify_content(tokens).map(|value| flex.with_justify_content(value))
        }
        "align-items" => parse_align_items(tokens).map(|value| flex.with_align_items(value)),
        "align-content" => parse_align_content(tokens).map(|value| flex.with_align_content(value)),
        "align-self" => parse_align_self(tokens).map(|value| flex.with_align_self(value)),
        "flex-grow" => parse_flex_factor(tokens).map(|value| flex.with_grow(value)),
        "flex-shrink" => parse_flex_factor(tokens).map(|value| flex.with_shrink(value)),
        "flex-basis" => parse_sizing(tokens).map(|value| flex.with_basis(value)),
        _ => None,
    }
}

/// `style` with one Flexbox property reset to its CSS `initial` value.
pub(crate) fn reset(style: ComputedStyle, property: &str) -> Option<ComputedStyle> {
    copy_from(style, FlexStyle::initial(), property)
}

/// `style` with one Flexbox property copied from `parent` — the `inherit`
/// keyword forcing inheritance of a property that never inherits on its own.
pub(crate) fn inherit(
    style: ComputedStyle,
    parent: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    copy_from(style, parent.flex(), property)
}

fn copy_from(style: ComputedStyle, source: FlexStyle, property: &str) -> Option<ComputedStyle> {
    let flex = style.flex();
    taken(flex, source, property).map(|changed| style.with_flex(changed))
}

fn taken(flex: FlexStyle, source: FlexStyle, property: &str) -> Option<FlexStyle> {
    match property {
        "flex-direction" => Some(flex.with_direction(source.direction())),
        "flex-wrap" => Some(flex.with_wrap(source.wrap())),
        "justify-content" => Some(flex.with_justify_content(source.justify_content())),
        "align-items" => Some(flex.with_align_items(source.align_items())),
        "align-content" => Some(flex.with_align_content(source.align_content())),
        "align-self" => Some(flex.with_align_self(source.align_self())),
        "flex-grow" => Some(flex.with_grow(source.grow())),
        "flex-shrink" => Some(flex.with_shrink(source.shrink())),
        "flex-basis" => Some(flex.with_basis(source.basis())),
        _ => None,
    }
}
