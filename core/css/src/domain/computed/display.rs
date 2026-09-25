//! [`Display`] — the computed value of the `display` property.
//!
//! B0 recognises the outer display types the placeholder [`crate::BlockLayout`]
//! needs: `none` (no box), `block`, `inline`, and `flex` (declared so the
//! aggregate is born whole — B4 gives it a formatting context).

use core::fmt;
use core::str::FromStr;

use thiserror::Error;

/// A `display` keyword this cut does not carry.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("unsupported display keyword `{0}`")]
pub struct ParseDisplayError(String);

/// How an element generates boxes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Display {
    /// The element and its subtree generate no box.
    None,
    /// A block-level box in normal flow — the `initial` value here, matching the
    /// UA rule for the elements B0 lays out.
    #[default]
    Block,
    /// An inline-level box in normal flow.
    Inline,
    /// A block-level flex container. Declared for the frozen contract; B4
    /// implements the layout.
    Flex,
}

impl Display {
    /// Whether this element generates no box at all.
    #[must_use]
    pub const fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Block => "block",
            Self::Inline => "inline",
            Self::Flex => "flex",
        }
    }
}

impl fmt::Display for Display {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.keyword())
    }
}

impl FromStr for Display {
    type Err = ParseDisplayError;

    /// The exact lowercase stylesheet keyword.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "none" => Ok(Self::None),
            "block" => Ok(Self::Block),
            "inline" => Ok(Self::Inline),
            "flex" => Ok(Self::Flex),
            other => Err(ParseDisplayError(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_round_trips_and_unknown_is_typed() {
        for display in [
            Display::None,
            Display::Block,
            Display::Inline,
            Display::Flex,
        ] {
            assert_eq!(display.keyword().parse(), Ok(display));
        }
        assert_eq!(
            "grid".parse::<Display>(),
            Err(ParseDisplayError("grid".to_owned()))
        );
    }
}
