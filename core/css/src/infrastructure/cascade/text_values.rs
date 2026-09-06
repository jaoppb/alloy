//! The typography and line formatting half of the cascade.
//!
//! Properties: `font-weight`, `font-style`, `line-height`, `letter-spacing`,
//! `word-spacing`, `text-decoration-line`, `text-decoration-color`,
//! `text-decoration-style`, `text-decoration` (shorthand), `text-transform`,
//! `text-overflow`, `overflow-wrap` (and `word-wrap`), `word-break`.

use crate::domain::color::CssColor;
use crate::domain::computed::text_advance::{
    FontStyle, FontWeight, LetterSpacing, LineHeight, LineHeightFactor, LineHeightPercentage,
    OverflowWrap, TextAdvanceStyle, TextDecoration, TextDecorationLine, TextDecorationStyle,
    TextOverflow, TextTransform, WordBreak, WordSpacing,
};
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::{parse_color, parse_length};

/// Parses `font-weight` keywords and numeric weights.
#[must_use]
pub fn parse_font_weight(tokens: &[Token]) -> Option<FontWeight> {
    match tokens {
        [Token::Ident(name)] => match name.to_ascii_lowercase().as_str() {
            "normal" => Some(FontWeight::NORMAL),
            "bold" => Some(FontWeight::BOLD),
            _ => None,
        },
        [Token::Number(value)] => integer_weight(*value).and_then(FontWeight::new),
        _ => None,
    }
}

fn integer_weight(value: f32) -> Option<u16> {
    format!("{value}").parse::<u16>().ok()
}

/// Parses `font-weight`, resolving relative keywords (`bolder`, `lighter`) against `current`.
#[must_use]
pub fn parse_font_weight_with_parent(tokens: &[Token], current: FontWeight) -> Option<FontWeight> {
    if let [Token::Ident(name)] = tokens {
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "bolder" => return Some(current.bolder()),
            "lighter" => return Some(current.lighter()),
            _ => {}
        }
    }
    parse_font_weight(tokens)
}

/// Parses `font-style`: `normal`, `italic`, `oblique`.
#[must_use]
pub fn parse_font_style(tokens: &[Token]) -> Option<FontStyle> {
    match tokens {
        [Token::Ident(name)] => match name.to_ascii_lowercase().as_str() {
            "normal" => Some(FontStyle::Normal),
            "italic" => Some(FontStyle::Italic),
            "oblique" => Some(FontStyle::Oblique),
            _ => None,
        },
        _ => None,
    }
}

/// Parses `line-height`: `normal`, `<length>`, `<number>`, `<percentage>`.
#[must_use]
pub fn parse_line_height(tokens: &[Token]) -> Option<LineHeight> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("normal") => Some(LineHeight::Normal),
        [Token::Number(val)] if *val >= 0.0 => {
            if *val == 0.0 {
                return Some(LineHeight::Length(Length::ZERO));
            }
            Some(LineHeight::Number(LineHeightFactor::new(*val)))
        }
        [Token::Percentage(val)] if *val >= 0.0 => {
            Some(LineHeight::Percentage(LineHeightPercentage::new(*val)))
        }
        _ => parse_length(tokens).map(LineHeight::Length),
    }
}

/// Parses `letter-spacing`: `normal` or `<length>`.
#[must_use]
pub fn parse_letter_spacing(tokens: &[Token]) -> Option<LetterSpacing> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("normal") => Some(LetterSpacing::Normal),
        _ => parse_length(tokens).map(LetterSpacing::Length),
    }
}

/// Parses `word-spacing`: `normal` or `<length>`.
#[must_use]
pub fn parse_word_spacing(tokens: &[Token]) -> Option<WordSpacing> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("normal") => Some(WordSpacing::Normal),
        _ => parse_length(tokens).map(WordSpacing::Length),
    }
}

fn is_none_keyword(tokens: &[Token]) -> bool {
    match tokens {
        [Token::Ident(name)] => name.eq_ignore_ascii_case("none"),
        _ => false,
    }
}

/// Parses `text-decoration-line`: `none`, `underline`, `overline`, `line-through`.
#[must_use]
pub fn parse_text_decoration_line(tokens: &[Token]) -> Option<TextDecorationLine> {
    if tokens.is_empty() {
        return None;
    }
    if is_none_keyword(tokens) {
        return Some(TextDecorationLine::NONE);
    }
    let mut line = TextDecorationLine::NONE;
    let mut matched = false;
    for token in tokens {
        let Token::Ident(name) = token else {
            return None;
        };
        match name.to_ascii_lowercase().as_str() {
            "underline" => {
                line = line.with_underline(true);
                matched = true;
            }
            "overline" => {
                line = line.with_overline(true);
                matched = true;
            }
            "line-through" => {
                line = line.with_line_through(true);
                matched = true;
            }
            _ => return None,
        }
    }
    if !matched {
        return None;
    }
    Some(line)
}

