//! The positioning half of the cascade: `position`, `top`, `right`, `bottom`, `left`, `z-index`.

use crate::domain::computed::position::PositionStyle;
use crate::domain::computed::style::ComputedStyle;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::{parse_position, parse_sizing, parse_z_index};

pub(crate) fn apply(
    style: ComputedStyle,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let position = style.position();
    updated(position, property, tokens).map(|changed| style.with_position(changed))
}

fn updated(position: PositionStyle, property: &str, tokens: &[Token]) -> Option<PositionStyle> {
    match property {
        "position" => parse_position(tokens).map(|val| position.with_position(val)),
        "top" => parse_sizing(tokens).map(|val| position.with_top(val)),
        "right" => parse_sizing(tokens).map(|val| position.with_right(val)),
        "bottom" => parse_sizing(tokens).map(|val| position.with_bottom(val)),
        "left" => parse_sizing(tokens).map(|val| position.with_left(val)),
        "z-index" => parse_z_index(tokens).map(|val| position.with_z_index(val)),
        _ => None,
    }
}

pub(crate) fn reset(style: ComputedStyle, property: &str) -> Option<ComputedStyle> {
    copy_from(style, PositionStyle::initial(), property)
}

pub(crate) fn inherit(
    style: ComputedStyle,
    parent: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    copy_from(style, parent.position(), property)
}

fn copy_from(style: ComputedStyle, source: PositionStyle, property: &str) -> Option<ComputedStyle> {
    let current = style.position();
    let updated = match property {
        "position" => current.with_position(source.position()),
        "top" => current.with_top(source.top()),
        "right" => current.with_right(source.right()),
        "bottom" => current.with_bottom(source.bottom()),
        "left" => current.with_left(source.left()),
        "z-index" => current.with_z_index(source.z_index()),
        _ => return None,
    };
    Some(style.with_position(updated))
}
