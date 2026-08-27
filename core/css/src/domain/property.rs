use std::fmt;

/// Strongly typed pixel length value object (ADR-0010).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Px(f32);

impl Px {
    /// Creates a new `Px` dimension.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    /// Returns the raw f32 value.
    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }
}

impl fmt::Display for Px {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}px", self.0)
    }
}

/// Strongly typed 32-bit RGBA color representation `0xAARRGGBB` (ADR-0010).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color(u32);

impl Color {
    /// Constructs a `Color` directly from raw u32 bits.
    #[must_use]
    pub const fn from_u32(val: u32) -> Self {
        Self(val)
    }

    /// Returns the raw u32 bits.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
    pub const BLACK: Self = Self::rgba(0, 0, 0, 255);
    pub const WHITE: Self = Self::rgba(255, 255, 255, 255);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const RED: Self = Self::rgba(255, 0, 0, 255);
    pub const GREEN: Self = Self::rgba(0, 255, 0, 255);
    pub const BLUE: Self = Self::rgba(0, 0, 255, 255);

    /// Constructs a `Color` from 8-bit RGBA channels.
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        let val = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        Self(val)
    }

    /// Constructs a `Color` from 8-bit RGB channels with full opacity (255).
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    /// Extracts the red component.
    #[must_use]
    pub const fn r(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }

    /// Extracts the green component.
    #[must_use]
    pub const fn g(self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }

    /// Extracts the blue component.
    #[must_use]
    pub const fn b(self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// Extracts the alpha component.
    #[must_use]
    pub const fn a(self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }

    /// Parses a CSS color string by delegating to the `CssColorResolver` (C-15, C-16).
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        crate::infrastructure::color_resolver::CssColorResolver::resolve_static(raw)
    }
}

/// Normalized CSS property name mapped to W3C CSS3 properties with extensible custom property fallback (C-17).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PropertyName {
    // Colors & Backgrounds
    Color,
    BackgroundColor,

    // Box Model & Display
    Display,
    Width,
    Height,
    MinWidth,
    MaxWidth,
    MinHeight,
    MaxHeight,
    Margin,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    Padding,
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    Border,
    BorderWidth,
    BorderColor,
    BorderStyle,
    BoxSizing,

    // Typography
    FontSize,
    FontFamily,
    FontWeight,
    FontStyle,
    LineHeight,
    TextAlign,
    TextDecoration,

    // Positioning & Layout
    Position,
    Top,
    Right,
    Bottom,
    Left,
    ZIndex,
    Overflow,
    Opacity,
    Visibility,

    // Extensible custom / vendor properties (e.g. `--theme-color`, `-webkit-...`)
    Custom(String),
}

impl PropertyName {
    /// Creates a new `PropertyName`, normalized to lowercase.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let raw = name.into();
        let trimmed = raw.trim();
        let lower = trimmed.to_ascii_lowercase();

