//! Opacity parser.

use crate::domain::computed::visual::Opacity;
use crate::infrastructure::parser::token::Token;

const PERCENT_DIVISOR: f32 = 100.0;

/// Parses an `opacity` declaration into an [`Opacity`] value.
#[must_use]
pub fn parse_opacity(tokens: &[Token]) -> Option<Opacity> {
    match tokens {
        [Token::Number(value)] => Opacity::new(*value),
        [Token::Percentage(value)] => Opacity::new(*value / PERCENT_DIVISOR),
        _ => None,
    }
}
