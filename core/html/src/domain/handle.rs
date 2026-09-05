//! Opaque node handle representing tree nodes in the [`crate::application::ports::TreeSink`] port.

use core::fmt;

/// An opaque identifier referencing a node constructed within a tree sink.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeHandle(u32);

impl NodeHandle {
    /// The canonical root document handle.
    pub const ROOT: Self = Self(0);

    /// Creates a handle from a raw identifier.
    #[must_use]
    pub const fn new(identifier: u32) -> Self {
        Self(identifier)
    }

    /// Returns the root handle.
    #[must_use]
    pub const fn root() -> Self {
        Self::ROOT
    }

    /// Access the underlying raw index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }

    /// Generates the next sequential handle.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for NodeHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_handle_is_zero() {
        assert_eq!(NodeHandle::root().index(), 0);
    }

    #[test]
    fn sequential_handles_advance_properly() {
        let first = NodeHandle::new(10);
        let second = first.next();
        assert_eq!(second.index(), 11);
    }

    #[test]
    fn display_formats_hash_prefix() {
        assert_eq!(NodeHandle::new(5).to_string(), "#5");
    }
}
