//! Browser chrome theming and UI styling.
//!
//! Centralizes colors, dimensions, and visual properties rendered to the user
//! (such as the omnibox navigation chrome), allowing dynamic styling and future
//! theme customization (e.g. dark mode, user themes).

use graphics::{Au, Color, Point, Px, Rect, Size};

/// Strongly-typed height of the browser chrome bar at the top of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChromeHeight(Au);

impl ChromeHeight {
    /// Default chrome height (42 px).
    pub const DEFAULT: Self = Self(DEFAULT_CHROME_HEIGHT);

    /// Create a new chrome height from app units.
    #[must_use]
    pub const fn from_au(height: Au) -> Self {
        Self(height)
    }

    /// Create a new chrome height from pixels.
    #[must_use]
    pub fn from_px(height: Px) -> Self {
        Self(Au::from_px(height).unwrap_or(DEFAULT_CHROME_HEIGHT))
    }

    /// Create a new chrome height from whole pixel count.
    #[must_use]
    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
    pub fn from_pixels(pixels: u32) -> Self {
        let px = Px::new(pixels as f32);
        Self::from_px(px)
    }

    /// The chrome height in app units.
    #[must_use]
    pub const fn to_au(self) -> Au {
        self.0
    }

    /// The chrome height in pixels.
    #[must_use]
    pub fn to_px(self) -> Px {
        self.0.to_px()
    }

    /// The chrome height in whole pixels.
    #[must_use]
    #[allow(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn to_pixels(self) -> u32 {
        self.to_px().get().max(0.0) as u32
    }
}

impl Default for ChromeHeight {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::fmt::Display for ChromeHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}px", self.to_px().get())
    }
}

impl From<u32> for ChromeHeight {
    fn from(pixels: u32) -> Self {
        Self::from_pixels(pixels)
    }
}

impl From<ChromeHeight> for u32 {
    fn from(height: ChromeHeight) -> Self {
        height.to_pixels()
    }
}

/// Default vertical height of the chrome navigation bar in app units (42 px).
pub const DEFAULT_CHROME_HEIGHT: Au = Au::from_raw(42 * graphics::AU_PER_PX);

/// Default height of the omnibox input box in app units (28 px).
pub const INPUT_BOX_HEIGHT: Au = Au::from_raw(28 * graphics::AU_PER_PX);

/// Horizontal margin on left and right sides of the input box in app units (12 px).
pub const INPUT_BOX_MARGIN_X: Au = Au::from_raw(12 * graphics::AU_PER_PX);

/// Corner radius of the input box (6 px), per specification.
pub const INPUT_BOX_RADIUS: Px = Px::new(6.0);

/// Thickness of the active focus border in app units (2 px).
pub const FOCUS_BORDER_WIDTH: Au = Au::from_raw(2 * graphics::AU_PER_PX);

/// Inner corner radius when focused (4 px = 6 px - 2 px).
pub const FOCUS_INNER_RADIUS: Px = Px::new(4.0);

/// Horizontal padding inside the input box before text begins in app units (10 px).
pub const TEXT_PADDING_X: Au = Au::from_raw(10 * graphics::AU_PER_PX);

/// Width of the vertical cursor caret in app units (2 px).
pub const CARET_WIDTH: Au = Au::from_raw(2 * graphics::AU_PER_PX);

/// Background color of the chrome top bar (#F3F4F6).
pub const CHROME_BACKGROUND_COLOR: Color = Color::rgb(243, 244, 246);

/// Background color of the omnibox input field (white).
pub const INPUT_BACKGROUND_COLOR: Color = Color::WHITE;

/// Active focus border ring color (#3B82F6).
pub const FOCUS_BORDER_COLOR: Color = Color::rgb(59, 130, 246);

/// Primary text color for typed input or active URL (#111827).
pub const TEXT_COLOR: Color = Color::rgb(17, 24, 39);

/// Color of the vertical insertion caret.
pub const CARET_COLOR: Color = Color::rgb(17, 24, 39);

/// Visual styling configuration for the browser omnibox navigation chrome.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OmniboxTheme {
    chrome_height: ChromeHeight,
    input_box_height: Au,
    input_box_margin_x: Au,
    input_box_radius: Px,
    focus_border_width: Au,
    focus_inner_radius: Px,
    text_padding_x: Au,
    caret_width: Au,
    chrome_background_color: Color,
    input_background_color: Color,
    focus_border_color: Color,
    text_color: Color,
    caret_color: Color,
}

impl OmniboxTheme {
    /// Default styling theme for the omnibox chrome.
    pub const DEFAULT: Self = Self {
        chrome_height: ChromeHeight::DEFAULT,
        input_box_height: INPUT_BOX_HEIGHT,
        input_box_margin_x: INPUT_BOX_MARGIN_X,
        input_box_radius: INPUT_BOX_RADIUS,
        focus_border_width: FOCUS_BORDER_WIDTH,
        focus_inner_radius: FOCUS_INNER_RADIUS,
        text_padding_x: TEXT_PADDING_X,
        caret_width: CARET_WIDTH,
        chrome_background_color: CHROME_BACKGROUND_COLOR,
        input_background_color: INPUT_BACKGROUND_COLOR,
        focus_border_color: FOCUS_BORDER_COLOR,
        text_color: TEXT_COLOR,
        caret_color: CARET_COLOR,
    };

    /// Creates a new omnibox theme with default styling.
    #[must_use]
    pub const fn new() -> Self {
        Self::DEFAULT
    }

    /// Set a custom chrome height.
    #[must_use]
    pub const fn with_chrome_height(mut self, height: ChromeHeight) -> Self {
        self.chrome_height = height;
        self
    }

