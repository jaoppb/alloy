//! Component-value parsing: the tokens of a declaration's value turned into the
//! computed-value vocabulary the cascade applies.
//!
//! The v0.5 B1 slice was deliberately narrow — the lengths `px` / `em` /
//! `rem` / `%` / `pt`, the colour forms `#rgb` / `#rrggbb` plus a handful of
//! names, and the `display` keywords `ComputedStyle` already carries. B2
//! (`plano:435-443`) adds the functional colour notation `rgb()` / `rgba()`
//! (CSS Color L4 §5.1, legacy comma syntax); the full CSS colour name table
//! remains out of the cut. A value outside this set makes the declaration
//! drop with a note, never a silently wrong colour.

use graphics::Opacity;

use crate::domain::color::CssColor;
use crate::domain::computed::display::Display;
use crate::domain::computed::edges::LengthEdges;
use crate::domain::computed::flex::{
    AlignContent, AlignItems, AlignSelf, FlexDirection, FlexFactor, FlexWrap, JustifyContent,
};
use crate::domain::computed::font::{FamilyName, FontFamily, FontFamilyList, GenericFamily};
use crate::domain::computed::inline_style::{TextAlign, WhiteSpace};
use crate::domain::computed::sizing::{BoxSizing, Sizing};
use crate::domain::declaration::DeclarationValue;
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::tokenizer::tokenize;

/// A percentage's divisor, turning `50%` into the unit fraction `0.5`.
const PERCENT_DIVISOR: f32 = 100.0;

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

/// A colour: `#rgb`, `#rrggbb`, `transparent`, one of the basic named
/// colours, or `rgb()` / `rgba()`. Anything else is `None`.
#[must_use]
pub(crate) fn parse_color(tokens: &[Token]) -> Option<CssColor> {
    match tokens {
        [Token::Hash(digits)] => CssColor::from_hex_digits(digits),
        [Token::Ident(name)] => CssColor::from_keyword(&name.to_ascii_lowercase()),
        _ => functional_color(tokens),
    }
}

/// `rgb(r, g, b)` / `rgba(r, g, b, a)` — CSS Color L4 §5.1's legacy,
/// comma-separated syntax. The modern space-separated syntax is outside this
/// cut, same as the full colour name table.
fn functional_color(tokens: &[Token]) -> Option<CssColor> {
    let (name, arguments) = function_call(tokens)?;
    match name.to_ascii_lowercase().as_str() {
        "rgb" => rgb_color(arguments),
        "rgba" => rgba_color(arguments),
        _ => None,
    }
}

/// A function token's name and its arguments, stripped of the closing `)` —
/// `value_tokens` already dropped every whitespace token, so the opening `(`
/// is folded into [`Token::Function`] and never appears on its own.
fn function_call(tokens: &[Token]) -> Option<(&str, &[Token])> {
    let [
        Token::Function(name),
        arguments @ ..,
        Token::CloseParenthesis,
    ] = tokens
    else {
        return None;
    };
    Some((name.as_str(), arguments))
}

/// `rgb(r, g, b)`: three integers, each clamped into `[0, 255]`
/// (CSS Color L4 §5.1) — a malformed component (not a bare number) refuses
/// the whole colour, an out-of-range one clamps rather than refusing.
fn rgb_color(arguments: &[Token]) -> Option<CssColor> {
    let parts = comma_separated(arguments);
    let [red, green, blue] = parts.as_slice() else {
        return None;
    };
    Some(CssColor::rgb(
        channel(red)?,
        channel(green)?,
        channel(blue)?,
    ))
}

/// `rgba(r, g, b, a)`: the same three components as [`rgb_color`], plus an
/// alpha given as `0`–`1` or a percentage.
fn rgba_color(arguments: &[Token]) -> Option<CssColor> {
    let parts = comma_separated(arguments);
    let [red, green, blue, alpha] = parts.as_slice() else {
        return None;
    };
    Some(CssColor::rgba(
        channel(red)?,
        channel(green)?,
        channel(blue)?,
        alpha_channel(alpha)?,
    ))
}

fn comma_separated(tokens: &[Token]) -> Vec<&[Token]> {
    tokens
        .split(|token| matches!(token, Token::Comma))
        .collect()
}

