//! Cascade application for border visual properties.

use crate::domain::computed::visual::VisualStyle;
use crate::infrastructure::parser::token::Token;

use super::border::{parse_border_color, parse_border_radius_corner, parse_border_style};
use super::border_shorthand::{
    parse_border_color_shorthand, parse_border_radius_shorthand, parse_border_style_shorthand,
};

#[must_use]
pub fn apply_border(style: VisualStyle, property: &str, tokens: &[Token]) -> Option<VisualStyle> {
    match property {
        "border-style" => {
            parse_border_style_shorthand(tokens).map(|val| style.with_border_styles(val))
        }
        "border-top-style" => parse_border_style(tokens)
            .map(|val| style.with_border_styles(style.border_styles().with_top(val))),
        "border-right-style" => parse_border_style(tokens)
            .map(|val| style.with_border_styles(style.border_styles().with_right(val))),
        "border-bottom-style" => parse_border_style(tokens)
            .map(|val| style.with_border_styles(style.border_styles().with_bottom(val))),
        "border-left-style" => parse_border_style(tokens)
            .map(|val| style.with_border_styles(style.border_styles().with_left(val))),
        "border-color" => {
            parse_border_color_shorthand(tokens).map(|val| style.with_border_colors(val))
        }
        "border-top-color" => parse_border_color(tokens)
            .map(|val| style.with_border_colors(style.border_colors().with_top(val))),
        "border-right-color" => parse_border_color(tokens)
            .map(|val| style.with_border_colors(style.border_colors().with_right(val))),
        "border-bottom-color" => parse_border_color(tokens)
            .map(|val| style.with_border_colors(style.border_colors().with_bottom(val))),
        "border-left-color" => parse_border_color(tokens)
            .map(|val| style.with_border_colors(style.border_colors().with_left(val))),
        "border-radius" => {
            parse_border_radius_shorthand(tokens).map(|val| style.with_border_radius(val))
        }
        "border-top-left-radius" => parse_border_radius_corner(tokens)
            .map(|val| style.with_border_radius(style.border_radius().with_top_left(val))),
        "border-top-right-radius" => parse_border_radius_corner(tokens)
            .map(|val| style.with_border_radius(style.border_radius().with_top_right(val))),
        "border-bottom-right-radius" => parse_border_radius_corner(tokens)
            .map(|val| style.with_border_radius(style.border_radius().with_bottom_right(val))),
        "border-bottom-left-radius" => parse_border_radius_corner(tokens)
            .map(|val| style.with_border_radius(style.border_radius().with_bottom_left(val))),
        _ => None,
    }
}
