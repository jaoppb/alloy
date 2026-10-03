//! Cascade application for background and visual effect properties.

use crate::domain::computed::visual::VisualStyle;
use crate::infrastructure::parser::token::Token;

use super::background::{parse_background_image, parse_background_repeat, parse_background_size};
use super::background_position::parse_background_position;
use super::opacity::parse_opacity;
use super::shadow::parse_box_shadow;

#[must_use]
pub fn apply_background_and_effects(
    style: VisualStyle,
    property: &str,
    tokens: &[Token],
) -> Option<VisualStyle> {
    match property {
        "background-image" => {
            parse_background_image(tokens).map(|image| style.with_background_image(image))
        }
        "background-position" => parse_background_position(tokens)
            .map(|position| style.with_background_position(position)),
        "background-size" => {
            parse_background_size(tokens).map(|size| style.with_background_size(size))
        }
        "background-repeat" => {
            parse_background_repeat(tokens).map(|repeat| style.with_background_repeat(repeat))
        }
        "box-shadow" => parse_box_shadow(tokens).map(|shadows| style.with_box_shadow(shadows)),
        "opacity" => parse_opacity(tokens).map(|opacity| style.with_opacity(opacity)),
        _ => None,
    }
}
