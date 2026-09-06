//! Helper token-parsing utilities for visual values.

use crate::domain::color::CssColor;
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;

use super::hex::hex_color;

#[must_use]
pub fn length_from_token(token: &Token) -> Option<Length> {
    match token {
        Token::Dimension(magnitude, unit) => length_with_unit(*magnitude, unit),
        Token::Percentage(magnitude) => Some(Length::Percent(*magnitude)),
        Token::Number(magnitude) if *magnitude == 0.0 => Some(Length::ZERO),
        _ => None,
    }
}

fn length_with_unit(magnitude: f32, unit: &str) -> Option<Length> {
    match unit.to_ascii_lowercase().as_str() {
        "px" => Some(Length::Pixels(magnitude)),
        "em" => Some(Length::Em(magnitude)),
        "rem" => Some(Length::Rem(magnitude)),
        "pt" => Some(Length::Points(magnitude)),
        _ => None,
    }
}

#[must_use]
pub fn parse_single_color(tokens: &[Token]) -> Option<CssColor> {
    match tokens {
        [Token::Ident(name)] => named_color(name),
        [Token::Hash(digits)] => hex_color(digits),
        _ => None,
    }
}

#[must_use]
pub fn named_color(name: &str) -> Option<CssColor> {
    match name.to_ascii_lowercase().as_str() {
        "transparent" => Some(CssColor::TRANSPARENT),
        "black" => Some(CssColor::BLACK),
        "white" => Some(CssColor::rgb(0xFF, 0xFF, 0xFF)),
        "red" => Some(CssColor::rgb(0xFF, 0x00, 0x00)),
        "green" => Some(CssColor::rgb(0x00, 0x80, 0x00)),
        "blue" => Some(CssColor::rgb(0x00, 0x00, 0xFF)),
        "silver" => Some(CssColor::rgb(0xC0, 0xC0, 0xC0)),
        "gray" | "grey" => Some(CssColor::rgb(0x80, 0x80, 0x80)),
        "maroon" => Some(CssColor::rgb(0x80, 0x00, 0x00)),
        "purple" => Some(CssColor::rgb(0x80, 0x00, 0x80)),
        "lime" => Some(CssColor::rgb(0x00, 0xFF, 0x00)),
        "olive" => Some(CssColor::rgb(0x80, 0x80, 0x00)),
        "yellow" => Some(CssColor::rgb(0xFF, 0xFF, 0x00)),
        "navy" => Some(CssColor::rgb(0x00, 0x00, 0x80)),
        "teal" => Some(CssColor::rgb(0x00, 0x80, 0x80)),
        "aqua" | "cyan" => Some(CssColor::rgb(0x00, 0xFF, 0xFF)),
        "fuchsia" | "magenta" => Some(CssColor::rgb(0xFF, 0x00, 0xFF)),
        "orange" => Some(CssColor::rgb(0xFF, 0xA5, 0x00)),
        _ => None,
    }
}
