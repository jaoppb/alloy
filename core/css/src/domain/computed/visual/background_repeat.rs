//! Background repeat types (`background-repeat`).

use core::fmt;

/// How a background image repeats across its canvas (CSS Backgrounds & Borders L3 §3.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BackgroundRepeat {
    /// Repeats horizontally and vertically — the CSS `initial` value.
    #[default]
    Repeat,
    /// Repeats horizontally only.
    RepeatX,
    /// Repeats vertically only.
    RepeatY,
    /// Rendered once, not repeated.
    NoRepeat,
    /// Repeated to fill the area without clipping, distributed with whitespace.
    Space,
    /// Repeated and scaled to fit an integer number of times.
    Round,
}

impl BackgroundRepeat {
    /// Keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Repeat => "repeat",
            Self::RepeatX => "repeat-x",
            Self::RepeatY => "repeat-y",
            Self::NoRepeat => "no-repeat",
            Self::Space => "space",
            Self::Round => "round",
        }
    }

    /// Parses a repeat keyword.
    #[must_use]
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        match keyword {
            "repeat" => Some(Self::Repeat),
            "repeat-x" => Some(Self::RepeatX),
            "repeat-y" => Some(Self::RepeatY),
            "no-repeat" => Some(Self::NoRepeat),
            "space" => Some(Self::Space),
            "round" => Some(Self::Round),
            _ => None,
        }
    }

    /// Whether this repeat style repeats along the horizontal axis.
    #[must_use]
    pub const fn repeats_x(self) -> bool {
        matches!(
            self,
            Self::Repeat | Self::RepeatX | Self::Round | Self::Space
        )
    }

    /// Whether this repeat style repeats along the vertical axis.
    #[must_use]
    pub const fn repeats_y(self) -> bool {
        matches!(
            self,
            Self::Repeat | Self::RepeatY | Self::Round | Self::Space
        )
    }
}

impl fmt::Display for BackgroundRepeat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}
