//! Main cascade dispatch for visual properties.

use crate::domain::computed::visual::VisualStyle;
use crate::infrastructure::parser::token::Token;

use super::apply_background::apply_background_and_effects;
use super::apply_border::apply_border;
use super::copy::copy_property;

/// Applies a visual property to `style`, returning `None` if the property is not recognized.
#[must_use]
pub fn apply_visual_property(
    style: VisualStyle,
    property: &str,
    tokens: &[Token],
) -> Option<VisualStyle> {
    apply_border(style, property, tokens)
        .or_else(|| apply_background_and_effects(style, property, tokens))
}

/// Resets a visual property on `style` to its CSS `initial` value.
#[must_use]
pub fn reset_visual_property(style: VisualStyle, property: &str) -> Option<VisualStyle> {
    copy_property(style, VisualStyle::initial(), property)
}

/// Copies a visual property from `parent` to `style`.
#[must_use]
pub fn inherit_visual_property(
    style: VisualStyle,
    parent: &VisualStyle,
    property: &str,
) -> Option<VisualStyle> {
    copy_property(style, *parent, property)
}
