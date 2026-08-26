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

    /// Parses a CSS color string (hex or named color).
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let s = raw.trim().to_ascii_lowercase();

        match s.as_str() {
            "transparent" => return Some(Self::TRANSPARENT),
            "black" => return Some(Self::BLACK),
            "white" => return Some(Self::WHITE),
            "red" => return Some(Self::RED),
            "green" => return Some(Self::GREEN),
            "blue" => return Some(Self::BLUE),
            "gray" | "grey" => return Some(Self::rgba(128, 128, 128, 255)),
            "yellow" => return Some(Self::rgba(255, 255, 0, 255)),
            _ => {}
        }

        if let Some(hex) = s.strip_prefix('#') {
            return Self::parse_hex(hex);
        }

        None
    }

    fn parse_hex(hex: &str) -> Option<Self> {
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::rgba(r, g, b, a))
            }
            _ => None,
        }
    }
}

/// Normalized CSS property name (e.g. `color`, `background-color`, `font-size`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PropertyName(String);

impl PropertyName {
    /// Creates a new `PropertyName`, normalized to lowercase.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into().trim().to_ascii_lowercase())
    }

    /// Accesses the name slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Strongly typed CSS property value.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// Color value.
    Color(Color),
    /// Length in pixels.
    Length(Px),
    /// Identifier / keyword value (e.g. `block`, `inline`, `bold`).
    Keyword(String),
    /// Unparsed raw string fallback.
    Raw(String),
}

/// CSS `display` property modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayType {
    /// Standard block formatting.
    #[default]
    Block,
    /// Inline flow formatting.
    Inline,
    /// Element is hidden and generates no layout box.
    None,
    /// Flexbox container formatting.
    Flex,
}
