//! [`Children`] — a node's child ids in document order (`ADR-0010:132` rule 4).
//! No public `Vec`; every mutation preserves order or changes it explicitly.

use crate::domain::node::NodeId;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Children {
    order: Vec<NodeId>,
}

impl Children {
    #[must_use]
    pub const fn new() -> Self {
        Self { order: Vec::new() }
    }

    pub(crate) fn push(&mut self, child: NodeId) {
        self.order.push(child);
    }

    /// Remove `child` if present; returns whether it was.
    pub(crate) fn remove_value(&mut self, child: NodeId) -> bool {
        match self.position(child) {
            Some(index) => {
                self.order.remove(index);
                true
            }
            None => false,
        }
    }

    /// Insert `child` immediately before `anchor`; `None` when `anchor` is absent.
    pub(crate) fn insert_before_value(&mut self, anchor: NodeId, child: NodeId) -> Option<()> {
        let index = self.position(anchor)?;
        self.order.insert(index, child);
        Some(())
    }

    #[must_use]
    pub fn position(&self, child: NodeId) -> Option<usize> {
        self.order.iter().position(|candidate| *candidate == child)
    }

    #[must_use]
    pub fn contains(&self, child: NodeId) -> bool {
        self.order.contains(&child)
    }

    pub fn iter(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.order.iter().copied()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.order.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}
