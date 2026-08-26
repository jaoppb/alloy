use crate::domain::property::{Color, DisplayType, Px};

/// Resolved and computed CSS properties for a DOM node.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    pub display: DisplayType,
    pub color: Color,
    pub background_color: Color,
    pub font_size: Px,
    pub width: Option<Px>,
    pub height: Option<Px>,
    pub margin_top: Px,
    pub margin_right: Px,
    pub margin_bottom: Px,
    pub margin_left: Px,
    pub padding_top: Px,
    pub padding_right: Px,
    pub padding_bottom: Px,
    pub padding_left: Px,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: DisplayType::Block,
            color: Color::BLACK,
            background_color: Color::TRANSPARENT,
            font_size: Px::new(16.0),
            width: None,
            height: None,
            margin_top: Px::new(0.0),
            margin_right: Px::new(0.0),
            margin_bottom: Px::new(0.0),
            margin_left: Px::new(0.0),
            padding_top: Px::new(0.0),
            padding_right: Px::new(0.0),
            padding_bottom: Px::new(0.0),
            padding_left: Px::new(0.0),
        }
    }
}

impl ComputedStyle {
    /// Inherits inheritable properties from a parent computed style.
    pub fn inherit_from(&mut self, parent: &ComputedStyle) {
        // Inheritable properties in CSS include color and font_size
        self.color = parent.color;
        self.font_size = parent.font_size;
    }
}