        match lower.as_str() {
            "color" => Self::Color,
            "background-color" | "background" => Self::BackgroundColor,
            "display" => Self::Display,
            "width" => Self::Width,
            "height" => Self::Height,
            "min-width" => Self::MinWidth,
            "max-width" => Self::MaxWidth,
            "min-height" => Self::MinHeight,
            "max-height" => Self::MaxHeight,
            "margin" => Self::Margin,
            "margin-top" => Self::MarginTop,
            "margin-right" => Self::MarginRight,
            "margin-bottom" => Self::MarginBottom,
            "margin-left" => Self::MarginLeft,
            "padding" => Self::Padding,
            "padding-top" => Self::PaddingTop,
            "padding-right" => Self::PaddingRight,
            "padding-bottom" => Self::PaddingBottom,
            "padding-left" => Self::PaddingLeft,
            "border" => Self::Border,
            "border-width" => Self::BorderWidth,
            "border-color" => Self::BorderColor,
            "border-style" => Self::BorderStyle,
            "box-sizing" => Self::BoxSizing,
            "font-size" => Self::FontSize,
            "font-family" => Self::FontFamily,
            "font-weight" => Self::FontWeight,
            "font-style" => Self::FontStyle,
            "line-height" => Self::LineHeight,
            "text-align" => Self::TextAlign,
            "text-decoration" => Self::TextDecoration,
            "position" => Self::Position,
            "top" => Self::Top,
            "right" => Self::Right,
            "bottom" => Self::Bottom,
            "left" => Self::Left,
            "z-index" => Self::ZIndex,
            "overflow" => Self::Overflow,
            "opacity" => Self::Opacity,
            "visibility" => Self::Visibility,
            _ => Self::Custom(lower),
        }
    }

    /// Accesses the name slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Color => "color",
            Self::BackgroundColor => "background-color",
            Self::Display => "display",
            Self::Width => "width",
            Self::Height => "height",
            Self::MinWidth => "min-width",
            Self::MaxWidth => "max-width",
            Self::MinHeight => "min-height",
            Self::MaxHeight => "max-height",
            Self::Margin => "margin",
            Self::MarginTop => "margin-top",
            Self::MarginRight => "margin-right",
            Self::MarginBottom => "margin-bottom",
            Self::MarginLeft => "margin-left",
            Self::Padding => "padding",
            Self::PaddingTop => "padding-top",
            Self::PaddingRight => "padding-right",
            Self::PaddingBottom => "padding-bottom",
            Self::PaddingLeft => "padding-left",
            Self::Border => "border",
            Self::BorderWidth => "border-width",
            Self::BorderColor => "border-color",
            Self::BorderStyle => "border-style",
            Self::BoxSizing => "box-sizing",
            Self::FontSize => "font-size",
            Self::FontFamily => "font-family",
            Self::FontWeight => "font-weight",
            Self::FontStyle => "font-style",
            Self::LineHeight => "line-height",
            Self::TextAlign => "text-align",
            Self::TextDecoration => "text-decoration",
            Self::Position => "position",
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::ZIndex => "z-index",
            Self::Overflow => "overflow",
            Self::Opacity => "opacity",
            Self::Visibility => "visibility",
            Self::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for PropertyName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Strongly typed CSS keywords mapped to W3C standards (C-46, C-50).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CssKeyword {
    // Global
    Inherit,
    Initial,
    Unset,
    Revert,

    // Display
    Block,
    Inline,
    None,
    Flex,

    // Typography
    Normal,
    Italic,
    Oblique,
    Bold,
    Bolder,
    Lighter,

    // Box & Layout
    ContentBox,
    BorderBox,
    Auto,

    // Alignment
    Left,
    Right,
    Center,
    Justify,

    // Fallback for custom/unrecognized keywords
    Custom(String),
}

impl CssKeyword {
    /// Parses a raw CSS keyword string.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        let lower = raw.trim().to_ascii_lowercase();
        match lower.as_str() {
            "inherit" => Self::Inherit,
            "initial" => Self::Initial,
            "unset" => Self::Unset,
            "revert" => Self::Revert,
            "block" => Self::Block,
            "inline" => Self::Inline,
            "none" => Self::None,
            "flex" => Self::Flex,
            "normal" => Self::Normal,
            "italic" => Self::Italic,
            "oblique" => Self::Oblique,
            "bold" => Self::Bold,
            "bolder" => Self::Bolder,
            "lighter" => Self::Lighter,
            "content-box" => Self::ContentBox,
            "border-box" => Self::BorderBox,
            "auto" => Self::Auto,
            "left" => Self::Left,
            "right" => Self::Right,
            "center" => Self::Center,
            "justify" => Self::Justify,
            _ => Self::Custom(lower),
        }
    }

    /// Accesses the keyword slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Inherit => "inherit",
            Self::Initial => "initial",
            Self::Unset => "unset",
            Self::Revert => "revert",
            Self::Block => "block",
            Self::Inline => "inline",
            Self::None => "none",
            Self::Flex => "flex",
            Self::Normal => "normal",
            Self::Italic => "italic",
            Self::Oblique => "oblique",
            Self::Bold => "bold",
            Self::Bolder => "bolder",
            Self::Lighter => "lighter",
            Self::ContentBox => "content-box",
            Self::BorderBox => "border-box",
            Self::Auto => "auto",
            Self::Left => "left",
            Self::Right => "right",
            Self::Center => "center",
            Self::Justify => "justify",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// Re-exported standard CSS display modes from engine domain (C-49).
pub use engine::DisplayType;

/// Strongly typed CSS property value (ADR-0010, C-46, C-50).
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// Color value.
    Color(Color),
    /// Length in pixels.
    Length(Px),
    /// Strongly typed keyword value (e.g. `block`, `inline`, `bold`).
    Keyword(CssKeyword),
    /// Strongly typed display mode.
    Display(DisplayType),
    /// Unparsed raw string fallback.
    Raw(String),
}

/// Helper deriving default `DisplayType` for a given DOM tag (C-13, C-49).
#[must_use]
pub fn display_from_tag(tag: &dom::TagName) -> DisplayType {
    tag.default_display()
}
