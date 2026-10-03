//! Property copying for reset and inherit.

use crate::domain::computed::visual::VisualStyle;

#[must_use]
pub fn copy_property(
    style: VisualStyle,
    source: VisualStyle,
    property: &str,
) -> Option<VisualStyle> {
    match property {
        "border-style" => Some(style.with_border_styles(source.border_styles())),
        "border-top-style" => Some(
            style.with_border_styles(style.border_styles().with_top(source.border_styles().top())),
        ),
        "border-right-style" => Some(
            style.with_border_styles(
                style
                    .border_styles()
                    .with_right(source.border_styles().right()),
            ),
        ),
        "border-bottom-style" => Some(
            style.with_border_styles(
                style
                    .border_styles()
                    .with_bottom(source.border_styles().bottom()),
            ),
        ),
        "border-left-style" => Some(
            style.with_border_styles(
                style
                    .border_styles()
                    .with_left(source.border_styles().left()),
            ),
        ),
        "border-color" => Some(style.with_border_colors(source.border_colors())),
        "border-top-color" => Some(
            style.with_border_colors(style.border_colors().with_top(source.border_colors().top())),
        ),
        "border-right-color" => Some(
            style.with_border_colors(
                style
                    .border_colors()
                    .with_right(source.border_colors().right()),
            ),
        ),
        "border-bottom-color" => Some(
            style.with_border_colors(
                style
                    .border_colors()
                    .with_bottom(source.border_colors().bottom()),
            ),
        ),
        "border-left-color" => Some(
            style.with_border_colors(
                style
                    .border_colors()
                    .with_left(source.border_colors().left()),
            ),
        ),
        "border-radius" => Some(style.with_border_radius(source.border_radius())),
        "border-top-left-radius" => Some(
            style.with_border_radius(
                style
                    .border_radius()
                    .with_top_left(source.border_radius().top_left()),
            ),
        ),
        "border-top-right-radius" => Some(
            style.with_border_radius(
                style
                    .border_radius()
                    .with_top_right(source.border_radius().top_right()),
            ),
        ),
        "border-bottom-right-radius" => Some(
            style.with_border_radius(
                style
                    .border_radius()
                    .with_bottom_right(source.border_radius().bottom_right()),
            ),
        ),
        "border-bottom-left-radius" => Some(
            style.with_border_radius(
                style
                    .border_radius()
                    .with_bottom_left(source.border_radius().bottom_left()),
            ),
        ),
        "box-shadow" => Some(style.with_box_shadow(source.box_shadow())),
        "opacity" => Some(style.with_opacity(source.opacity())),
        "background-image" => Some(style.with_background_image(source.background_image())),
        "background-position" => Some(style.with_background_position(source.background_position())),
        "background-size" => Some(style.with_background_size(source.background_size())),
        "background-repeat" => Some(style.with_background_repeat(source.background_repeat())),
        _ => None,
    }
}
