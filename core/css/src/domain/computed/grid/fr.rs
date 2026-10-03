//! `<flex>` fraction for grid tracks (`fr` unit).

use core::fmt;

/// A `<flex>` fraction for grid tracks (`fr` unit), non-negative and finite.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct GridFr(f32);

impl GridFr {
    /// Initial unit fraction `1fr`.
    pub const ONE: Self = Self(1.0);

    /// Constructs a grid fraction, or `None` if negative or non-finite.
    #[must_use]
    pub fn new(value: f32) -> Option<Self> {
        if !value.is_finite() || value < 0.0 {
            return None;
        }
        Some(Self(value))
    }

    /// The raw magnitude of the fraction.
    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }

    /// Whether this fraction takes no free space.
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }
}

impl fmt::Display for GridFr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}fr", self.0)
    }
}
