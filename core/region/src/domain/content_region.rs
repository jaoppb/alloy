//! What `core/ui` is allowed to know of a link.

use graphics::Rect;

use super::RegionId;

/// Where a link is, and the id to hand back to the browser when it is hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentRegion {
    id: RegionId,
    bounds: Rect,
}

impl ContentRegion {
    /// A region of the document, `bounds` in document space.
    #[must_use]
    pub const fn new(id: RegionId, bounds: Rect) -> Self {
        Self { id, bounds }
    }

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

#[cfg(test)]
mod tests {
    use graphics::{Au, Point, Rect, Size};

    use super::ContentRegion;
    use crate::domain::{ContextId, LayoutEpoch, RegionId};

    #[test]
    fn a_region_keeps_the_id_and_bounds_it_was_built_with() {
        let id = RegionId::new(ContextId::ROOT, LayoutEpoch::INITIAL, 0);
        let side = Au::from_whole_px(8).expect("fits the Au envelope");
        let bounds = Rect::new(
            Point::new(Au::ZERO, Au::ZERO),
            Size::new(side, side).expect("non-negative size"),
        );
        let region = ContentRegion::new(id, bounds);

        assert_eq!(region.id(), id);
        assert_eq!(region.bounds(), bounds);
    }
}