    /// Set a custom input box height.
    #[must_use]
    pub const fn with_input_box_height(mut self, height: Au) -> Self {
        self.input_box_height = height;
        self
    }

    /// Set custom horizontal margins for the input box.
    #[must_use]
    pub const fn with_input_box_margin_x(mut self, margin: Au) -> Self {
        self.input_box_margin_x = margin;
        self
    }

    /// Set a custom corner radius for the input box.
    #[must_use]
    pub const fn with_input_box_radius(mut self, radius: Px) -> Self {
        self.input_box_radius = radius;
        self
    }

    /// Set a custom focus border thickness.
    #[must_use]
    pub const fn with_focus_border_width(mut self, width: Au) -> Self {
        self.focus_border_width = width;
        self
    }

    /// Set a custom focus inner radius.
    #[must_use]
    pub const fn with_focus_inner_radius(mut self, radius: Px) -> Self {
        self.focus_inner_radius = radius;
        self
    }

    /// Set custom horizontal text padding.
    #[must_use]
    pub const fn with_text_padding_x(mut self, padding: Au) -> Self {
        self.text_padding_x = padding;
        self
    }

    /// Set a custom caret width.
    #[must_use]
    pub const fn with_caret_width(mut self, width: Au) -> Self {
        self.caret_width = width;
        self
    }

    /// Set a custom chrome background color.
    #[must_use]
    pub const fn with_chrome_background_color(mut self, color: Color) -> Self {
        self.chrome_background_color = color;
        self
    }

    /// Set a custom input field background color.
    #[must_use]
    pub const fn with_input_background_color(mut self, color: Color) -> Self {
        self.input_background_color = color;
        self
    }

    /// Set a custom active focus border color.
    #[must_use]
    pub const fn with_focus_border_color(mut self, color: Color) -> Self {
        self.focus_border_color = color;
        self
    }

    /// Set a custom text color.
    #[must_use]
    pub const fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set a custom cursor caret color.
    #[must_use]
    pub const fn with_caret_color(mut self, color: Color) -> Self {
        self.caret_color = color;
        self
    }

    /// Returns the chrome navigation bar height.
    #[must_use]
    pub const fn chrome_height(&self) -> ChromeHeight {
        self.chrome_height
    }

    /// Returns the input box height.
    #[must_use]
    pub const fn input_box_height(&self) -> Au {
        self.input_box_height
    }

    /// Returns the input box horizontal margin.
    #[must_use]
    pub const fn input_box_margin_x(&self) -> Au {
        self.input_box_margin_x
    }

    /// Returns the input box corner radius.
    #[must_use]
    pub const fn input_box_radius(&self) -> Px {
        self.input_box_radius
    }

    /// Returns the active focus border width.
    #[must_use]
    pub const fn focus_border_width(&self) -> Au {
        self.focus_border_width
    }

    /// Returns the inner corner radius when focused.
    #[must_use]
    pub const fn focus_inner_radius(&self) -> Px {
        self.focus_inner_radius
    }

    /// Returns the horizontal text padding.
    #[must_use]
    pub const fn text_padding_x(&self) -> Au {
        self.text_padding_x
    }

    /// Returns the cursor caret width.
    #[must_use]
    pub const fn caret_width(&self) -> Au {
        self.caret_width
    }

    /// Returns the chrome background color.
    #[must_use]
    pub const fn chrome_background_color(&self) -> Color {
        self.chrome_background_color
    }

    /// Returns the input field background color.
    #[must_use]
    pub const fn input_background_color(&self) -> Color {
        self.input_background_color
    }

    /// Returns the active focus border ring color.
    #[must_use]
    pub const fn focus_border_color(&self) -> Color {
        self.focus_border_color
    }

    /// Returns the primary text color.
    #[must_use]
    pub const fn text_color(&self) -> Color {
        self.text_color
    }

    /// Returns the insertion cursor caret color.
    #[must_use]
    pub const fn caret_color(&self) -> Color {
        self.caret_color
    }

    /// Where the input box sits inside a chrome bar `chrome_width` wide, or
    /// `None` when the margins leave no room for it. The painter draws the box
    /// here and the UI render tree hit-tests it here, so the two cannot drift.
    #[must_use]
    pub fn input_box_bounds(&self, chrome_width: Au) -> Option<Rect> {
        let margin_x = self.input_box_margin_x;
        let double_margin = margin_x.saturating_add(margin_x);
        let input_width = chrome_width.saturating_sub(double_margin);
        if input_width.is_zero() {
            return None;
        }
        let input_size = Size::new(input_width, self.input_box_height)?;
        let chrome_height_au = self.chrome_height.to_au();
        let vertical_space = chrome_height_au.saturating_sub(self.input_box_height);
        let half_vertical_space = vertical_space.raw().checked_div(2).unwrap_or(0);
        let vertical_offset = Au::from_raw(half_vertical_space);
        let origin = Point::new(margin_x, vertical_offset);
        Some(Rect::new(origin, input_size))
    }
}

impl Default for OmniboxTheme {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Browser chrome theme containing visual styling configuration for UI elements.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Theme {
    /// Omnibox navigation chrome theme.
    pub omnibox: OmniboxTheme,
}

impl Theme {
    /// Default browser theme.
    pub const DEFAULT: Self = Self {
        omnibox: OmniboxTheme::DEFAULT,
    };

    /// Creates a new default browser theme.
    #[must_use]
    pub const fn new() -> Self {
        Self::DEFAULT
    }

    /// Customizes the omnibox theme.
    #[must_use]
    pub const fn with_omnibox(mut self, omnibox: OmniboxTheme) -> Self {
        self.omnibox = omnibox;
        self
    }

    /// Returns the omnibox theme.
    #[must_use]
    pub const fn omnibox(&self) -> &OmniboxTheme {
        &self.omnibox
    }
}
