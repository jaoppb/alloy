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
use crate::infrastructure::parser::values::{component_values, parse_color, parse_length};

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

/// Parses `font-weight`, resolving `bolder` / `lighter` against `parent_weight`.
///
/// `parent_weight` is the **parent's** computed weight (CSS Fonts 4 §2.2),
/// never the element's own in-progress one, which a lower-precedence rule such
/// as the UA's `h1 { font-weight: bold }` may already have raised.
#[must_use]
pub fn parse_font_weight_with_parent(
    tokens: &[Token],
    parent_weight: FontWeight,
) -> Option<FontWeight> {
    let [Token::Ident(name)] = tokens else {
        return parse_font_weight(tokens);
    };
    match name.to_ascii_lowercase().as_str() {
        "bolder" => Some(parent_weight.bolder()),
        "lighter" => Some(parent_weight.lighter()),
        _ => parse_font_weight(tokens),
    }
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
        [Token::Number(value)] if *value >= 0.0 => {
            if *value == 0.0 {
                return Some(LineHeight::Length(Length::ZERO));
            }
            Some(LineHeight::Number(LineHeightFactor::new(*value)))
        }
        [Token::Percentage(value)] if *value >= 0.0 => {
            Some(LineHeight::Percentage(LineHeightPercentage::new(*value)))
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
    tokens
        .iter()
        .try_fold(TextDecorationLine::NONE, |line, token| {
            Some(line.adding(decoration_line_of(core::slice::from_ref(token))?))
        })
}

/// Parses `text-decoration-style`: `solid`, `double`, `dotted`, `dashed`, `wavy`.
#[must_use]
pub fn parse_text_decoration_style(tokens: &[Token]) -> Option<TextDecorationStyle> {
    let [Token::Ident(name)] = tokens else {
        return None;
    };
    match name.to_ascii_lowercase().as_str() {
        "solid" => Some(TextDecorationStyle::Solid),
        "double" => Some(TextDecorationStyle::Double),
        "dotted" => Some(TextDecorationStyle::Dotted),
        "dashed" => Some(TextDecorationStyle::Dashed),
        "wavy" => Some(TextDecorationStyle::Wavy),
        _ => None,
    }
}

/// Parses `text-decoration-color`.
#[must_use]
pub fn parse_text_decoration_color(tokens: &[Token]) -> Option<CssColor> {
    parse_color(tokens)
}

/// The one line a component names, if it is a line keyword.
fn decoration_line_of(component: &[Token]) -> Option<TextDecorationLine> {
    let [Token::Ident(name)] = component else {
        return None;
    };
    match name.to_ascii_lowercase().as_str() {
        "underline" => Some(TextDecorationLine::UNDERLINE),
        "overline" => Some(TextDecorationLine::OVERLINE),
        "line-through" => Some(TextDecorationLine::LINE_THROUGH),
        _ => None,
    }
}

/// What the components of a `text-decoration` shorthand have set so far:
/// any number of lines, at most one style and at most one colour.
#[derive(Default)]
struct DecorationParts {
    line: TextDecorationLine,
    style: Option<TextDecorationStyle>,
    color: Option<CssColor>,
}

impl DecorationParts {
    /// Folds one component in, or `None` when it fits nowhere — not a line,
    /// not a style or colour still unset.
    fn absorb(mut self, component: &[Token]) -> Option<Self> {
        if let Some(line) = decoration_line_of(component) {
            self.line = self.line.adding(line);
            return Some(self);
        }
        if let (None, Some(style)) = (self.style, parse_text_decoration_style(component)) {
            self.style = Some(style);
            return Some(self);
        }
        if self.color.is_some() {
            return None;
        }
        self.color = Some(parse_color(component)?);
        Some(self)
    }

    fn into_decoration(self) -> TextDecoration {
        TextDecoration::initial()
            .with_line(self.line)
            .with_style(self.style.unwrap_or(TextDecorationStyle::Solid))
            .with_color(self.color.unwrap_or(CssColor::BLACK))
    }
}

/// Parses `text-decoration` shorthand: line, style, and color in any order.
///
/// Walks whole component values, so a functional colour (`rgb(255, 0, 0)`)
/// is read as the one colour it is rather than token by token.
#[must_use]
pub fn parse_text_decoration(tokens: &[Token]) -> Option<TextDecoration> {
    if tokens.is_empty() {
        return None;
    }
    if is_none_keyword(tokens) {
        return Some(TextDecoration::initial());
    }
    component_values(tokens)
        .try_fold(DecorationParts::default(), DecorationParts::absorb)
        .map(DecorationParts::into_decoration)
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

/// Applies declaration to [`TextAdvanceStyle`]. `parent_weight` is the
/// parent's computed `font-weight` ([`FontWeight::NORMAL`] at the root), the
/// one value `bolder` / `lighter` are relative to.
#[must_use]
pub fn apply(
    style: TextAdvanceStyle,
    parent_weight: FontWeight,
    property: &str,
    tokens: &[Token],
) -> Option<TextAdvanceStyle> {
    match property {
        "font-weight" => parse_font_weight_with_parent(tokens, parent_weight)
            .map(|value| style.with_font_weight(value)),
        "font-style" => parse_font_style(tokens).map(|value| style.with_font_style(value)),
        "line-height" => parse_line_height(tokens).map(|value| style.with_line_height(value)),
        "letter-spacing" => {
            parse_letter_spacing(tokens).map(|value| style.with_letter_spacing(value))
        }
        "word-spacing" => parse_word_spacing(tokens).map(|value| style.with_word_spacing(value)),
        "text-decoration-line" => parse_text_decoration_line(tokens)
            .map(|value| style.with_text_decoration(style.text_decoration().with_line(value))),
        "text-decoration-color" => parse_text_decoration_color(tokens)
            .map(|value| style.with_text_decoration(style.text_decoration().with_color(value))),
        "text-decoration-style" => parse_text_decoration_style(tokens)
            .map(|value| style.with_text_decoration(style.text_decoration().with_style(value))),
        "text-decoration" => {
            parse_text_decoration(tokens).map(|value| style.with_text_decoration(value))
        }
        "text-transform" => {
            parse_text_transform(tokens).map(|value| style.with_text_transform(value))
        }
        "text-overflow" => parse_text_overflow(tokens).map(|value| style.with_text_overflow(value)),
        "overflow-wrap" | "word-wrap" => {
            parse_overflow_wrap(tokens).map(|value| style.with_overflow_wrap(value))
        }
        "word-break" => parse_word_break(tokens).map(|value| style.with_word_break(value)),
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
