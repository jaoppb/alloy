//! Border shorthand parsers.

use crate::domain::computed::visual::{BorderColorEdges, BorderRadius, BorderStyleEdges};
use crate::infrastructure::parser::token::Token;
use crate::infrastructure::parser::values::component_values;

use super::border::{parse_border_color, parse_border_radius_corner, parse_border_style};

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
