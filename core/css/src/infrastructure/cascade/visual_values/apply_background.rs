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
            parse_background_image(tokens).map(|val| style.with_background_image(val))
        }
        "background-position" => {
            parse_background_position(tokens).map(|val| style.with_background_position(val))
        }
        "background-size" => {
            parse_background_size(tokens).map(|val| style.with_background_size(val))
        }
        "background-repeat" => {
            parse_background_repeat(tokens).map(|val| style.with_background_repeat(val))
        }
        "box-shadow" => parse_box_shadow(tokens).map(|val| style.with_box_shadow(val)),
        "opacity" => parse_opacity(tokens).map(|val| style.with_opacity(val)),
        _ => None,
    }
}
