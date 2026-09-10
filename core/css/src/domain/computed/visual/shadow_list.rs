//! Fixed-capacity box shadow list.

use super::shadow::BoxShadow;

/// A fixed-capacity list of box shadows, or `none`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoxShadowList {
    entries: [Option<BoxShadow>; Self::CAPACITY],
}

impl BoxShadowList {
    /// The maximum number of shadows stored in a single list.
    pub const CAPACITY: usize = 4;

    /// The CSS `initial` value: `none`.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            entries: [None; Self::CAPACITY],
        }
    }

    /// List containing a single shadow.
    #[must_use]
    pub const fn from_single(shadow: BoxShadow) -> Self {
        Self {
            entries: [Some(shadow), None, None, None],
        }
    }

    /// Constructs from an array of shadow options.
    #[must_use]
    pub const fn from_entries(entries: [Option<BoxShadow>; Self::CAPACITY]) -> Self {
        Self { entries }
    }

    /// Whether this list contains no shadows (`none`).
    #[must_use]
    pub const fn is_none(self) -> bool {
        matches!(self.entries, [None, None, None, None])
    }

    /// The stored shadow entries.
    #[must_use]
    pub const fn entries(self) -> [Option<BoxShadow>; Self::CAPACITY] {
        self.entries
    }
}
