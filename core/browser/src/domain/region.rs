//! The links of a layout, with the `href`s `core/ui` never sees.
//!
//! `core/ui` hit-tests `region::ContentRegion` rectangles and hands the winning
//! `region::RegionId` back (#110 decision 12); the browser — which owns the
//! links and the link policy — resolves it in [`super::ContentRegions`].

use graphics::Rect;

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
    pub(super) const fn area(&self) -> Rect {
        self.area
    }

    /// The link destination as written, unresolved.
    #[must_use]
    pub(super) fn href(&self) -> &str {
        &self.href
    }
}

/// The link areas one layout found, in paint order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkTargets(Vec<LinkTarget>);

impl LinkTargets {
    /// No links.
    #[must_use]
    pub const fn none() -> Self {
        Self(Vec::new())
    }

    pub(super) fn get(&self, index: usize) -> Option<&LinkTarget> {
        self.0.get(index)
    }

    /// The links in paint order.
    pub fn iter(&self) -> std::slice::Iter<'_, LinkTarget> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a LinkTargets {
    type Item = &'a LinkTarget;
    type IntoIter = std::slice::Iter<'a, LinkTarget>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<LinkTarget> for LinkTargets {
    fn from_iter<T: IntoIterator<Item = LinkTarget>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
