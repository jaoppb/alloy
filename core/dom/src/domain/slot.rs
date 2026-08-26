/// A generational slot in the DOM arena containing either an active node or a linked free pointer (ADR-0013, C-27).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Slot<T> {
    /// Slot is actively occupied by a node.
    Occupied { data: T, generation: u32 },
    /// Slot is vacant and points to the next free slot index in the free list.
    Vacant {
        next_free: Option<u32>,
        generation: u32,
    },
}

impl<T> Slot<T> {
    /// Returns the generational counter of this slot.
    #[must_use]
    pub const fn generation(&self) -> u32 {
        match self {
            Self::Occupied { generation, .. } | Self::Vacant { generation, .. } => *generation,
        }
    }

    /// Checks if this slot is occupied.
    #[must_use]
    pub const fn is_occupied(&self) -> bool {
        matches!(self, Self::Occupied { .. })
    }

    /// Accesses the data if occupied.
    #[must_use]
    pub const fn as_occupied(&self) -> Option<&T> {
        match self {
            Self::Occupied { data, .. } => Some(data),
            Self::Vacant { .. } => None,
        }
    }

    /// Accesses the data mutably if occupied.
    pub fn as_occupied_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Occupied { data, .. } => Some(data),
            Self::Vacant { .. } => None,
        }
    }
}
