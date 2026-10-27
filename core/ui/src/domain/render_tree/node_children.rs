//! The children of a render tree node.

use super::node::UiNode;

/// A first-class collection of child UI nodes in the render tree (ADR-0010).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiNodeChildren(Vec<UiNode>);

impl UiNodeChildren {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, child: UiNode) {
        self.0.push(child);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, UiNode> {
        self.0.iter()
    }

    pub fn reverse_iter(&self) -> std::iter::Rev<std::slice::Iter<'_, UiNode>> {
        self.0.iter().rev()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl FromIterator<UiNode> for UiNodeChildren {
    fn from_iter<T: IntoIterator<Item = UiNode>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'a> IntoIterator for &'a UiNodeChildren {
    type Item = &'a UiNode;
    type IntoIter = std::slice::Iter<'a, UiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for UiNodeChildren {
    type Item = UiNode;
    type IntoIter = std::vec::IntoIter<UiNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