/// Parses `text-decoration-style`: `solid`, `double`, `dotted`, `dashed`, `wavy`.
#[must_use]
pub fn parse_text_decoration_style(tokens: &[Token]) -> Option<TextDecorationStyle> {
    match tokens {
        [Token::Ident(name)] => match name.to_ascii_lowercase().as_str() {
            "solid" => Some(TextDecorationStyle::Solid),
            "double" => Some(TextDecorationStyle::Double),
            "dotted" => Some(TextDecorationStyle::Dotted),
            "dashed" => Some(TextDecorationStyle::Dashed),
            "wavy" => Some(TextDecorationStyle::Wavy),
            _ => None,
        },
        _ => None,
    }
}

/// Parses `text-decoration-color`.
#[must_use]
pub fn parse_text_decoration_color(tokens: &[Token]) -> Option<CssColor> {
    parse_color(tokens)
}

fn match_decoration_keyword(
    keyword: &str,
    line: &mut TextDecorationLine,
    style: &mut Option<TextDecorationStyle>,
) -> bool {
    match keyword {
        "underline" => {
            *line = line.with_underline(true);
            true
        }
        "overline" => {
            *line = line.with_overline(true);
            true
        }
        "line-through" => {
            *line = line.with_line_through(true);
            true
        }
        "solid" if style.is_none() => {
            *style = Some(TextDecorationStyle::Solid);
            true
        }
        "double" if style.is_none() => {
            *style = Some(TextDecorationStyle::Double);
            true
        }
        "dotted" if style.is_none() => {
            *style = Some(TextDecorationStyle::Dotted);
            true
        }
        "dashed" if style.is_none() => {
            *style = Some(TextDecorationStyle::Dashed);
            true
        }
        "wavy" if style.is_none() => {
            *style = Some(TextDecorationStyle::Wavy);
            true
        }
        _ => false,
    }
}

/// Parses `text-decoration` shorthand: line, style, and color in any order.
#[must_use]
pub fn parse_text_decoration(tokens: &[Token]) -> Option<TextDecoration> {
    if tokens.is_empty() {
        return None;
    }
    if is_none_keyword(tokens) {
        return Some(TextDecoration::initial());
    }
    let mut line = TextDecorationLine::NONE;
    let mut style = None;
    let mut color = None;
    let mut matched = false;

    for token in tokens {
        if let Token::Ident(name) = token {
            let lower = name.to_ascii_lowercase();
            if match_decoration_keyword(&lower, &mut line, &mut style) {
                matched = true;
                continue;
            }
        }
        let parsed_color = parse_color(core::slice::from_ref(token));
        if color.is_none() && parsed_color.is_some() {
            color = parsed_color;
            matched = true;
            continue;
        }
        return None;
    }

    if !matched {
        return None;
    }

    Some(
        TextDecoration::initial()
            .with_line(line)
            .with_style(style.unwrap_or(TextDecorationStyle::Solid))
            .with_color(color.unwrap_or(CssColor::BLACK)),
    )
}

/// Parses `text-transform`: `none`, `capitalize`, `uppercase`, `lowercase`.
#[must_use]
pub fn parse_text_transform(tokens: &[Token]) -> Option<TextTransform> {
    match tokens {
        [Token::Ident(name)] => match name.to_ascii_lowercase().as_str() {
            "none" => Some(TextTransform::None),
            "capitalize" => Some(TextTransform::Capitalize),
            "uppercase" => Some(TextTransform::Uppercase),
            "lowercase" => Some(TextTransform::Lowercase),
            _ => None,
        },
        _ => None,
    }
}

/// Parses `text-overflow`: `clip`, `ellipsis`.
#[must_use]
pub fn parse_text_overflow(tokens: &[Token]) -> Option<TextOverflow> {
    match tokens {
        [Token::Ident(name)] => match name.to_ascii_lowercase().as_str() {
            "clip" => Some(TextOverflow::Clip),
            "ellipsis" => Some(TextOverflow::Ellipsis),
            _ => None,
        },
        _ => None,
    }
}

/// Parses `overflow-wrap`: `normal`, `break-word`, `anywhere`.
#[must_use]
pub fn parse_overflow_wrap(tokens: &[Token]) -> Option<OverflowWrap> {
    match tokens {
        [Token::Ident(name)] => match name.to_ascii_lowercase().as_str() {
            "normal" => Some(OverflowWrap::Normal),
            "break-word" => Some(OverflowWrap::BreakWord),
            "anywhere" => Some(OverflowWrap::Anywhere),
            _ => None,
        },
        _ => None,
    }
}