/// One `r` / `g` / `b` component: a bare integer, clamped into `[0, 255]`.
///
/// Reuses [`graphics::Opacity`]'s clamp-then-round rather than writing a
/// second float→`u8` narrowing: `core/graphics/src/domain/convert.rs` is this
/// workspace's one sanctioned place for that conversion (`ADR-0016`), and
/// `core/css` has no carve-out of its own to duplicate it. Scaling the
/// `[0, 255]` component onto `Opacity`'s `[0, 1]` unit interval and back is
/// exact for the integers this cut accepts.
fn channel(tokens: &[Token]) -> Option<u8> {
    let [Token::Number(value)] = tokens else {
        return None;
    };
    Opacity::from_unit_interval(*value / f32::from(u8::MAX)).map(Opacity::level)
}

/// The alpha component of `rgba()`: `0`–`1` as a bare number, or a percentage.
fn alpha_channel(tokens: &[Token]) -> Option<u8> {
    match tokens {
        [Token::Number(value)] => Opacity::from_unit_interval(*value).map(Opacity::level),
        [Token::Percentage(value)] => {
            Opacity::from_unit_interval(*value / PERCENT_DIVISOR).map(Opacity::level)
        }
        _ => None,
    }
}

/// The `background` shorthand (CSS Backgrounds & Borders L3 §3.10), narrowed to
/// the single component this cut resolves: the background **colour**. `url()`
/// layers, `linear-gradient()` and the position / size / repeat / attachment
/// keywords are scanned past; `none` clears the colour to `transparent`.
///
/// `None` when the value names no colour and is not `none`, so a previous
/// `background-color` stands rather than being silently cleared. The image
/// itself is not fetched — a v0.7 line in `tests/data/MANIFEST.md`.
#[must_use]
pub(crate) fn parse_background_shorthand(tokens: &[Token]) -> Option<CssColor> {
    first_color(tokens).or_else(|| names_keyword(tokens, "none").then_some(CssColor::TRANSPARENT))
}

/// The `border` (and `border-top` / … ) shorthand (same spec, §4.3), narrowed
/// to the one component that is geometry: the border **width**. A
/// `<line-style>` keyword and a colour are scanned past; `none` and `0` mean no
/// border.
///
/// `None` when the value carries no width and is not `none` — CSS would compute
/// `medium` there, which this cut has no representation for.
#[must_use]
pub(crate) fn parse_border_shorthand(tokens: &[Token]) -> Option<Length> {
    if names_keyword(tokens, "none") {
        return Some(Length::ZERO);
    }
    tokens.iter().find_map(length_from_token)
}

/// Whether `tokens` contains the bare identifier `keyword`, case-insensitively.
fn names_keyword(tokens: &[Token], keyword: &str) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token, Token::Ident(name) if name.eq_ignore_ascii_case(keyword)))
}

/// The first run of `tokens` that reads as a colour — one `#hex` / name token,
/// or a whole `rgb()` / `rgba()` call — scanning past everything else.
fn first_color(tokens: &[Token]) -> Option<CssColor> {
    let mut rest = tokens;
    while let Some(head) = rest.first() {
        let width = colour_run_len(head, rest).clamp(1, rest.len());
        let (candidate, tail) = rest.split_at(width);
        if let Some(color) = parse_color(candidate) {
            return Some(color);
        }
        rest = tail;
    }
    None
}

/// How many tokens the colour candidate at `head` spans: a function call runs
/// through its matching `)`, anything else is a single token.
fn colour_run_len(head: &Token, tokens: &[Token]) -> usize {
    match head {
        Token::Function(_) => function_span(tokens),
        _ => 1,
    }
}

fn function_span(tokens: &[Token]) -> usize {
    let mut depth: usize = 0;
    for (index, token) in tokens.iter().enumerate() {
        depth = paren_depth(depth, token);
        if depth == 0 {
            return index.saturating_add(1);
        }
    }
    tokens.len()
}

const fn paren_depth(depth: usize, token: &Token) -> usize {
    match token {
        Token::Function(_) | Token::OpenParenthesis => depth.saturating_add(1),
        Token::CloseParenthesis => depth.saturating_sub(1),
        _ => depth,
    }
}

