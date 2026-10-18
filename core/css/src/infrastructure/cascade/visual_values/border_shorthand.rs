//! Border shorthand parsers.

use crate::domain::computed::visual::{
    BorderColor, BorderColorEdges, BorderRadius, BorderStyle, BorderStyleEdges,
};
use crate::domain::length::Length;
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::{component_values, parse_length};

use super::border::{parse_border_color, parse_border_radius_corner, parse_border_style};

/// The components of the `border` shorthand declaration.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ParsedBorderShorthand {
    pub width: Length,
    pub style: BorderStyle,
    pub color: BorderColor,
}

#[derive(Default)]
struct BorderShorthandAccumulator {
    width: Option<Length>,
    style: Option<BorderStyle>,
    color: Option<BorderColor>,
}

impl BorderShorthandAccumulator {
    fn absorb(mut self, component: &[Token]) -> Option<Self> {
        if let Some(style) = parse_border_style(component) {
            if self.style.is_some() {
                return None;
            }
            self.style = Some(style);
            return Some(self);
        }
        if let Some(width) = parse_border_width_component(component) {
            if self.width.is_some() {
                return None;
            }
            self.width = Some(width);
            return Some(self);
        }
        if let Some(color) = parse_border_color(component) {
            if self.color.is_some() {
                return None;
            }
            self.color = Some(color);
            return Some(self);
        }
        None
    }

    fn build(self) -> Option<ParsedBorderShorthand> {
        if self.width.is_none() && self.style.is_none() && self.color.is_none() {
            return None;
        }
        let style = self.style.unwrap_or(BorderStyle::None);
        let color = self.color.unwrap_or(BorderColor::CurrentColor);
        let width = self.width.unwrap_or_else(|| {
            if style.is_none_or_hidden() {
                Length::ZERO
            } else {
                Length::Pixels(3.0)
            }
        });
        Some(ParsedBorderShorthand {
            width,
            style,
            color,
        })
    }
}

fn parse_border_width_component(tokens: &[Token]) -> Option<Length> {
    match tokens {
        [Token::Ident(name)] if name.eq_ignore_ascii_case("thin") => Some(Length::Pixels(1.0)),
        [Token::Ident(name)] if name.eq_ignore_ascii_case("medium") => Some(Length::Pixels(3.0)),
        [Token::Ident(name)] if name.eq_ignore_ascii_case("thick") => Some(Length::Pixels(5.0)),
        _ => parse_length(tokens),
    }
}

/// `border`: `<line-width> || <line-style> || <color>` (CSS Backgrounds & Borders L3 §4.3).
#[must_use]
pub fn parse_border_shorthand(tokens: &[Token]) -> Option<ParsedBorderShorthand> {
    if tokens.is_empty() {
        return None;
    }
    let accumulator = component_values(tokens).try_fold(
        BorderShorthandAccumulator::default(),
        BorderShorthandAccumulator::absorb,
    )?;
    accumulator.build()
}

/// `border-style` (CSS Backgrounds & Borders L3 §4.2): one to four styles.
#[must_use]
pub fn parse_border_style_shorthand(tokens: &[Token]) -> Option<BorderStyleEdges> {
    let [top, right, bottom, left] = expand_one_to_four(tokens, parse_border_style)?;
    Some(BorderStyleEdges::new(top, right, bottom, left))
}

/// `border-color` (same spec, §4.1): one to four colours. Each colour is one
/// component value, so a multi-token `rgba(…)` counts as a single colour.
#[must_use]
pub fn parse_border_color_shorthand(tokens: &[Token]) -> Option<BorderColorEdges> {
    let [top, right, bottom, left] = expand_one_to_four(tokens, parse_border_color)?;
    Some(BorderColorEdges::new(top, right, bottom, left))
}

/// `border-radius` (same spec, §5.1), circular radii only: one to four corner
/// radii in top-left, top-right, bottom-right, bottom-left order.
#[must_use]
pub fn parse_border_radius_shorthand(tokens: &[Token]) -> Option<BorderRadius> {
    let [top_left, top_right, bottom_right, bottom_left] =
        expand_one_to_four(tokens, parse_border_radius_corner)?;
    Some(BorderRadius::new(
        top_left,
        top_right,
        bottom_right,
        bottom_left,
    ))
}

/// The one-to-four-value expansion every box shorthand shares (CSS Box Model
/// §8.3, CSS Backgrounds & Borders L3 §5.1): one value fills all four sides,
/// two are vertical / horizontal, three are top / horizontal / bottom, four
/// are clockwise from the top (top-left for corners).
fn expand_one_to_four<T: Copy>(
    tokens: &[Token],
    parse_component: impl Fn(&[Token]) -> Option<T>,
) -> Option<[T; 4]> {
    let values: Option<Vec<T>> = component_values(tokens).map(parse_component).collect();
    match values?.as_slice() {
        [all] => Some([*all; 4]),
        [vertical, horizontal] => Some([*vertical, *horizontal, *vertical, *horizontal]),
        [top, horizontal, bottom] => Some([*top, *horizontal, *bottom, *horizontal]),
        [top, right, bottom, left] => Some([*top, *right, *bottom, *left]),
        _ => None,
    }
}
