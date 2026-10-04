//! [`Display`] — the computed value of the `display` property.
//!
//! B0 recognises the outer display types the placeholder [`crate::BlockLayout`]
//! needs: `none` (no box), `block`, `inline`, and `flex` (declared so the
//! aggregate is born whole — B4 gives it a formatting context). The UA sheet's
//! form controls and list items add `inline-block` (an atomic inline-level
//! block container, CSS Display L3 §2.4) and `list-item` (a block box that
//! also generates a `::marker`, CSS Display L3 §2.5 — the marker is not
//! painted yet, see `core/css/tests/data/MANIFEST.md`).

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
    /// An inline-level block container laid out as one atomic box on a line
    /// (CSS 2.1 §9.2.4) — the UA display of `<input>` / `<button>`.
    InlineBlock,
    /// A block-level box that also generates a marker (CSS 2.1 §12.5). Lays
    /// out exactly like [`Display::Block`]; the marker itself is a declared
    /// cut of this engine.
    ListItem,
}

impl Display {
    /// Whether this element generates no box at all.
    #[must_use]
    pub const fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    /// Whether this box takes part in its parent's inline formatting context
    /// (CSS 2.1 §9.2.2) — an `inline` box or an atomic `inline-block`.
    #[must_use]
    pub const fn is_inline_level(self) -> bool {
        matches!(self, Self::Inline | Self::InlineBlock)
    }

    /// Whether this box sits on a line as one unbreakable unit whose inside is
    /// its own block formatting context (CSS 2.1 §9.2.4, "atomic inline-level").
    #[must_use]
    pub const fn is_atomic_inline(self) -> bool {
        matches!(self, Self::InlineBlock)
    }

    /// The keyword as it appears in a stylesheet.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Block => "block",
            Self::Inline => "inline",
            Self::Flex => "flex",
            Self::InlineBlock => "inline-block",
            Self::ListItem => "list-item",
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
            "inline-block" => Ok(Self::InlineBlock),
            "list-item" => Ok(Self::ListItem),
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
            Display::InlineBlock,
            Display::ListItem,
        ] {
            assert_eq!(display.keyword().parse(), Ok(display));
        }
        assert_eq!(
            "grid".parse::<Display>(),
            Err(ParseDisplayError("grid".to_owned()))
        );
        assert_eq!(
            "inline-flex".parse::<Display>(),
            Err(ParseDisplayError("inline-flex".to_owned()))
        );
    }
}
