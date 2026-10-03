//! Sizing constraints half of the cascade: `min-width`, `max-width`, `min-height`, `max-height`.

use crate::domain::computed::sizing_constraints::SizingConstraints;
use crate::domain::computed::style::ComputedStyle;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::parse_constraint_sizing;

pub(crate) fn apply(
    style: ComputedStyle,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let constraints = style.constraints();
    updated(constraints, property, tokens).map(|changed| style.with_constraints(changed))
}

fn updated(
    constraints: SizingConstraints,
    property: &str,
    tokens: &[Token],
) -> Option<SizingConstraints> {
    match property {
        "min-width" => parse_constraint_sizing(tokens).map(|val| constraints.with_min_width(val)),
        "max-width" => parse_constraint_sizing(tokens).map(|val| constraints.with_max_width(val)),
        "min-height" => parse_constraint_sizing(tokens).map(|val| constraints.with_min_height(val)),
        "max-height" => parse_constraint_sizing(tokens).map(|val| constraints.with_max_height(val)),
        _ => None,
    }
}

pub(crate) fn reset(style: ComputedStyle, property: &str) -> Option<ComputedStyle> {
    copy_from(style, SizingConstraints::initial(), property)
}

pub(crate) fn inherit(
    style: ComputedStyle,
    parent: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    copy_from(style, parent.constraints(), property)
}

fn copy_from(
    style: ComputedStyle,
    source: SizingConstraints,
    property: &str,
) -> Option<ComputedStyle> {
    let current = style.constraints();
    let updated = match property {
        "min-width" => current.with_min_width(source.min_width()),
        "max-width" => current.with_max_width(source.max_width()),
        "min-height" => current.with_min_height(source.min_height()),
        "max-height" => current.with_max_height(source.max_height()),
        _ => return None,
    };
    Some(style.with_constraints(updated))
}
