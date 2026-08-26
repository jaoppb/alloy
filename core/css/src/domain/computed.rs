use crate::domain::property::{Color, DisplayType, Px};

/// Resolved and computed CSS properties for a DOM node.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    display: DisplayType,
    color: Color,
    background_color: Color,
    font_size: Px,
    width: Option<Px>,
    height: Option<Px>,
    margin_top: Px,
    margin_right: Px,
    margin_bottom: Px,
    margin_left: Px,
    padding_top: Px,
    padding_right: Px,
    padding_bottom: Px,
    padding_left: Px,
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

    /// Returns the resolved display formatting type.
    #[must_use]
    pub const fn display(&self) -> DisplayType {
        self.display
    }

    /// Sets the display formatting type.
    pub fn set_display(&mut self, display: DisplayType) {
        self.display = display;
    }

    /// Returns the resolved foreground text color.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    /// Sets the foreground text color.
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Returns the resolved background color.
    #[must_use]
    pub const fn background_color(&self) -> Color {
        self.background_color
    }

    /// Sets the background color.
    pub fn set_background_color(&mut self, background_color: Color) {
        self.background_color = background_color;
    }

    /// Returns the font size in pixels.
    #[must_use]
    pub const fn font_size(&self) -> Px {
        self.font_size
    }

    /// Sets the font size in pixels.
    pub fn set_font_size(&mut self, font_size: Px) {
        self.font_size = font_size;
    }

    /// Returns the explicit width, if set.
    #[must_use]
    pub const fn width(&self) -> Option<Px> {
        self.width
    }

    /// Sets the explicit width.
    pub fn set_width(&mut self, width: Option<Px>) {
        self.width = width;
    }

    /// Returns the explicit height, if set.
    #[must_use]
    pub const fn height(&self) -> Option<Px> {
        self.height
    }

    /// Sets the explicit height.
    pub fn set_height(&mut self, height: Option<Px>) {
        self.height = height;
    }

    /// Returns the top margin.
    #[must_use]
    pub const fn margin_top(&self) -> Px {
        self.margin_top
    }

    /// Sets the top margin.
    pub fn set_margin_top(&mut self, margin_top: Px) {
        self.margin_top = margin_top;
    }

    /// Returns the right margin.
    #[must_use]
    pub const fn margin_right(&self) -> Px {
        self.margin_right
    }

    /// Sets the right margin.
    pub fn set_margin_right(&mut self, margin_right: Px) {
        self.margin_right = margin_right;
    }

    /// Returns the bottom margin.
    #[must_use]
    pub const fn margin_bottom(&self) -> Px {
        self.margin_bottom
    }

    /// Sets the bottom margin.
    pub fn set_margin_bottom(&mut self, margin_bottom: Px) {
        self.margin_bottom = margin_bottom;
    }

    /// Returns the left margin.
    #[must_use]
    pub const fn margin_left(&self) -> Px {
        self.margin_left
    }

    /// Sets the left margin.
    pub fn set_margin_left(&mut self, margin_left: Px) {
        self.margin_left = margin_left;
    }

    /// Returns the top padding.
    #[must_use]
    pub const fn padding_top(&self) -> Px {
        self.padding_top
    }

    /// Sets the top padding.
    pub fn set_padding_top(&mut self, padding_top: Px) {
        self.padding_top = padding_top;
    }

    /// Returns the right padding.
    #[must_use]
    pub const fn padding_right(&self) -> Px {
        self.padding_right
    }

    /// Sets the right padding.
    pub fn set_padding_right(&mut self, padding_right: Px) {
        self.padding_right = padding_right;
    }

    /// Returns the bottom padding.
    #[must_use]
    pub const fn padding_bottom(&self) -> Px {
        self.padding_bottom
    }

    /// Sets the bottom padding.
    pub fn set_padding_bottom(&mut self, padding_bottom: Px) {
        self.padding_bottom = padding_bottom;
    }

    /// Returns the left padding.
    #[must_use]
    pub const fn padding_left(&self) -> Px {
        self.padding_left
    }

    /// Sets the left padding.
    pub fn set_padding_left(&mut self, padding_left: Px) {
        self.padding_left = padding_left;
    }
}
