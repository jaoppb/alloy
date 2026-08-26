/// Strongly typed unforgeable index handle to a node in the generational DOM arena (ADR-0013, C-27).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId {
    index: u32,
    generation: u32,
}

impl NodeId {
    /// Creates a new `NodeId` from an unsigned 32-bit index with generation 0.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self {
            index,
            generation: 0,
        }
    }

    /// Creates a new `NodeId` with an explicit generation counter.
    #[must_use]
    pub const fn with_generation(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Returns the raw u32 index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Returns the generational counter.
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }

    /// Returns the index as a `usize` for vector lookup.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.index as usize
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.generation == 0 {
            write!(f, "#{}", self.index)
        } else {
            write!(f, "#{}:v{}", self.index, self.generation)
        }
    }
}
