//! Colour and opacity.
//!
//! [`Color`] is straight (non-premultiplied) RGBA8 packed into a `u32`, because
//! that is the form an author supplies and the form a PNG stores.
//! [`Color::premultiplied`] produces the form the compositor wants, and is the
//! only place the multiplication happens.

use core::fmt;

use crate::domain::convert;

/// A straight-alpha RGBA colour, packed as `0xRRGGBBAA`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color(u32);

impl Color {
    /// Fully transparent — the identity for `src-over` composition.
    pub const TRANSPARENT: Self = Self(0x0000_0000);
    /// Opaque black.
    pub const BLACK: Self = Self(0x0000_00ff);
    /// Opaque white.
    pub const WHITE: Self = Self(0xffff_ffff);

    #[must_use]
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        let channels = [red, green, blue, alpha];
        Self(u32::from_be_bytes(channels))
    }

    /// Opaque, from the three colour channels.
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, u8::MAX)
    }

    #[must_use]
    pub const fn red(self) -> u8 {
        let [red, _, _, _] = self.0.to_be_bytes();
        red
    }

    #[must_use]
    pub const fn green(self) -> u8 {
        let [_, green, _, _] = self.0.to_be_bytes();
        green
    }

    #[must_use]
    pub const fn blue(self) -> u8 {
        let [_, _, blue, _] = self.0.to_be_bytes();
        blue
    }

    #[must_use]
    pub const fn alpha(self) -> u8 {
        let [_, _, _, alpha] = self.0.to_be_bytes();
        alpha
    }

    /// The packed representation, for a backend that writes whole words.
    #[must_use]
    pub const fn packed(self) -> u32 {
        self.0
    }

    /// The four channels in memory order, which is what a RGBA8 framebuffer
    /// row holds.
    #[must_use]
    pub const fn to_rgba8(self) -> [u8; 4] {
        self.0.to_be_bytes()
    }

    #[must_use]
    pub const fn is_opaque(self) -> bool {
        self.alpha() == u8::MAX
    }

    #[must_use]
    pub const fn is_transparent(self) -> bool {
        self.alpha() == 0
    }

    /// The same colour with each channel scaled by its own alpha, which is the
    /// form `src-over` composition operates on.
    #[must_use]
    pub fn premultiplied(self) -> Self {
        let alpha = self.alpha();
        Self::rgba(
            scale_by_alpha(self.red(), alpha),
            scale_by_alpha(self.green(), alpha),
            scale_by_alpha(self.blue(), alpha),
            alpha,
        )
    }

    /// The same colour at a different alpha.
    #[must_use]
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self::rgba(self.red(), self.green(), self.blue(), alpha)
    }

    /// The same colour with its alpha attenuated by `opacity`.
    #[must_use]
    pub fn faded(self, opacity: Opacity) -> Self {
        self.with_alpha(scale_by_alpha(self.alpha(), opacity.level()))
    }
}

/// Multiplies two 8-bit fractions, rounding to nearest.
///
/// `(a * b + 127 + (a * b + 127) / 255) / 256` is the standard rounding form of
/// `a * b / 255` that keeps `255 × 255` at exactly `255`. Widening to `u16`
/// through `From` makes every step total — the largest intermediate is `65_407` —
/// so the saturating operators below can never actually saturate; they are
/// there because the lint gate asks for a total operator, not because the
/// arithmetic is in doubt.
fn scale_by_alpha(channel: u8, alpha: u8) -> u8 {
    let product = u16::from(channel)
        .saturating_mul(u16::from(alpha))
        .saturating_add(127);
    let rounded = product.saturating_add(product / 255) / 256;
    u8::try_from(rounded).unwrap_or(u8::MAX)
}

impl fmt::Display for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "#{:08x}", self.0)
    }
}

/// A compositing opacity on the unit interval, stored as 8-bit precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Opacity(u8);

impl Opacity {
    /// Fully opaque.
    pub const OPAQUE: Self = Self(u8::MAX);
    /// Fully transparent.
    pub const TRANSPARENT: Self = Self(0);

    /// Builds an opacity from an author-supplied fraction.
    ///
    /// Returns `None` for a non-finite input, and **clamps** a finite one into
    /// `[0, 1]`. Same split as [`crate::Au::from_px`], for the same reason:
    /// `NaN` has no correct reading, but `1.5` plainly means "opaque" and
    /// refusing it would break the page (v0.3 report §2.3).
    #[must_use]
    pub fn from_unit_interval(value: f32) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        Some(Self(convert::unit_interval_to_u8(value)))
    }

    #[must_use]
    pub const fn level(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn is_opaque(self) -> bool {
        self.0 == u8::MAX
    }

    #[must_use]
    pub const fn is_transparent(self) -> bool {
        self.0 == 0
    }
}

impl Default for Opacity {
    fn default() -> Self {
        Self::OPAQUE
    }
}

impl fmt::Display for Opacity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/255", self.0)
    }
}
