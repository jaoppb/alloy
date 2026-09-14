//! [`Length`] — a CSS length as an author wrote it, before resolution.
//!
//! A tagged sum over the unit the author chose (`px` / `em` / `rem` / `%` /
//! `pt`). It is the value object; the `f32` payload is the raw magnitude, the
//! same way [`graphics::Px`] wraps an unvalidated `f32`
//! (`core/graphics/src/domain/unit.rs:27`). Resolution to a computed
//! [`graphics::Au`] happens exactly once, in the layout engine, through
//! [`Length::resolve_to_au`] — which is the single place a non-finite magnitude
//! is caught (it delegates to [`graphics::Au::from_px`]).

use core::fmt;

use graphics::{Au, Px};

/// CSS pixels per CSS point: `96 / 72`, expressed as a fraction to keep the
/// conversion exact rather than a rounded constant.
const PX_PER_POINT_NUMERATOR: f32 = 96.0;
const PX_PER_POINT_DENOMINATOR: f32 = 72.0;
/// Percent is a fraction of a reference length.
const PERCENT_DIVISOR: f32 = 100.0;

/// A length in the unit the stylesheet used.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Length {
    /// An absolute length in CSS pixels.
    Pixels(f32),
    /// A multiple of the element's own computed `font-size`.
    Em(f32),
    /// A multiple of the root element's computed `font-size`. B0 has no root
    /// font-size channel yet, so this resolves against the element's own
    /// `font-size` — B2 threads the real root value.
    Rem(f32),
    /// A percentage of a reference length (the containing block's width, here).
    Percent(f32),
    /// An absolute length in CSS points (`1pt = 96/72 px`).
    Points(f32),
}

impl Length {
    /// Zero length, unit-independent.
    pub const ZERO: Self = Self::Pixels(0.0);

    /// An absolute length in CSS pixels.
    #[must_use]
    pub const fn pixels(value: f32) -> Self {
        Self::Pixels(value)
    }

    /// The raw magnitude, without its unit.
    #[must_use]
    pub const fn magnitude(self) -> f32 {
        match self {
            Self::Pixels(value)
            | Self::Em(value)
            | Self::Rem(value)
            | Self::Percent(value)
            | Self::Points(value) => value,
        }
    }

    /// Resolves to a computed [`Au`], or `None` when the magnitude is
    /// non-finite (`NaN` / `±inf` has no correct reading — same rule as
    /// [`graphics::Au::from_px`]).
    ///
    /// `font_size` resolves `em` / `rem`; `container` resolves `%`. Both are
    /// already-computed lengths, so the arithmetic is a single `f32` scale
    /// followed by the one author-input crossing.
    #[must_use]
    pub fn resolve_to_au(self, font_size: Au, container: Au) -> Option<Au> {
        let pixels = match self {
            Self::Pixels(value) => value,
            Self::Em(factor) | Self::Rem(factor) => factor * font_size.to_px().get(),
            Self::Percent(percent) => percent / PERCENT_DIVISOR * container.to_px().get(),
            Self::Points(points) => points * PX_PER_POINT_NUMERATOR / PX_PER_POINT_DENOMINATOR,
        };
        Au::from_px(Px::new(pixels))
    }
}

impl Default for Length {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for Length {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pixels(value) => write!(formatter, "{value}px"),
            Self::Em(value) => write!(formatter, "{value}em"),
            Self::Rem(value) => write!(formatter, "{value}rem"),
            Self::Percent(value) => write!(formatter, "{value}%"),
            Self::Points(value) => write!(formatter, "{value}pt"),
        }
    }
}
