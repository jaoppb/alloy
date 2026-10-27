//! Which layout of a document a [`crate::RegionId`] was minted for.

/// Bumps on every layout and every document swap, so an id that outlived either
/// is recognised as stale instead of resolving to whatever now sits at its index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LayoutEpoch(u64);

impl LayoutEpoch {
    /// The epoch before the first layout.
    pub const INITIAL: Self = Self(0);

    /// The epoch of the next layout.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::LayoutEpoch;

    #[test]
    fn each_layout_starts_a_distinct_epoch() {
        let first = LayoutEpoch::INITIAL.next();

        assert_ne!(LayoutEpoch::INITIAL, first);
        assert_ne!(first, first.next());
    }
}
