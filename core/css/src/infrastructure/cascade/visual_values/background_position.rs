//! Background position parser.

use crate::domain::computed::visual::BackgroundPosition;
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;

use super::helpers::length_from_token;

#[must_use]
pub fn parse_background_position(tokens: &[Token]) -> Option<BackgroundPosition> {
    match tokens {
        [single] => {
            let (x, y) = position_single(single)?;
            Some(BackgroundPosition::new(x, y))
        }
        [first, second] => {
            let x = position_component(first)?;
            let y = position_component(second)?;
            Some(BackgroundPosition::new(x, y))
        }
        _ => None,
    }
}

fn position_single(token: &Token) -> Option<(Length, Length)> {
    match token {
        Token::Ident(name) if name.eq_ignore_ascii_case("center") => {
            Some((Length::Percent(50.0), Length::Percent(50.0)))
        }
        Token::Ident(name) if name.eq_ignore_ascii_case("left") => {
            Some((Length::Percent(0.0), Length::Percent(50.0)))
        }
        Token::Ident(name) if name.eq_ignore_ascii_case("right") => {
            Some((Length::Percent(100.0), Length::Percent(50.0)))
        }
        Token::Ident(name) if name.eq_ignore_ascii_case("top") => {
            Some((Length::Percent(50.0), Length::Percent(0.0)))
        }
        Token::Ident(name) if name.eq_ignore_ascii_case("bottom") => {
            Some((Length::Percent(50.0), Length::Percent(100.0)))
        }
        _ => {
            let x = length_from_token(token)?;
            Some((x, Length::Percent(50.0)))
        }
    }
}

fn position_component(token: &Token) -> Option<Length> {
    match token {
        Token::Ident(name)
            if name.eq_ignore_ascii_case("left") || name.eq_ignore_ascii_case("top") =>
        {
            Some(Length::Percent(0.0))
        }
        Token::Ident(name) if name.eq_ignore_ascii_case("center") => Some(Length::Percent(50.0)),
        Token::Ident(name)
            if name.eq_ignore_ascii_case("right") || name.eq_ignore_ascii_case("bottom") =>
        {
            Some(Length::Percent(100.0))
        }
        _ => length_from_token(token),
    }
}
