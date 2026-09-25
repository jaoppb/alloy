//! Border longhand parsers: single styles, colors, and radii.

use crate::domain::color::CssColor;
use crate::domain::computed::visual::BorderStyle;
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;

use super::helpers::{length_from_token, parse_single_color};

#[must_use]
pub fn parse_border_style(tokens: &[Token]) -> Option<BorderStyle> {
    match tokens {
        [Token::Ident(name)] => BorderStyle::from_keyword(name),
        _ => None,
    }
}

#[must_use]
pub fn parse_border_color(tokens: &[Token]) -> Option<CssColor> {
    parse_single_color(tokens)
}

#[must_use]
pub fn parse_border_radius_corner(tokens: &[Token]) -> Option<Length> {
    match tokens {
        [token] => length_from_token(token),
        _ => None,
    }
}
