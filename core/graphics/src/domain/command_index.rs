//! Where in a display list a failure happened.
//!
//! `ADR-0011:93-95` requires every port's typed error to carry source-location
//! metadata. In a display list the analogue of a line number is the index of the
//! offending command — which is why [`crate::DisplayListBuilder`] reports it
//! rather than just saying "a command was bad".

use core::fmt;

/// The position of a command within a display list, counting from zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandIndex(u32);

impl CommandIndex {
    /// The first command.
    pub const FIRST: Self = Self(0);

    #[must_use]
    pub const fn new(position: u32) -> Self {
        Self(position)
    }

    /// The index of the command at `position` in a builder's buffer.
    ///
    /// Saturates rather than failing: a display list long enough to overflow a
    /// `u32` has already exhausted memory, and losing precision in a diagnostic
    /// is better than losing the diagnostic.
    #[must_use]
    pub fn from_position(position: usize) -> Self {
        Self(u32::try_from(position).unwrap_or(u32::MAX))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for CommandIndex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}
