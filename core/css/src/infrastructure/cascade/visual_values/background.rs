//! Background property parsers: image, size, repeat.

use crate::domain::computed::sizing::Sizing;
use crate::domain::computed::visual::{
    BackgroundImage, BackgroundRepeat, BackgroundSize, ImageSource,
};
use crate::infrastructure::parser::token::Token;

use super::helpers::length_from_token;

#[must_use]
pub fn parse_background_image(tokens: &[Token]) -> Option<BackgroundImage> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("none") => Some(BackgroundImage::None),
        [Token::Url(raw)] => Some(BackgroundImage::Url(ImageSource::new(raw))),
        [
            Token::Function(name),
            Token::QuotedString(raw) | Token::Ident(raw),
            Token::CloseParenthesis,
        ] if name.eq_ignore_ascii_case("url") => Some(BackgroundImage::Url(ImageSource::new(raw))),
        _ => None,
    }
}

#[must_use]
pub fn parse_background_size(tokens: &[Token]) -> Option<BackgroundSize> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("cover") => Some(BackgroundSize::Cover),
        [Token::Ident(name)] if name.eq_ignore_ascii_case("contain") => {
            Some(BackgroundSize::Contain)
        }
        [Token::Ident(name)] if name.eq_ignore_ascii_case("auto") => Some(BackgroundSize::Auto),
        [single] => {
            let width = size_component(single)?;
            Some(BackgroundSize::explicit(width, Sizing::Auto))
        }
        [w, h] => {
            let width = size_component(w)?;
            let height = size_component(h)?;
            Some(BackgroundSize::explicit(width, height))
        }
        _ => None,
    }
}

fn size_component(token: &Token) -> Option<Sizing> {
    match token {
        Token::Ident(name) if name.eq_ignore_ascii_case("auto") => Some(Sizing::Auto),
        _ => length_from_token(token).map(Sizing::Fixed),
    }
}

#[must_use]
pub fn parse_background_repeat(tokens: &[Token]) -> Option<BackgroundRepeat> {
    match tokens {
        [Token::Ident(name)] => BackgroundRepeat::from_keyword(name),
        [Token::Ident(first), Token::Ident(second)] => parse_two_repeats(first, second),
        _ => None,
    }
}

fn parse_two_repeats(first: &str, second: &str) -> Option<BackgroundRepeat> {
    match (first, second) {
        ("repeat", "no-repeat") => Some(BackgroundRepeat::RepeatX),
        ("no-repeat", "repeat") => Some(BackgroundRepeat::RepeatY),
        ("repeat", "repeat") => Some(BackgroundRepeat::Repeat),
        ("no-repeat", "no-repeat") => Some(BackgroundRepeat::NoRepeat),
        ("space", "space") => Some(BackgroundRepeat::Space),
        ("round", "round") => Some(BackgroundRepeat::Round),
        _ => None,
    }
}