/// The `display` keywords [`Display`] carries.
#[must_use]
pub(crate) fn parse_display(tokens: &[Token]) -> Option<Display> {
    keyword(tokens).and_then(|name| name.parse().ok())
}

// ---- v0.5 B4: the properties the real layout engine reads -----------------

/// The single, lowercased identifier a keyword-valued property expects.
fn keyword(tokens: &[Token]) -> Option<String> {
    let [Token::Ident(name)] = tokens else {
        return None;
    };
    Some(name.to_ascii_lowercase())
}

/// `width` / `height` / `flex-basis`: `auto` or one length.
#[must_use]
pub(crate) fn parse_sizing(tokens: &[Token]) -> Option<Sizing> {
    if keyword(tokens).is_some_and(|name| name == "auto") {
        return Some(Sizing::Auto);
    }
    parse_length(tokens).map(Sizing::Fixed)
}

/// `box-sizing` (CSS Box Sizing L3 §5).
#[must_use]
pub(crate) fn parse_box_sizing(tokens: &[Token]) -> Option<BoxSizing> {
    keyword(tokens).and_then(|name| match name.as_str() {
        "content-box" => Some(BoxSizing::ContentBox),
        "border-box" => Some(BoxSizing::BorderBox),
        _ => None,
    })
}

/// `text-align` (CSS Text L3 §7.3) — the four physical keywords of the cut.
#[must_use]
pub(crate) fn parse_text_align(tokens: &[Token]) -> Option<TextAlign> {
    keyword(tokens).and_then(|name| match name.as_str() {
        "left" => Some(TextAlign::Left),
        "right" => Some(TextAlign::Right),
        "center" => Some(TextAlign::Center),
        "justify" => Some(TextAlign::Justify),
        _ => None,
    })
}

/// `white-space` (CSS Text L3 §4.1.1) — `pre-wrap` / `pre-line` are out.
#[must_use]
pub(crate) fn parse_white_space(tokens: &[Token]) -> Option<WhiteSpace> {
    keyword(tokens).and_then(|name| match name.as_str() {
        "normal" => Some(WhiteSpace::Normal),
        "pre" => Some(WhiteSpace::Pre),
        "nowrap" => Some(WhiteSpace::NoWrap),
        _ => None,
    })
}

/// `flex-direction` (CSS Flexbox L1 §5.1).
#[must_use]
pub(crate) fn parse_flex_direction(tokens: &[Token]) -> Option<FlexDirection> {
    keyword(tokens).and_then(|name| match name.as_str() {
        "row" => Some(FlexDirection::Row),
        "row-reverse" => Some(FlexDirection::RowReverse),
        "column" => Some(FlexDirection::Column),
        "column-reverse" => Some(FlexDirection::ColumnReverse),
        _ => None,
    })
}

/// `flex-wrap` (CSS Flexbox L1 §5.2).
#[must_use]
pub(crate) fn parse_flex_wrap(tokens: &[Token]) -> Option<FlexWrap> {
    keyword(tokens).and_then(|name| match name.as_str() {
        "nowrap" => Some(FlexWrap::NoWrap),
        "wrap" => Some(FlexWrap::Wrap),
        "wrap-reverse" => Some(FlexWrap::WrapReverse),
        _ => None,
    })
}

/// `justify-content` (CSS Flexbox L1 §8.2).
#[must_use]
pub(crate) fn parse_justify_content(tokens: &[Token]) -> Option<JustifyContent> {
    keyword(tokens).and_then(|name| match name.as_str() {
        "flex-start" => Some(JustifyContent::FlexStart),
        "flex-end" => Some(JustifyContent::FlexEnd),
        "center" => Some(JustifyContent::Center),
        "space-between" => Some(JustifyContent::SpaceBetween),
        "space-around" => Some(JustifyContent::SpaceAround),
        "space-evenly" => Some(JustifyContent::SpaceEvenly),
        _ => None,
    })
}

