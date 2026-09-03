//! Fixed-point length units, and the single author-input → fixed-point crossing.
//!
//! [`Au`] ("app unit") is 1/64 px — the 26.6 convention shared with font
//! metrics. Every *computed* geometry value in this crate is an `Au`, so box
//! arithmetic is integer arithmetic: no floating-point error accumulates and no
//! rounding mode varies between platforms. That is what lets a golden image
//! match byte for byte on Linux, macOS and Windows (`ADR-0016`, v0.3 report
//! §2.5).
//!
//! [`Px`] is the *input* type — a length as an author or a stylesheet wrote it.
//! It becomes an `Au` through exactly one function, [`Au::from_px`], which is
//! therefore the single place a non-finite length can be caught (v0.3 report
//! §2.3).

use core::fmt;

use crate::domain::convert;

/// How many [`Au`] make one CSS pixel.
pub const AU_PER_PX: i32 = 64;

/// A length as supplied by an author or a stylesheet, before validation.
///
/// Deliberately unvalidated: a `Px` may hold `NaN` or `±inf`. Validation happens
/// once, at [`Au::from_px`].
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Px(f32);

impl Px {
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl fmt::Display for Px {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}px", self.0)
    }
}

/// A computed length in 1/64 px.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Au(i32);

impl Au {
    /// Zero length.
    pub const ZERO: Self = Self(0);

    /// The furthest a coordinate may sit from the origin: 2^26 `Au`, which is
    /// `1_048_576` px.
    ///
    /// Chosen so that adding two extremes still fits an `i32` with room to
    /// spare, and so that a legitimately tall page — the v0.3 report's `10_000` px
    /// case — is nowhere near the clamp (v0.3 report §4).
    pub const MAX_EXTENT: Self = Self(67_108_864);

    /// The negative twin of [`Au::MAX_EXTENT`].
    pub const MIN_EXTENT: Self = Self(-67_108_864);

    /// Wraps a raw 1/64-px count.
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// The raw 1/64-px count.
    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Converts a whole number of pixels, or `None` on overflow.
    #[must_use]
    pub const fn from_whole_px(pixels: i32) -> Option<Self> {
        match pixels.checked_mul(AU_PER_PX) {
            Some(product) => Some(Self(product)),
            None => None,
        }
    }

    /// The one crossing from author input into computed geometry.
    ///
    /// Returns `None` for a non-finite input: there is no correct reading of
    /// `NaN` or `±inf`, and quietly substituting a number turns a layout defect
    /// into a wrong picture. A finite value outside the envelope is **clamped**
    /// to [`Au::MAX_EXTENT`] instead — a legitimate page has a giant box, and
    /// refusing one would break the page. The two rules are deliberately
    /// different (v0.3 report §2.3).
    #[must_use]
    pub fn from_px(px: Px) -> Option<Self> {
        let value = px.get();
        if !value.is_finite() {
            return None;
        }
        let scaled = value * convert::to_f32(AU_PER_PX);
        Some(Self(convert::round_and_clamp_to_i32(
            scaled,
            Self::MIN_EXTENT.0,
            Self::MAX_EXTENT.0,
        )))
    }

    /// The length back in pixels, for diagnostics and author-facing output.
    #[must_use]
    pub fn to_px(self) -> Px {
        Px(convert::to_f32(self.0) / convert::to_f32(AU_PER_PX))
    }

    #[must_use]
    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(sum) => Some(Self(sum)),
            None => None,
        }
    }

    #[must_use]
    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(difference) => Some(Self(difference)),
            None => None,
        }
    }

    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    #[must_use]
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    #[must_use]
    pub const fn smaller(self, other: Self) -> Self {
        if self.0 <= other.0 {
            return self;
        }
        other
    }

    #[must_use]
    pub const fn larger(self, other: Self) -> Self {
        if self.0 >= other.0 {
            return self;
        }
        other
    }

    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Au {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}au", self.0)
    }
}
