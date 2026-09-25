//! Visual decoration properties: borders, shadows, opacity, and backgrounds.

#[path = "visual/background_image.rs"]
pub mod background_image;
#[path = "visual/background_position.rs"]
pub mod background_position;
#[path = "visual/background_repeat.rs"]
pub mod background_repeat;
#[path = "visual/background_size.rs"]
pub mod background_size;
#[path = "visual/border_color.rs"]
pub mod border_color;
#[path = "visual/border_radius.rs"]
pub mod border_radius;
#[path = "visual/border_style.rs"]
pub mod border_style;
#[path = "visual/border_style_edges.rs"]
pub mod border_style_edges;
#[path = "visual/opacity.rs"]
pub mod opacity;
#[path = "visual/shadow.rs"]
pub mod shadow;
#[path = "visual/shadow_list.rs"]
pub mod shadow_list;
#[path = "visual/style.rs"]
pub mod style;

pub use background_image::{BackgroundImage, ImageSource};
pub use background_position::BackgroundPosition;
pub use background_repeat::BackgroundRepeat;
pub use background_size::BackgroundSize;
pub use border_color::BorderColorEdges;
pub use border_radius::BorderRadius;
pub use border_style::BorderStyle;
pub use border_style_edges::BorderStyleEdges;
pub use opacity::Opacity;
pub use shadow::{BoxShadow, ShadowPlacement};
pub use shadow_list::BoxShadowList;
pub use style::VisualStyle;