/// `align-items` (CSS Flexbox L1 §8.3).
#[must_use]
pub(crate) fn parse_align_items(tokens: &[Token]) -> Option<AlignItems> {
    keyword(tokens).and_then(|name| match name.as_str() {
        "flex-start" => Some(AlignItems::FlexStart),
        "flex-end" => Some(AlignItems::FlexEnd),
        "center" => Some(AlignItems::Center),
        "stretch" => Some(AlignItems::Stretch),
        "baseline" => Some(AlignItems::Baseline),
        _ => None,
    })
}

/// `align-content` (CSS Flexbox L1 §8.4).
#[must_use]
pub(crate) fn parse_align_content(tokens: &[Token]) -> Option<AlignContent> {
    keyword(tokens).and_then(|name| match name.as_str() {
        "flex-start" => Some(AlignContent::FlexStart),
        "flex-end" => Some(AlignContent::FlexEnd),
        "center" => Some(AlignContent::Center),
        "space-between" => Some(AlignContent::SpaceBetween),
        "space-around" => Some(AlignContent::SpaceAround),
        "stretch" => Some(AlignContent::Stretch),
        _ => None,
    })
}

/// `align-self` (CSS Flexbox L1 §8.3).
#[must_use]
pub(crate) fn parse_align_self(tokens: &[Token]) -> Option<AlignSelf> {
    keyword(tokens).and_then(|name| match name.as_str() {
        "auto" => Some(AlignSelf::Auto),
        "flex-start" => Some(AlignSelf::FlexStart),
        "flex-end" => Some(AlignSelf::FlexEnd),
        "center" => Some(AlignSelf::Center),
        "stretch" => Some(AlignSelf::Stretch),
        "baseline" => Some(AlignSelf::Baseline),
        _ => None,
    })
}

/// `flex-grow` / `flex-shrink`: one non-negative number.
#[must_use]
pub(crate) fn parse_flex_factor(tokens: &[Token]) -> Option<FlexFactor> {
    let [Token::Number(value)] = tokens else {
        return None;
    };
    FlexFactor::new(*value)
}

/// `font-family` (CSS Fonts L4 §5.1): a comma-separated list where each entry is
/// a quoted string, one or more bare identifiers folded to a single-space name,
/// or a bare generic keyword (`serif` / `sans-serif` / `monospace`). Over-long
/// chains and names are capped by [`FontFamilyList`] / [`FamilyName`] — a
/// documented cut, not a refusal, so a list with at least one readable entry
/// always parses.
#[must_use]
pub(crate) fn parse_font_family(tokens: &[Token]) -> Option<FontFamilyList> {
    if tokens.is_empty() {
        return None;
    }
    let parts = comma_separated(tokens);
    let families: Option<Vec<FontFamily>> =
        parts.iter().map(|part| family_from_part(part)).collect();
    Some(FontFamilyList::from_families(families?))
}

/// One comma-separated entry of a `font-family` list.
fn family_from_part(tokens: &[Token]) -> Option<FontFamily> {
    if let [Token::QuotedString(name)] = tokens {
        return Some(FontFamily::Named(FamilyName::new(name)));
    }
    named_or_generic(tokens)
}

/// A run of bare identifiers: a generic keyword when the run is exactly one of
/// the three, otherwise the identifiers folded to one space-separated name
/// (CSS Fonts L4 §5.1 — consecutive identifiers collapse to a single space).
fn named_or_generic(tokens: &[Token]) -> Option<FontFamily> {
    let names: Option<Vec<&str>> = tokens.iter().map(identifier_text).collect();
    let joined = names?.join(" ");
    if let Some(generic) = generic_family(&joined) {
        return Some(FontFamily::Generic(generic));
    }
    Some(FontFamily::Named(FamilyName::new(&joined)))
}

const fn identifier_text(token: &Token) -> Option<&str> {
    match token {
        Token::Ident(name) => Some(name.as_str()),
        _ => None,
    }
}

fn generic_family(name: &str) -> Option<GenericFamily> {
    match name.to_ascii_lowercase().as_str() {
        "serif" => Some(GenericFamily::Serif),
        "sans-serif" => Some(GenericFamily::SansSerif),
        "monospace" => Some(GenericFamily::Monospace),
        _ => None,
    }
}
