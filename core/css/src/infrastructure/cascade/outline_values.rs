//! Outline half of the cascade: `outline`, `outline-offset`, `outline-color`,
//! `outline-width`, `outline-style` (CSS UI 4 §3).

use crate::domain::computed::outline::Outline;
use crate::domain::computed::style::ComputedStyle;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::{
    parse_color, parse_length, parse_outline_shorthand, parse_outline_style, parse_outline_width,
};

pub(crate) fn apply(
    style: &ComputedStyle,
    property: &str,
    tokens: &[Token],
) -> Option<ComputedStyle> {
    let current = style.outline();
    updated(current, property, tokens).map(|changed| style.clone().with_outline(changed))
}

fn updated(current: Outline, property: &str, tokens: &[Token]) -> Option<Outline> {
    match property {
        "outline" => apply_outline_shorthand(current, tokens),
        "outline-offset" => parse_length(tokens).map(|offset| current.with_offset(offset)),
        "outline-color" => parse_color(tokens).map(|color| current.with_color(color)),
        "outline-width" => parse_outline_width(tokens).map(|width| current.with_width(width)),
        "outline-style" => parse_outline_style(tokens).map(|style| current.with_style(style)),
        _ => None,
    }
}

fn apply_outline_shorthand(current: Outline, tokens: &[Token]) -> Option<Outline> {
    let parsed = parse_outline_shorthand(tokens)?;
    Some(
        current
            .with_color(parsed.color())
            .with_style(parsed.style())
            .with_width(parsed.width()),
    )
}

pub(crate) fn reset(style: &ComputedStyle, property: &str) -> Option<ComputedStyle> {
    copy_from(style, Outline::initial(), property)
}

pub(crate) fn inherit(
    style: &ComputedStyle,
    parent: &ComputedStyle,
    property: &str,
) -> Option<ComputedStyle> {
    copy_from(style, parent.outline(), property)
}

fn copy_from(style: &ComputedStyle, source: Outline, property: &str) -> Option<ComputedStyle> {
    let current = style.outline();
    let updated = match property {
        "outline" => current
            .with_color(source.color())
            .with_style(source.style())
            .with_width(source.width()),
        "outline-offset" => current.with_offset(source.offset()),
        "outline-color" => current.with_color(source.color()),
        "outline-width" => current.with_width(source.width()),
        "outline-style" => current.with_style(source.style()),
        _ => return None,
    };
    Some(style.clone().with_outline(updated))
}
