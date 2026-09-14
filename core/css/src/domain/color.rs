//! [`CssColor`] — a colour in the CSS value space.
//!
//! A thin newtype over [`graphics::Color`] so the CSS colour vocabulary
//! (`color`, `background-color`, named colours, `#rgb`, `rgb()` — B2) never
//! forces `graphics` into a boundary aggregate's public API. The wrapped
//! representation *is* `graphics::Color` (straight RGBA8), so painting is a
//! zero-cost [`CssColor::to_graphics`] at the pipeline's end.

use core::fmt;

use graphics::Color;

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
