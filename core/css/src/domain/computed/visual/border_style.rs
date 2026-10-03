//! Border style enum (`border-style`, `border-top-style`, etc.).

use core::fmt;

/// The line style of a box border (CSS Backgrounds & Borders L3 §4.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BorderStyle {
    /// No border — the `initial` value.
    #[default]
    None,
    /// Same as `none` in the cascade, distinct in collapsing borders.
    Hidden,
    /// A series of round dots.
    Dotted,
    /// A series of square-ended dashes.
    Dashed,
    /// A single solid line segment.
    Solid,
    /// Two parallel solid lines.
    Double,
    /// Carved into the canvas.
    Groove,
    /// Extruded from the canvas.
    Ridge,
    /// Embedded into the canvas.
    Inset,
    /// Raised above the canvas.
    Outset,
}

impl BorderStyle {
    /// Keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Hidden => "hidden",
            Self::Dotted => "dotted",
            Self::Dashed => "dashed",
            Self::Solid => "solid",
            Self::Double => "double",
            Self::Groove => "groove",
            Self::Ridge => "ridge",
            Self::Inset => "inset",
            Self::Outset => "outset",
        }
    }

    /// Parses a border style keyword.
    #[must_use]
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        match keyword {
            "none" => Some(Self::None),
            "hidden" => Some(Self::Hidden),
            "dotted" => Some(Self::Dotted),
            "dashed" => Some(Self::Dashed),
            "solid" => Some(Self::Solid),
            "double" => Some(Self::Double),
            "groove" => Some(Self::Groove),
            "ridge" => Some(Self::Ridge),
            "inset" => Some(Self::Inset),
            "outset" => Some(Self::Outset),
            _ => None,
        }
    }

    /// Whether this style renders no visible line.
    #[must_use]
    pub const fn is_none_or_hidden(self) -> bool {
        matches!(self, Self::None | Self::Hidden)
    }
}

impl fmt::Display for BorderStyle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
