//! What a pointer landed on.

use region::RegionId;

/// The interactive target hit by a pointer event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiHitTarget {
    /// The omnibox input field was clicked.
    Omnibox,
    /// The chrome top bar background was clicked.
    Chrome,
    /// A hyperlink within the web page was clicked.
    PageLink(RegionId),
    /// The page content area (outside of links) was clicked.
    PageContent,
}
