//! [`CssColor`] — a colour in the CSS value space.
//!
//! A thin newtype over [`graphics::Color`] so the CSS colour vocabulary
//! (`color`, `background-color`, named colours, `#rgb`, `rgb()` — B2) never
//! forces `graphics` into a boundary aggregate's public API. The wrapped
//! representation *is* `graphics::Color` (straight RGBA8), so painting is a
//! zero-cost [`CssColor::to_graphics`] at the pipeline's end.

use core::fmt;
use core::str::FromStr;

use graphics::Color;
use thiserror::Error;

/// How many hex digits a `#rrggbb` colour carries.
const LONG_HEX_DIGITS: usize = 6;
/// How many a `#rgb` colour carries.
const SHORT_HEX_DIGITS: usize = 3;

/// Why a colour literal was not a [`CssColor`].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ParseColorError {
    /// Neither a recognised keyword nor a `#rgb` / `#rrggbb` literal.
    #[error("unrecognised colour literal `{0}`")]
    Unrecognised(String),
    /// A `#` literal whose digits are not valid hexadecimal of length 3 or 6.
    #[error("invalid hex colour `{0}`")]
    InvalidHex(String),
}

/// An sRGB colour with straight alpha, as CSS computes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CssColor(Color);

impl CssColor {
    /// The CSS `initial` value of `color`: opaque black.
    pub const BLACK: Self = Self(Color::BLACK);
    /// The CSS `initial` value of `background-color`: fully transparent.
    pub const TRANSPARENT: Self = Self(Color::TRANSPARENT);

    /// From the four straight-alpha channels.
    #[must_use]
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self(Color::rgba(red, green, blue, alpha))
    }

    /// Opaque, from the three colour channels.
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self(Color::rgb(red, green, blue))
    }

    /// Adopts a `graphics` colour unchanged.
    #[must_use]
    pub const fn from_graphics(color: Color) -> Self {
        Self(color)
    }

    /// The colour keywords this cut recognises (CSS Color L4 §6.1 basic set);
    /// `name` must already be lowercase. The full name table is a later cut.
    #[must_use]
    pub fn from_keyword(name: &str) -> Option<Self> {
        match name {
            "transparent" => Some(Self::TRANSPARENT),
            "black" => Some(Self::rgb(0x00, 0x00, 0x00)),
            "silver" => Some(Self::rgb(0xC0, 0xC0, 0xC0)),
            "gray" | "grey" => Some(Self::rgb(0x80, 0x80, 0x80)),
            "white" => Some(Self::rgb(0xFF, 0xFF, 0xFF)),
            "maroon" => Some(Self::rgb(0x80, 0x00, 0x00)),
            "red" => Some(Self::rgb(0xFF, 0x00, 0x00)),
            "purple" => Some(Self::rgb(0x80, 0x00, 0x80)),
            "green" => Some(Self::rgb(0x00, 0x80, 0x00)),
            "lime" => Some(Self::rgb(0x00, 0xFF, 0x00)),
            "olive" => Some(Self::rgb(0x80, 0x80, 0x00)),
            "yellow" => Some(Self::rgb(0xFF, 0xFF, 0x00)),
            "navy" => Some(Self::rgb(0x00, 0x00, 0x80)),
            "blue" => Some(Self::rgb(0x00, 0x00, 0xFF)),
            "teal" => Some(Self::rgb(0x00, 0x80, 0x80)),
            "aqua" | "cyan" => Some(Self::rgb(0x00, 0xFF, 0xFF)),
            "fuchsia" | "magenta" => Some(Self::rgb(0xFF, 0x00, 0xFF)),
            "orange" => Some(Self::rgb(0xFF, 0xA5, 0x00)),
            _ => None,
        }
    }

    /// The digits of a `#rrggbb` / `#rgb` literal, without the `#`.
    #[must_use]
    pub fn from_hex_digits(digits: &str) -> Option<Self> {
        let expanded = Self::expand_hex(digits)?;
        let mut channels = expanded.as_bytes().chunks_exact(2).map(Self::pair_value);
        let red = channels.next()??;
        let green = channels.next()??;
        let blue = channels.next()??;
        Some(Self::rgb(red, green, blue))
    }

    /// `#rgb` is `#rrggbb` with each digit doubled (CSS Color L4 §6.1).
    fn expand_hex(digits: &str) -> Option<String> {
        if digits.len() == LONG_HEX_DIGITS {
            return Some(digits.to_owned());
        }
        if digits.len() != SHORT_HEX_DIGITS {
            return None;
        }
        Some(digits.chars().flat_map(|digit| [digit, digit]).collect())
    }

    /// One `rr` / `gg` / `bb` pair as a channel value.
    fn pair_value(pair: &[u8]) -> Option<u8> {
        let text = core::str::from_utf8(pair).ok()?;
        u8::from_str_radix(text, 16).ok()
    }

    /// The colour as `graphics` wants it for painting.
    #[must_use]
    pub const fn to_graphics(self) -> Color {
        self.0
    }
}

impl fmt::Display for CssColor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for CssColor {
    type Err = ParseColorError;

    /// A keyword or `#rgb` / `#rrggbb` literal, case-insensitive, trimmed.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let lower = text.trim().to_ascii_lowercase();
        let Some(digits) = lower.strip_prefix('#') else {
            return Self::from_keyword(&lower).ok_or(ParseColorError::Unrecognised(lower));
        };
        Self::from_hex_digits(digits).ok_or(ParseColorError::InvalidHex(lower))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_hex_and_case_parse() {
        assert_eq!("Red".parse(), Ok(CssColor::rgb(255, 0, 0)));
        assert_eq!(" #0F0 ".parse(), Ok(CssColor::rgb(0, 255, 0)));
        assert_eq!("#0000ff".parse(), Ok(CssColor::rgb(0, 0, 255)));
        assert_eq!("transparent".parse(), Ok(CssColor::TRANSPARENT));
    }

    #[test]
    fn rejects_unknown_keyword_and_bad_hex() {
        assert_eq!(
            "chartreuse".parse::<CssColor>(),
            Err(ParseColorError::Unrecognised("chartreuse".to_owned()))
        );
        assert_eq!(
            "#12".parse::<CssColor>(),
            Err(ParseColorError::InvalidHex("#12".to_owned()))
        );
        assert_eq!(
            "#gggggg".parse::<CssColor>(),
            Err(ParseColorError::InvalidHex("#gggggg".to_owned()))
        );
    }
}
