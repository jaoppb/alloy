//! The name of one link region.

use super::{ContextId, LayoutEpoch};

/// A link region of one layout of one context's document.
///
/// Opaque to `core/ui`, which only copies and compares it; `core/browser`
/// mints and resolves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionId {
    context: ContextId,
    epoch: LayoutEpoch,
    index: usize,
}

impl RegionId {
    /// The `index`-th link, in paint order, of `context`'s layout number `epoch`.
    #[must_use]
    pub const fn new(context: ContextId, epoch: LayoutEpoch, index: usize) -> Self {
        Self {
            context,
            epoch,
            index,
        }
    }

    /// The browsing context that minted the id.
    #[must_use]
    pub const fn context(self) -> ContextId {
        self.context
    }

    /// The layout the id was minted for.
    #[must_use]
    pub const fn epoch(self) -> LayoutEpoch {
        self.epoch
    }

    /// The link's position in paint order.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }
}

#[cfg(test)]
mod tests {
    use super::RegionId;
    use crate::domain::{ContextId, LayoutEpoch};

    #[test]
    fn an_id_reports_the_context_epoch_and_index_it_was_minted_with() {
        let epoch = LayoutEpoch::INITIAL.next();
        let id = RegionId::new(ContextId::ROOT, epoch, 3);

        assert_eq!(id.context(), ContextId::ROOT);
        assert_eq!(id.epoch(), epoch);
        assert_eq!(id.index(), 3);
    }

    #[test]
    fn ids_of_different_epochs_are_different_even_at_the_same_index() {
        let first = RegionId::new(ContextId::ROOT, LayoutEpoch::INITIAL, 0);
        let second = RegionId::new(ContextId::ROOT, LayoutEpoch::INITIAL.next(), 0);

        assert_ne!(first, second);
    }
}