/// Parses `word-break`: `normal`, `break-all`, `keep-all`.
#[must_use]
pub fn parse_word_break(tokens: &[Token]) -> Option<WordBreak> {
    match tokens {
        [Token::Ident(name)] => match name.to_ascii_lowercase().as_str() {
            "normal" => Some(WordBreak::Normal),
            "break-all" => Some(WordBreak::BreakAll),
            "keep-all" => Some(WordBreak::KeepAll),
            _ => None,
        },
        _ => None,
    }
}

/// Applies declaration to [`TextAdvanceStyle`].
#[must_use]
pub fn apply(
    style: TextAdvanceStyle,
    property: &str,
    tokens: &[Token],
) -> Option<TextAdvanceStyle> {
    match property {
        "font-weight" => parse_font_weight_with_parent(tokens, style.font_weight())
            .map(|val| style.with_font_weight(val)),
        "font-style" => parse_font_style(tokens).map(|val| style.with_font_style(val)),
        "line-height" => parse_line_height(tokens).map(|val| style.with_line_height(val)),
        "letter-spacing" => parse_letter_spacing(tokens).map(|val| style.with_letter_spacing(val)),
        "word-spacing" => parse_word_spacing(tokens).map(|val| style.with_word_spacing(val)),
        "text-decoration-line" => parse_text_decoration_line(tokens)
            .map(|val| style.with_text_decoration(style.text_decoration().with_line(val))),
        "text-decoration-color" => parse_text_decoration_color(tokens)
            .map(|val| style.with_text_decoration(style.text_decoration().with_color(val))),
        "text-decoration-style" => parse_text_decoration_style(tokens)
            .map(|val| style.with_text_decoration(style.text_decoration().with_style(val))),
        "text-decoration" => {
            parse_text_decoration(tokens).map(|val| style.with_text_decoration(val))
        }
        "text-transform" => parse_text_transform(tokens).map(|val| style.with_text_transform(val)),
        "text-overflow" => parse_text_overflow(tokens).map(|val| style.with_text_overflow(val)),
        "overflow-wrap" | "word-wrap" => {
            parse_overflow_wrap(tokens).map(|val| style.with_overflow_wrap(val))
        }
        "word-break" => parse_word_break(tokens).map(|val| style.with_word_break(val)),
        _ => None,
    }
}

/// Resets property on [`TextAdvanceStyle`] to its CSS `initial` value.
#[must_use]
pub fn reset(style: TextAdvanceStyle, property: &str) -> Option<TextAdvanceStyle> {
    copy_from(style, TextAdvanceStyle::initial(), property)
}

/// Inherits property value from `parent`.
#[must_use]
pub fn inherit(
    style: TextAdvanceStyle,
    parent: TextAdvanceStyle,
    property: &str,
) -> Option<TextAdvanceStyle> {
    copy_from(style, parent, property)
}

fn copy_from(
    style: TextAdvanceStyle,
    source: TextAdvanceStyle,
    property: &str,
) -> Option<TextAdvanceStyle> {
    match property {
        "font-weight" => Some(style.with_font_weight(source.font_weight())),
        "font-style" => Some(style.with_font_style(source.font_style())),
        "line-height" => Some(style.with_line_height(source.line_height())),
        "letter-spacing" => Some(style.with_letter_spacing(source.letter_spacing())),
        "word-spacing" => Some(style.with_word_spacing(source.word_spacing())),
        "text-decoration-line" => Some(
            style.with_text_decoration(
                style
                    .text_decoration()
                    .with_line(source.text_decoration().line()),
            ),
        ),
        "text-decoration-color" => Some(
            style.with_text_decoration(
                style
                    .text_decoration()
                    .with_color(source.text_decoration().color()),
            ),
        ),
        "text-decoration-style" => Some(
            style.with_text_decoration(
                style
                    .text_decoration()
                    .with_style(source.text_decoration().style()),
            ),
        ),
        "text-decoration" => Some(style.with_text_decoration(source.text_decoration())),
        "text-transform" => Some(style.with_text_transform(source.text_transform())),
        "text-overflow" => Some(style.with_text_overflow(source.text_overflow())),
        "overflow-wrap" | "word-wrap" => Some(style.with_overflow_wrap(source.overflow_wrap())),
        "word-break" => Some(style.with_word_break(source.word_break())),
        _ => None,
    }
}
