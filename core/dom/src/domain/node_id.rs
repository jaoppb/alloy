/// Strongly typed unforgeable index handle to a node in the DOM arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u32);

impl NodeId {
    /// Creates a new `NodeId` from an unsigned 32-bit index.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the raw u32 index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }

    /// Returns the index as a `usize` for vector lookup.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}
