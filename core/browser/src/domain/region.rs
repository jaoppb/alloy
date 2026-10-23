//! Ids and values for the clickable link regions of a layout.
//!
//! `core/ui` never sees an `href` (#110 decision 12): it hit-tests
//! [`ContentRegion`] rectangles and hands the winning [`RegionId`] back, and the
//! browser — which owns the links and the link policy — resolves it. A region
//! is only valid for the layout that minted it, so the id carries a
//! [`LayoutEpoch`]; one that outlived a relayout or a document swap is refused
//! instead of resolving to whatever now sits at that index (decision 13).

use graphics::Rect;

use crate::domain::ContextId;

/// Which layout of the document a [`RegionId`] was minted for. Bumps on every
/// layout and every document swap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LayoutEpoch(pub(super) u64);

impl LayoutEpoch {
    pub(super) const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// A link region of one layout of one context's document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionId {
    pub(super) context: ContextId,
    pub(super) epoch: LayoutEpoch,
    pub(super) index: usize,
}

/// A link area as layout found it: where it is, and where it points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkTarget {
    pub(super) area: Rect,
    pub(super) href: String,
}

impl LinkTarget {
    /// A link whose clickable `area` is in document space and which points at
    /// `href` exactly as the markup spelled it.
    #[must_use]
    pub fn new(area: Rect, href: impl Into<String>) -> Self {
        Self {
            area,
            href: href.into(),
        }
    }

    /// Bounding rectangle in document space.
    #[must_use]
    pub const fn area(&self) -> Rect {
        self.area
    }

    /// The link destination as written, unresolved.
    #[must_use]
    pub fn href(&self) -> &str {
        &self.href
    }
}

/// What `core/ui` is allowed to know of a link: where it is and its id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentRegion {
    pub(super) id: RegionId,
    pub(super) bounds: Rect,
}

impl ContentRegion {
    /// The id to hand back to the browser when this region is hit.
    #[must_use]
    pub const fn id(&self) -> RegionId {
        self.id
    }

    /// Bounding rectangle in document space.
    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}
