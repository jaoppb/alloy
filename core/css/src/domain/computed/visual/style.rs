//! [`VisualStyle`] — the visual decorations sub-aggregate of computed style.

use super::background_image::BackgroundImage;
use super::background_position::BackgroundPosition;
use super::background_repeat::BackgroundRepeat;
use super::background_size::BackgroundSize;
use super::border_color::BorderColorEdges;
use super::border_radius::BorderRadius;
use super::border_style_edges::BorderStyleEdges;
use super::opacity::Opacity;
use super::shadow_list::BoxShadowList;

/// A node's visual decoration properties grouped into one aggregate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct VisualStyle {
    border_styles: BorderStyleEdges,
    border_colors: BorderColorEdges,
    border_radius: BorderRadius,
    box_shadow: BoxShadowList,
    opacity: Opacity,
    background_image: BackgroundImage,
    background_position: BackgroundPosition,
    background_size: BackgroundSize,
    background_repeat: BackgroundRepeat,
}

impl VisualStyle {
    /// Every visual decoration property at its CSS `initial` value.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            border_styles: BorderStyleEdges::NONE,
            border_colors: BorderColorEdges::BLACK,
            border_radius: BorderRadius::ZERO,
            box_shadow: BoxShadowList::none(),
            opacity: Opacity::ONE,
            background_image: BackgroundImage::None,
            background_position: BackgroundPosition::INITIAL,
            background_size: BackgroundSize::Auto,
            background_repeat: BackgroundRepeat::Repeat,
        }
    }

    #[must_use]
    pub const fn border_styles(self) -> BorderStyleEdges {
        self.border_styles
    }

    #[must_use]
    pub const fn border_colors(self) -> BorderColorEdges {
        self.border_colors
    }

    #[must_use]
    pub const fn border_radius(self) -> BorderRadius {
        self.border_radius
    }

    #[must_use]
    pub const fn box_shadow(self) -> BoxShadowList {
        self.box_shadow
    }

    #[must_use]
    pub const fn opacity(self) -> Opacity {
        self.opacity
    }

    #[must_use]
    pub const fn background_image(self) -> BackgroundImage {
        self.background_image
    }

    #[must_use]
    pub const fn background_position(self) -> BackgroundPosition {
        self.background_position
    }

    #[must_use]
    pub const fn background_size(self) -> BackgroundSize {
        self.background_size
    }

    #[must_use]
    pub const fn background_repeat(self) -> BackgroundRepeat {
        self.background_repeat
    }

    #[must_use]
    pub const fn with_border_styles(self, border_styles: BorderStyleEdges) -> Self {
        Self {
            border_styles,
            ..self
        }
    }

    #[must_use]
    pub const fn with_border_colors(self, border_colors: BorderColorEdges) -> Self {
        Self {
            border_colors,
            ..self
        }
    }

    #[must_use]
    pub const fn with_border_radius(self, border_radius: BorderRadius) -> Self {
        Self {
            border_radius,
            ..self
        }
    }

    #[must_use]
    pub const fn with_box_shadow(self, box_shadow: BoxShadowList) -> Self {
        Self { box_shadow, ..self }
    }

    #[must_use]
    pub const fn with_opacity(self, opacity: Opacity) -> Self {
        Self { opacity, ..self }
    }

    #[must_use]
    pub const fn with_background_image(self, background_image: BackgroundImage) -> Self {
        Self {
            background_image,
            ..self
        }
    }

    #[must_use]
    pub const fn with_background_position(self, background_position: BackgroundPosition) -> Self {
        Self {
            background_position,
            ..self
        }
    }

    #[must_use]
    pub const fn with_background_size(self, background_size: BackgroundSize) -> Self {
        Self {
            background_size,
            ..self
        }
    }

    #[must_use]
    pub const fn with_background_repeat(self, background_repeat: BackgroundRepeat) -> Self {
        Self {
            background_repeat,
            ..self
        }
    }
}
