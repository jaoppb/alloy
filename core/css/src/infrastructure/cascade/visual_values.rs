//! Cascade values and parsers for visual decoration properties.

#[path = "visual_values/apply.rs"]
pub mod apply;
#[path = "visual_values/apply_background.rs"]
pub mod apply_background;
#[path = "visual_values/apply_border.rs"]
pub mod apply_border;
#[path = "visual_values/background.rs"]
pub mod background;
#[path = "visual_values/background_position.rs"]
pub mod background_position;
#[path = "visual_values/border.rs"]
pub mod border;
#[path = "visual_values/border_shorthand.rs"]
pub mod border_shorthand;
#[path = "visual_values/copy.rs"]
pub mod copy;
#[path = "visual_values/helpers.rs"]
pub mod helpers;
#[path = "visual_values/hex.rs"]
pub mod hex;
#[path = "visual_values/opacity.rs"]
pub mod opacity;
#[path = "visual_values/shadow.rs"]
pub mod shadow;

pub use apply::{apply_visual_property, inherit_visual_property, reset_visual_property};
pub use background::{parse_background_image, parse_background_repeat, parse_background_size};
pub use background_position::parse_background_position;
pub use border::{parse_border_color, parse_border_radius_corner, parse_border_style};
pub use border_shorthand::{
    parse_border_color_shorthand, parse_border_radius_shorthand, parse_border_style_shorthand,
};
pub use opacity::parse_opacity;
pub use shadow::parse_box_shadow;
