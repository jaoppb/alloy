//! Subresource discovery (v0.5 Phase I4, moved from `alloy` by #111): the port
//! that answers "what does this document reference", a pure function of a
//! [`DomSnapshot`]. Fetching is `alloy`'s job.

use css::DomSnapshot;
use network::Url;

use crate::domain::Subresources;

/// Finds the subresources a document references.
///
/// The browsing context is generic over this port, so where references come
/// from (tags today; CSS `url()`, scripts later) can grow without touching the
/// fetch machinery, while [`crate::SubresourceRequest`] keeps the fetchable
/// kinds closed.
pub trait SubresourceDiscoverer: Send + Sync {
    /// Walks `snapshot` and resolves every reference against `base` — the
    /// document's effective base URL, which the caller computes once per
    /// navigation with [`document_base_url`] so subresources and link clicks
    /// agree on it. An implementation must not re-apply `<base href>`: a
    /// relative one (`<base href="app/">`) would then be joined twice.
    fn discover(&self, snapshot: &DomSnapshot, base: &Url) -> Subresources;
}

/// The URL every relative reference of the document resolves against.
///
/// Subresources and link targets alike use it: the first `<base href>` in
/// document order resolved against the navigation URL (WHATWG HTML §4.2.3),
/// or the navigation URL itself when there is none. Performance-tuned sites
/// point `<base>` at a CDN, so ignoring it fetches every subresource (and
/// follows every relative link) from the wrong origin.
#[must_use]
pub fn document_base_url(snapshot: &DomSnapshot, navigation_url: &Url) -> Url {
    let declared_href = snapshot
        .nodes_in_document_order()
        .filter_map(|id| snapshot.node(id))
        .filter(|node| node.tag_str() == Some("base"))
        .find_map(|node| node.attribute("href"));
    declared_href
        .and_then(|href| navigation_url.join(href).ok())
        .unwrap_or_else(|| navigation_url.clone())
}
