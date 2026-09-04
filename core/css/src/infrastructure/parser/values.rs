//! Component-value parsing: the tokens of a declaration's value turned into the
//! computed-value vocabulary the cascade applies.
//!
//! The v0.5 B1 slice is deliberately narrow — the lengths `px` / `em` / `rem` /
//! `%` / `pt`, the colour forms `#rgb` / `#rrggbb` plus a handful of names, and
//! the `display` keywords `ComputedStyle` already carries. `rgb()` / `rgba()`
//! and the full CSS colour name table are B2's slice (`plano:420-434`); a value
//! outside this set makes the declaration drop with a note, never a silently
//! wrong colour.

use crate::domain::color::CssColor;
use crate::domain::computed::display::Display;
use crate::domain::computed::edges::LengthEdges;
use crate::domain::declaration::DeclarationValue;
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::tokenizer::tokenize;

/// How many hex digits a `#rrggbb` colour carries.
const LONG_HEX_DIGITS: usize = 6;
/// How many a `#rgb` colour carries.
const SHORT_HEX_DIGITS: usize = 3;

/// The non-whitespace tokens of a declaration's value.
#[must_use]
pub(crate) fn value_tokens(value: &DeclarationValue) -> Vec<Token> {
    tokenize(value.as_str())
        .iter()
        .map(|spanned| spanned.token().clone())
        .filter(|token| !token.is_whitespace())
        .collect()
}

/// One length component: a dimension in a supported unit, a percentage, or the
/// unitless `0`.
#[must_use]
pub(crate) fn length_from_token(token: &Token) -> Option<Length> {
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

/// A single-component length value — `font-size: 1.2em`.
#[must_use]
pub(crate) fn parse_length(tokens: &[Token]) -> Option<Length> {
    match tokens {
        [only] => length_from_token(only),
        _ => None,
    }
}

/// The one-to-four component shorthand of `margin` and `padding`
/// (CSS Box Model §8.3).
#[must_use]
pub(crate) fn parse_length_edges(tokens: &[Token]) -> Option<LengthEdges> {
    let lengths: Option<Vec<Length>> = tokens.iter().map(length_from_token).collect();
    expand_edges(&lengths?)
}

fn expand_edges(lengths: &[Length]) -> Option<LengthEdges> {
    match lengths {
        [all] => Some(LengthEdges::uniform(*all)),
        [block, inline] => Some(LengthEdges::new(*block, *inline, *block, *inline)),
        [top, inline, bottom] => Some(LengthEdges::new(*top, *inline, *bottom, *inline)),
        [top, right, bottom, left] => Some(LengthEdges::new(*top, *right, *bottom, *left)),
        _ => None,
    }
}

/// A colour: `#rgb`, `#rrggbb`, `transparent`, or one of the basic named
/// colours. Anything else — including `rgb()`, which arrives in B2 — is `None`.
#[must_use]
pub(crate) fn parse_color(tokens: &[Token]) -> Option<CssColor> {
    match tokens {
        [Token::Hash(digits)] => hex_color(digits),
        [Token::Ident(name)] => named_color(&name.to_ascii_lowercase()),
        _ => None,
    }
}

fn hex_color(digits: &str) -> Option<CssColor> {
    let expanded = expand_hex(digits)?;
    let mut channels = expanded.as_bytes().chunks_exact(2).map(pair_value);
    let red = channels.next()??;
    let green = channels.next()??;
    let blue = channels.next()??;
    Some(CssColor::rgb(red, green, blue))
}

/// `#rgb` is `#rrggbb` with each digit doubled (CSS Color L4 §6.1).
fn expand_hex(digits: &str) -> Option<String> {
    if digits.len() == LONG_HEX_DIGITS {
        return Some(digits.to_owned());
    }
    if digits.len() != SHORT_HEX_DIGITS {
        return None;
    }
    Some(digits.chars().flat_map(|digit| [digit, digit]).collect())
}

/// One `rr` / `gg` / `bb` pair as a channel value.
fn pair_value(pair: &[u8]) -> Option<u8> {
    let text = core::str::from_utf8(pair).ok()?;
    u8::from_str_radix(text, 16).ok()
}

/// The colour keywords B1 recognises. B2 replaces this with the full CSS Color
/// L4 name table; until then an unknown name drops the declaration with a note
/// rather than painting something arbitrary.
fn named_color(name: &str) -> Option<CssColor> {
    match name {
        "transparent" => Some(CssColor::TRANSPARENT),
        "black" => Some(CssColor::rgb(0x00, 0x00, 0x00)),
        "silver" => Some(CssColor::rgb(0xC0, 0xC0, 0xC0)),
        "gray" | "grey" => Some(CssColor::rgb(0x80, 0x80, 0x80)),
        "white" => Some(CssColor::rgb(0xFF, 0xFF, 0xFF)),
        "maroon" => Some(CssColor::rgb(0x80, 0x00, 0x00)),
        "red" => Some(CssColor::rgb(0xFF, 0x00, 0x00)),
        "purple" => Some(CssColor::rgb(0x80, 0x00, 0x80)),
        "green" => Some(CssColor::rgb(0x00, 0x80, 0x00)),
        "lime" => Some(CssColor::rgb(0x00, 0xFF, 0x00)),
        "olive" => Some(CssColor::rgb(0x80, 0x80, 0x00)),
        "yellow" => Some(CssColor::rgb(0xFF, 0xFF, 0x00)),
        "navy" => Some(CssColor::rgb(0x00, 0x00, 0x80)),
        "blue" => Some(CssColor::rgb(0x00, 0x00, 0xFF)),
        "teal" => Some(CssColor::rgb(0x00, 0x80, 0x80)),
        "aqua" | "cyan" => Some(CssColor::rgb(0x00, 0xFF, 0xFF)),
        "fuchsia" | "magenta" => Some(CssColor::rgb(0xFF, 0x00, 0xFF)),
        "orange" => Some(CssColor::rgb(0xFF, 0xA5, 0x00)),
        _ => None,
    }
}

/// The `display` keywords [`Display`] carries.
#[must_use]
pub(crate) fn parse_display(tokens: &[Token]) -> Option<Display> {
    let [Token::Ident(name)] = tokens else {
        return None;
    };
    match name.to_ascii_lowercase().as_str() {
        "none" => Some(Display::None),
        "block" => Some(Display::Block),
        "inline" => Some(Display::Inline),
        "flex" => Some(Display::Flex),
        _ => None,
    }
}
