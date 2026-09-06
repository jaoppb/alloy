//! Overflow half of the cascade: `overflow`, `overflow-x`, `overflow-y`.

use crate::domain::computed::overflow::OverflowStyle;
use crate::domain::computed::style::ComputedStyle;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::parse_overflow;

pub(crate) fn apply(
    style: ComputedStyle,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let overflow = style.overflow();
    updated(overflow, property, tokens).map(|changed| style.with_overflow(changed))
}

fn updated(overflow: OverflowStyle, property: &str, tokens: &[Token]) -> Option<OverflowStyle> {
    match property {
        "overflow" => parse_overflow_shorthand(tokens),
        "overflow-x" => parse_overflow(tokens).map(|val| overflow.with_x(val)),
        "overflow-y" => parse_overflow(tokens).map(|val| overflow.with_y(val)),
        _ => None,
    }
}

fn parse_overflow_shorthand(tokens: &[Token]) -> Option<OverflowStyle> {
    match tokens {
        [single] => parse_overflow(core::slice::from_ref(single)).map(OverflowStyle::uniform),
        [first, second] => {
            let x = parse_overflow(core::slice::from_ref(first))?;
            let y = parse_overflow(core::slice::from_ref(second))?;
            Some(OverflowStyle::initial().with_x(x).with_y(y))
        }
        _ => None,
    }
}

pub(crate) fn reset(style: ComputedStyle, property: &str) -> Option<ComputedStyle> {
    copy_from(style, OverflowStyle::initial(), property)
}

pub(crate) fn inherit(
    style: ComputedStyle,
    parent: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    copy_from(style, parent.overflow(), property)
}

fn copy_from(style: ComputedStyle, source: OverflowStyle, property: &str) -> Option<ComputedStyle> {
    let current = style.overflow();
    let updated = match property {
        "overflow" => source,
        "overflow-x" => current.with_x(source.x()),
        "overflow-y" => current.with_y(source.y()),
        _ => return None,
    };
    Some(style.with_overflow(updated))
}
