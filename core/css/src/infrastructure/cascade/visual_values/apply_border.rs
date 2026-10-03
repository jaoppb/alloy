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
            parse_border_style_shorthand(tokens).map(|styles| style.with_border_styles(styles))
        }
        "border-top-style" => parse_border_style(tokens).map(|border_style| {
            style.with_border_styles(style.border_styles().with_top(border_style))
        }),
        "border-right-style" => parse_border_style(tokens).map(|border_style| {
            style.with_border_styles(style.border_styles().with_right(border_style))
        }),
        "border-bottom-style" => parse_border_style(tokens).map(|border_style| {
            style.with_border_styles(style.border_styles().with_bottom(border_style))
        }),
        "border-left-style" => parse_border_style(tokens).map(|border_style| {
            style.with_border_styles(style.border_styles().with_left(border_style))
        }),
        "border-color" => {
            parse_border_color_shorthand(tokens).map(|colors| style.with_border_colors(colors))
        }
        "border-top-color" => parse_border_color(tokens)
            .map(|color| style.with_border_colors(style.border_colors().with_top(color))),
        "border-right-color" => parse_border_color(tokens)
            .map(|color| style.with_border_colors(style.border_colors().with_right(color))),
        "border-bottom-color" => parse_border_color(tokens)
            .map(|color| style.with_border_colors(style.border_colors().with_bottom(color))),
        "border-left-color" => parse_border_color(tokens)
            .map(|color| style.with_border_colors(style.border_colors().with_left(color))),
        "border-radius" => {
            parse_border_radius_shorthand(tokens).map(|radius| style.with_border_radius(radius))
        }
        "border-top-left-radius" => parse_border_radius_corner(tokens)
            .map(|radius| style.with_border_radius(style.border_radius().with_top_left(radius))),
        "border-top-right-radius" => parse_border_radius_corner(tokens)
            .map(|radius| style.with_border_radius(style.border_radius().with_top_right(radius))),
        "border-bottom-right-radius" => parse_border_radius_corner(tokens).map(|radius| {
            style.with_border_radius(style.border_radius().with_bottom_right(radius))
        }),
        "border-bottom-left-radius" => parse_border_radius_corner(tokens)
            .map(|radius| style.with_border_radius(style.border_radius().with_bottom_left(radius))),
        _ => None,
    }
}
