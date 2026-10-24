//! A node of the UI render tree, and its recursive hit test.

use graphics::Rect;

use super::hit_geometry::{contains, raw_edge};
use super::hit_target::UiHitTarget;
use super::node_children::UiNodeChildren;
use super::node_kind::UiNodeKind;

/// A node in the browser UI render tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiNode {
    kind: UiNodeKind,
    bounds: Rect,
    children: UiNodeChildren,
}

impl UiNode {
    #[must_use]
    pub const fn new(kind: UiNodeKind, bounds: Rect) -> Self {
        Self {
            kind,
            bounds,
            children: UiNodeChildren::new(),
        }
    }

    #[must_use]
    pub const fn with_children(kind: UiNodeKind, bounds: Rect, children: UiNodeChildren) -> Self {
        Self {
            kind,
            bounds,
            children,
        }
    }

    pub fn add_child(&mut self, child: Self) {
        self.children.push(child);
    }

    #[must_use]
    pub const fn kind(&self) -> &UiNodeKind {
        &self.kind
    }

    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    #[must_use]
    pub const fn children(&self) -> &UiNodeChildren {
        &self.children
    }

    /// Recursively hit-tests this node and its children in local coordinate space.
    #[must_use]
    pub fn hit_test(&self, x: f64, y: f64) -> Option<UiHitTarget> {
        if !contains(self.bounds, x, y) {
            return None;
        }
        let local_x = x - raw_edge(self.bounds.min_x());
        let local_y = y - raw_edge(self.bounds.min_y());
        self.hit_test_children(local_x, local_y)
            .or_else(|| self.as_target())
    }

    fn hit_test_children(&self, local_x: f64, local_y: f64) -> Option<UiHitTarget> {
        self.children
            .reverse_iter()
            .find_map(|child| child.hit_test(local_x, local_y))
    }

    const fn as_target(&self) -> Option<UiHitTarget> {
        match &self.kind {
            UiNodeKind::Omnibox => Some(UiHitTarget::Omnibox),
            UiNodeKind::Chrome => Some(UiHitTarget::Chrome),
            UiNodeKind::Link(region) => Some(UiHitTarget::PageLink(*region)),
            UiNodeKind::Content => Some(UiHitTarget::PageContent),
            UiNodeKind::Window => None,
        }
    }
}
