//! Border longhand parsers: single styles, colors, and radii.

use crate::domain::color::CssColor;
use crate::domain::computed::visual::BorderStyle;
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::{parse_color, parse_length};

#[must_use]
pub fn parse_border_style(tokens: &[Token]) -> Option<BorderStyle> {
    match tokens {
        [Token::Ident(name)] => BorderStyle::from_keyword(name),
        _ => None,
    }
}

/// One border colour — any form the canonical colour parser reads, `rgb()` /
/// `rgba()` included (CSS Backgrounds & Borders L3 §4.1).
#[must_use]
pub fn parse_border_color(tokens: &[Token]) -> Option<CssColor> {
    parse_color(tokens)
}

/// One corner radius: a single length or percentage.
#[must_use]
pub fn parse_border_radius_corner(tokens: &[Token]) -> Option<Length> {
    parse_length(tokens)
}
