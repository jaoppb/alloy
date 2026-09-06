//! Property copying for reset and inherit.

use crate::domain::computed::visual::VisualStyle;

#[must_use]
pub fn copy_property(style: VisualStyle, src: VisualStyle, prop: &str) -> Option<VisualStyle> {
    match prop {
        "border-style" => Some(style.with_border_styles(src.border_styles())),
        "border-top-style" => Some(
            style.with_border_styles(style.border_styles().with_top(src.border_styles().top())),
        ),
        "border-right-style" => Some(
            style.with_border_styles(
                style
                    .border_styles()
                    .with_right(src.border_styles().right()),
            ),
        ),
        "border-bottom-style" => Some(
            style.with_border_styles(
                style
                    .border_styles()
                    .with_bottom(src.border_styles().bottom()),
            ),
        ),
        "border-left-style" => Some(
            style.with_border_styles(style.border_styles().with_left(src.border_styles().left())),
        ),
        "border-color" => Some(style.with_border_colors(src.border_colors())),
        "border-top-color" => Some(
            style.with_border_colors(style.border_colors().with_top(src.border_colors().top())),
        ),
        "border-right-color" => Some(
            style.with_border_colors(
                style
                    .border_colors()
                    .with_right(src.border_colors().right()),
            ),
        ),
        "border-bottom-color" => Some(
            style.with_border_colors(
                style
                    .border_colors()
                    .with_bottom(src.border_colors().bottom()),
            ),
        ),
        "border-left-color" => Some(
            style.with_border_colors(style.border_colors().with_left(src.border_colors().left())),
        ),
        "border-radius" => Some(style.with_border_radius(src.border_radius())),
        "border-top-left-radius" => Some(
            style.with_border_radius(
                style
                    .border_radius()
                    .with_top_left(src.border_radius().top_left()),
            ),
        ),
        "border-top-right-radius" => Some(
            style.with_border_radius(
                style
                    .border_radius()
                    .with_top_right(src.border_radius().top_right()),
            ),
        ),
        "border-bottom-right-radius" => Some(
            style.with_border_radius(
                style
                    .border_radius()
                    .with_bottom_right(src.border_radius().bottom_right()),
            ),
        ),
        "border-bottom-left-radius" => Some(
            style.with_border_radius(
                style
                    .border_radius()
                    .with_bottom_left(src.border_radius().bottom_left()),
            ),
        ),
        "box-shadow" => Some(style.with_box_shadow(src.box_shadow())),
        "opacity" => Some(style.with_opacity(src.opacity())),
        "background-image" => Some(style.with_background_image(src.background_image())),
        "background-position" => Some(style.with_background_position(src.background_position())),
        "background-size" => Some(style.with_background_size(src.background_size())),
        "background-repeat" => Some(style.with_background_repeat(src.background_repeat())),
        _ => None,
    }
}
