//! The clickable link regions of the last layout, addressed by id.
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
pub struct LayoutEpoch(u64);

impl LayoutEpoch {
    const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// A link region of one layout of one context's document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionId {
    context: ContextId,
    epoch: LayoutEpoch,
    index: usize,
}

/// A link area as layout found it: where it is, and where it points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkTarget {
    area: Rect,
    href: String,
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
    id: RegionId,
    bounds: Rect,
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

/// Why a [`RegionId`] could not be resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RegionError {
    /// Minted by another browsing context.
    #[error("region belongs to {0}, not this context")]
    WrongContext(ContextId),
    /// Minted for an earlier layout or document.
    #[error("region is from an earlier layout (epoch {minted:?}, current {current:?})")]
    Stale {
        /// The epoch the id was minted for.
        minted: LayoutEpoch,
        /// The epoch the document is in now.
        current: LayoutEpoch,
    },
    /// No such link in the current layout.
    #[error("region index {0} is out of range")]
    Unknown(usize),
}

/// The links of the current layout, plus the epoch they were minted in.
#[derive(Clone, Debug)]
pub struct ContentRegions {
    context: ContextId,
    epoch: LayoutEpoch,
    targets: Vec<LinkTarget>,
}

impl ContentRegions {
    /// No links, epoch zero.
    #[must_use]
    pub const fn new(context: ContextId) -> Self {
        Self {
            context,
            epoch: LayoutEpoch(0),
            targets: Vec::new(),
        }
    }

    /// Adopts the links of a fresh layout under a new epoch.
    pub fn replace(&mut self, targets: Vec<LinkTarget>) {
        self.epoch = self.epoch.next();
        self.targets = targets;
    }

    /// Forgets every link under a new epoch — the document they described is gone.
    pub fn invalidate(&mut self) {
        self.replace(Vec::new());
    }

    /// The epoch ids are currently minted in.
    #[must_use]
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    /// The regions of the current layout, in paint order.
    pub fn iter(&self) -> impl Iterator<Item = ContentRegion> + '_ {
        self.targets
            .iter()
            .enumerate()
            .map(|(index, target)| ContentRegion {
                id: RegionId {
                    context: self.context,
                    epoch: self.epoch,
                    index,
                },
                bounds: target.area,
            })
    }

    /// Where the region `id` points, if it is still from the current layout.
    ///
    /// # Errors
    ///
    /// [`RegionError`] when `id` belongs to another context, to an earlier
    /// layout or document, or to no link at all.
    pub fn href(&self, id: RegionId) -> Result<&str, RegionError> {
        if id.context != self.context {
            return Err(RegionError::WrongContext(id.context));
        }
        if id.epoch != self.epoch {
            return Err(RegionError::Stale {
                minted: id.epoch,
                current: self.epoch,
            });
        }
        self.targets
            .get(id.index)
            .map(LinkTarget::href)
            .ok_or(RegionError::Unknown(id.index))
    }
}

#[cfg(test)]
mod tests {
    use graphics::{Au, Point, Rect, Size};

    use super::{ContentRegions, LinkTarget, RegionError};
    use crate::domain::ContextId;

    fn link(href: &str) -> LinkTarget {
        let au = |pixels| Au::from_whole_px(pixels).unwrap();
        let area = Rect::new(Point::new(au(0), au(0)), Size::new(au(10), au(10)).unwrap());
        LinkTarget::new(area, href)
    }

    fn regions_with(hrefs: &[&str]) -> ContentRegions {
        let mut regions = ContentRegions::new(ContextId::ROOT);
        regions.replace(hrefs.iter().map(|href| link(href)).collect());
        regions
    }

    #[test]
    fn a_minted_region_resolves_to_its_href_in_paint_order() {
        let regions = regions_with(&["/a", "/b"]);
        let ids: Vec<_> = regions.iter().map(|region| region.id()).collect();

        assert_eq!(regions.href(ids[0]), Ok("/a"));
        assert_eq!(regions.href(ids[1]), Ok("/b"));
    }

    #[test]
    fn a_region_from_before_a_relayout_is_stale_even_when_its_index_still_exists() {
        let mut regions = regions_with(&["/old"]);
        let old = regions.iter().next().unwrap().id();
        regions.replace(vec![link("/new")]);

        assert!(matches!(regions.href(old), Err(RegionError::Stale { .. })));
    }

    #[test]
    fn invalidating_refuses_every_earlier_region_and_lists_none() {
        let mut regions = regions_with(&["/a"]);
        let old = regions.iter().next().unwrap().id();
        regions.invalidate();

        assert!(matches!(regions.href(old), Err(RegionError::Stale { .. })));
        assert_eq!(regions.iter().count(), 0);
    }

    #[test]
    fn an_index_past_the_last_link_is_unknown_not_a_panic() {
        let two = regions_with(&["/a", "/b"]);
        let past_the_end = two.iter().nth(1).unwrap().id();
        let mut one = ContentRegions::new(ContextId::ROOT);
        one.replace(vec![link("/a")]);
        // Same epoch number (1), but only one link: index 1 does not exist.
        assert_eq!(one.href(past_the_end), Err(RegionError::Unknown(1)));
    }
}
