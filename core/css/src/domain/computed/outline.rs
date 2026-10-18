//! [`Outline`] — the computed values of the outline properties (CSS UI 4 §3).

use core::fmt;
use core::str::FromStr;

use thiserror::Error;

use crate::domain::color::CssColor;
use crate::domain::length::Length;

/// An `outline-style` keyword this cut does not carry.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("unsupported outline-style keyword `{0}`")]
pub struct ParseOutlineStyleError(pub String);

/// The line style of an element's outline (CSS UI 4 §3.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OutlineStyle {
    /// No outline — the `initial` value.
    #[default]
    None,
    /// User-agent focus ring, rendered as solid in this cut.
    Auto,
    /// Same as `none` in painting, distinct in cascading.
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

impl OutlineStyle {
    /// Whether this outline style renders visible pixels.
    #[must_use]
    pub const fn is_drawn(self) -> bool {
        !matches!(self, Self::None | Self::Hidden)
    }

    /// The stylesheet keyword for this value.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Auto => "auto",
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
}

impl fmt::Display for OutlineStyle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

impl FromStr for OutlineStyle {
    type Err = ParseOutlineStyleError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "none" => Ok(Self::None),
            "auto" => Ok(Self::Auto),
            "hidden" => Ok(Self::Hidden),
            "dotted" => Ok(Self::Dotted),
            "dashed" => Ok(Self::Dashed),
            "solid" => Ok(Self::Solid),
            "double" => Ok(Self::Double),
            "groove" => Ok(Self::Groove),
            "ridge" => Ok(Self::Ridge),
            "inset" => Ok(Self::Inset),
            "outset" => Ok(Self::Outset),
            other => Err(ParseOutlineStyleError(other.to_owned())),
        }
    }
}

/// The four properties of an element's outline grouped into one aggregate.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Outline {
    color: CssColor,
    style: OutlineStyle,
    width: Length,
    offset: Length,
}

impl Outline {
    /// Initial outline width (CSS `medium` keyword default, 3px).
    pub const INITIAL_WIDTH_PX: f32 = 3.0;

    /// Initial outline values: black, none, 3px medium width, 0px offset.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            color: CssColor::BLACK,
            style: OutlineStyle::None,
            width: Length::Pixels(Self::INITIAL_WIDTH_PX),
            offset: Length::ZERO,
        }
    }

    /// Explicit constructor for all four components.
    #[must_use]
    pub const fn new(color: CssColor, style: OutlineStyle, width: Length, offset: Length) -> Self {
        Self {
            color,
            style,
            width,
            offset,
        }
    }

    #[must_use]
    pub const fn color(self) -> CssColor {
        self.color
    }

    #[must_use]
    pub const fn style(self) -> OutlineStyle {
        self.style
    }

    #[must_use]
    pub const fn width(self) -> Length {
        self.width
    }

    #[must_use]
    pub const fn offset(self) -> Length {
        self.offset
    }

    #[must_use]
    pub const fn with_color(self, color: CssColor) -> Self {
        Self { color, ..self }
    }

    #[must_use]
    pub const fn with_style(self, style: OutlineStyle) -> Self {
        Self { style, ..self }
    }

    #[must_use]
    pub const fn with_width(self, width: Length) -> Self {
        Self { width, ..self }
    }

    #[must_use]
    pub const fn with_offset(self, offset: Length) -> Self {
        Self { offset, ..self }
    }
}

impl Default for Outline {
    fn default() -> Self {
        Self::initial()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_style_round_trips() {
        for style in [
            OutlineStyle::None,
            OutlineStyle::Auto,
            OutlineStyle::Hidden,
            OutlineStyle::Dotted,
            OutlineStyle::Dashed,
            OutlineStyle::Solid,
            OutlineStyle::Double,
            OutlineStyle::Groove,
            OutlineStyle::Ridge,
            OutlineStyle::Inset,
            OutlineStyle::Outset,
        ] {
            assert_eq!(style.keyword().parse(), Ok(style));
        }
        assert_eq!(
            "invalid".parse::<OutlineStyle>(),
            Err(ParseOutlineStyleError("invalid".to_owned()))
        );
    }

    #[test]
    fn outline_style_is_drawn_semantics() {
        assert!(!OutlineStyle::None.is_drawn());
        assert!(!OutlineStyle::Hidden.is_drawn());
        assert!(OutlineStyle::Auto.is_drawn());
        assert!(OutlineStyle::Solid.is_drawn());
        assert!(OutlineStyle::Dotted.is_drawn());
    }

    #[test]
    fn outline_initial_values_and_builders() {
        let initial = Outline::initial();
        assert_eq!(initial.color(), CssColor::BLACK);
        assert_eq!(initial.style(), OutlineStyle::None);
        assert_eq!(initial.width(), Length::Pixels(3.0));
        assert_eq!(initial.offset(), Length::ZERO);

        let custom = initial
            .with_color(CssColor::rgb(255, 0, 0))
            .with_style(OutlineStyle::Solid)
            .with_width(Length::Pixels(5.0))
            .with_offset(Length::Pixels(2.0));

        assert_eq!(custom.color(), CssColor::rgb(255, 0, 0));
        assert_eq!(custom.style(), OutlineStyle::Solid);
        assert_eq!(custom.width(), Length::Pixels(5.0));
        assert_eq!(custom.offset(), Length::Pixels(2.0));
    }
}
