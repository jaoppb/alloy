//! [`LineHeight`] — minimal height of line boxes (CSS 2.1 §10.8.1).

use core::fmt;
use graphics::{Au, Px};

use crate::domain::length::Length;

/// Typical browser default ratio for normal line height.
const NORMAL_LINE_HEIGHT_RATIO: f32 = 1.2;
/// Percentage divisor to convert percentage to fraction.
const PERCENT_DIVISOR: f32 = 100.0;

/// A unitless multiplier for `line-height`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHeightFactor(f32);

impl LineHeightFactor {
    #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self(factor)
    }

    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }
}

impl fmt::Display for LineHeightFactor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A percentage value for `line-height`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHeightPercentage(f32);

impl LineHeightPercentage {
    #[must_use]
    pub const fn new(percentage: f32) -> Self {
        Self(percentage)
    }

    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }
}

impl fmt::Display for LineHeightPercentage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}%", self.0)
    }
}

/// The computed value of `line-height`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum LineHeight {
    /// Layout-dependent default line height — CSS `initial`.
    #[default]
    Normal,
    /// Explicit length (e.g. `24px`).
    Length(Length),
    /// Unitless factor multiplied by the element's font size.
    Number(LineHeightFactor),
    /// Percentage of the element's font size.
    Percentage(LineHeightPercentage),
}

impl LineHeight {
    #[must_use]
    pub const fn is_normal(self) -> bool {
        matches!(self, Self::Normal)
    }

    /// Resolves this line height against the computed font size.
    #[must_use]
    pub fn resolve_to_au(self, font_size: Au) -> Option<Au> {
        match self {
            Self::Normal => {
                Au::from_px(Px::new(font_size.to_px().get() * NORMAL_LINE_HEIGHT_RATIO))
            }
            Self::Length(length) => length.resolve_to_au(font_size, font_size),
            Self::Number(factor) => Au::from_px(Px::new(font_size.to_px().get() * factor.value())),
            Self::Percentage(percent) => Au::from_px(Px::new(
                font_size.to_px().get() * (percent.value() / PERCENT_DIVISOR),
            )),
        }
    }
}

impl fmt::Display for LineHeight {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => formatter.write_str("normal"),
            Self::Length(length) => length.fmt(formatter),
            Self::Number(factor) => factor.fmt(formatter),
            Self::Percentage(percentage) => percentage.fmt(formatter),
        }
    }
}
