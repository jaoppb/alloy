//! Opacity type (`opacity`).

use core::fmt;

/// An opacity value on the unit interval `[0.0, 1.0]` (CSS Color L4 §4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opacity(f32);

impl Opacity {
    /// Fully opaque — the CSS `initial` value.
    pub const ONE: Self = Self(1.0);
    /// Fully transparent.
    pub const ZERO: Self = Self(0.0);

    /// Constructs an opacity from a float, clamping to `[0.0, 1.0]`.
    ///
    /// Returns `None` for non-finite values (NaN, infinity).
    #[must_use]
    pub const fn new(value: f32) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        Some(Self(clamp_unit(value)))
    }

    /// Constructs an opacity from a float, clamping finite values to `[0.0, 1.0]`.
    ///
    /// Non-finite values fall back to [`Self::ONE`].
    #[must_use]
    pub const fn clamped(value: f32) -> Self {
        if !value.is_finite() {
            return Self::ONE;
        }
        Self(clamp_unit(value))
    }

    /// The raw float value on `[0.0, 1.0]`.
    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }

    /// Whether this opacity is fully opaque (1.0).
    #[must_use]
    pub fn is_opaque(self) -> bool {
        self.0 >= 1.0
    }

    /// Whether this opacity is fully transparent (0.0).
    #[must_use]
    pub fn is_transparent(self) -> bool {
        self.0 <= 0.0
    }
}

const fn clamp_unit(value: f32) -> f32 {
    if value < 0.0 {
        return 0.0;
    }
    if value > 1.0 {
        return 1.0;
    }
    value
}

impl Default for Opacity {
    fn default() -> Self {
        Self::ONE
    }
}

impl fmt::Display for Opacity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
