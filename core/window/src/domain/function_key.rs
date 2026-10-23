//! [`FunctionKey`] — F1 through F12.

use core::fmt;

/// A function key, F1 through F12.
///
/// A newtype rather than twelve enum variants (and twelve matching `Display`
/// and adapter arms): the range is the whole invariant. F13 and above are
/// [`LogicalKey::Other`](crate::domain::logical_key::LogicalKey::Other).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FunctionKey(u8);

impl FunctionKey {
    const FIRST: u8 = 1;
    const LAST: u8 = 12;

    /// `None` outside `1..=12`.
    #[must_use]
    pub const fn new(number: u8) -> Option<Self> {
        if number < Self::FIRST || number > Self::LAST {
            return None;
        }
        Some(Self(number))
    }

    /// The key's number: `5` for F5.
    #[must_use]
    pub const fn number(self) -> u8 {
        self.0
    }
}

impl fmt::Display for FunctionKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "F{}", self.0)
    }
}
