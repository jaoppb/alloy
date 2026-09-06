//! Track size values and breadths for grid tracks.

use core::fmt;

use super::fr::GridFr;
use crate::domain::length::Length;

/// Minimum track breadth for `minmax(min, max)` (CSS Grid L1 §7.2.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MinTrackBreadth {
    Length(Length),
    MinContent,
    MaxContent,
    Auto,
}

impl MinTrackBreadth {
    #[must_use]
    pub const fn pixels(px: f32) -> Self {
        Self::Length(Length::Pixels(px))
    }
}

impl fmt::Display for MinTrackBreadth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length(len) => write!(formatter, "{len}"),
            Self::MinContent => formatter.write_str("min-content"),
            Self::MaxContent => formatter.write_str("max-content"),
            Self::Auto => formatter.write_str("auto"),
        }
    }
}

/// Maximum track breadth for `minmax(min, max)` (CSS Grid L1 §7.2.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaxTrackBreadth {
    Length(Length),
    Flex(GridFr),
    MinContent,
    MaxContent,
    Auto,
}

impl MaxTrackBreadth {
    #[must_use]
    pub const fn pixels(px: f32) -> Self {
        Self::Length(Length::Pixels(px))
    }

    #[must_use]
    pub const fn fr(fr: GridFr) -> Self {
        Self::Flex(fr)
    }
}

impl fmt::Display for MaxTrackBreadth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length(len) => write!(formatter, "{len}"),
            Self::Flex(fr) => write!(formatter, "{fr}"),
            Self::MinContent => formatter.write_str("min-content"),
            Self::MaxContent => formatter.write_str("max-content"),
            Self::Auto => formatter.write_str("auto"),
        }
    }
}

/// A track sizing function (CSS Grid L1 §7.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrackSize {
    Length(Length),
    Flex(GridFr),
    MinContent,
    MaxContent,
    Auto,
    MinMax(MinTrackBreadth, MaxTrackBreadth),
}

impl TrackSize {
    #[must_use]
    pub const fn pixels(px: f32) -> Self {
        Self::Length(Length::Pixels(px))
    }

    #[must_use]
    pub const fn percent(pct: f32) -> Self {
        Self::Length(Length::Percent(pct))
    }

    #[must_use]
    pub fn fr(value: f32) -> Option<Self> {
        GridFr::new(value).map(Self::Flex)
    }

    #[must_use]
    pub const fn minmax(min: MinTrackBreadth, max: MaxTrackBreadth) -> Self {
        Self::MinMax(min, max)
    }

    #[must_use]
    pub const fn is_flexible(&self) -> bool {
        matches!(
            self,
            Self::Flex(_) | Self::MinMax(_, MaxTrackBreadth::Flex(_))
        )
    }

    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }
}

impl fmt::Display for TrackSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length(length) => write!(formatter, "{length}"),
            Self::Flex(fr) => write!(formatter, "{fr}"),
            Self::MinContent => formatter.write_str("min-content"),
            Self::MaxContent => formatter.write_str("max-content"),
            Self::Auto => formatter.write_str("auto"),
            Self::MinMax(min, max) => write!(formatter, "minmax({min}, {max})"),
        }
    }
}
