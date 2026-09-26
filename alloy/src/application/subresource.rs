//! Subresource discovery (v0.5 Phase I4).
//!
//! What a document references is a *closed* set of typed requests
//! ([`SubresourceRequest`]); *how* they are found is the open extension point
//! ([`SubresourceDiscoverer`]). Fetching and coalescing live in
//! [`crate::application::event_loop`] — this module only answers "what does
//! this document reference", a pure function of a [`DomSnapshot`].

use css::{DomSnapshot, NodeRef, SnapshotId};
use graphics::{Color, Framebuffer, ImageId, SurfaceSize};
use network::Url;

/// A `<link rel="stylesheet" href>` target, already resolved against the
/// page's base URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StylesheetRequest {
    url: Url,
}

impl StylesheetRequest {
    /// A request for the stylesheet at `url`.
    #[must_use]
    pub const fn new(url: Url) -> Self {
        Self { url }
    }

    /// Where to fetch the stylesheet from.
    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.url
    }
}

/// An `<img src>` target, tagged with the [`ImageId`] `paint_box_tree`
/// (`crate::application::paint`) will look up for that same node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageRequest {
    id: ImageId,
    url: Url,
}

impl ImageRequest {
    /// A request for the image at `url`, painted for the node `id`.
    #[must_use]
    pub const fn new(id: ImageId, url: Url) -> Self {
        Self { id, url }
    }

    /// The identity the painter looks this image up by.
    #[must_use]
    pub const fn id(&self) -> ImageId {
        self.id
    }

    /// Where to fetch the image from.
    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.url
    }
}

/// Everything a page can ask to have fetched — closed on purpose: a new kind
/// of subresource is a new variant the fetch loop must then handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubresourceRequest {
    /// An external stylesheet.
    Stylesheet(StylesheetRequest),
    /// A decodable image.
    Image(ImageRequest),
}

/// One page's discovered subresources, in document order.
///
/// A reference that fails to resolve (a relative URL with no scheme, an
/// unsupported one) is silently dropped — a broken subresource link must
/// never abort navigation.
#[derive(Clone, Debug, Default)]
pub struct Subresources {
    requests: Vec<SubresourceRequest>,
}

impl Subresources {
    /// Appends `request`.
    pub fn push(&mut self, request: SubresourceRequest) {
        self.requests.push(request);
    }
}

impl IntoIterator for Subresources {
    type Item = SubresourceRequest;
    type IntoIter = std::vec::IntoIter<SubresourceRequest>;

    fn into_iter(self) -> Self::IntoIter {
        self.requests.into_iter()
    }
}

/// Finds the subresources a document references.
///
/// The event loop is generic over this port, so where references come from
/// (tags today; CSS `url()`, scripts later) can grow without touching the
/// fetch machinery, while [`SubresourceRequest`] keeps the fetchable kinds
/// closed.
pub trait SubresourceDiscoverer: Send + Sync {
    /// Walks `snapshot` and resolves every reference against `base`.
    fn discover(&self, snapshot: &DomSnapshot, base: &Url) -> Subresources;
}

/// Discovers `<link rel="stylesheet">` and `<img>` — the two
/// subresource-bearing tags v0.5 understands.
#[derive(Clone, Copy, Debug, Default)]
pub struct MarkupDiscoverer;

impl SubresourceDiscoverer for MarkupDiscoverer {
    fn discover(&self, snapshot: &DomSnapshot, base: &Url) -> Subresources {
        let mut found = Subresources::default();
        for id in snapshot.nodes_in_document_order() {
            let Some(node) = snapshot.node(id) else {
                continue;
            };
            let Some(request) = request_for(id, node, base) else {
                continue;
            };
            found.push(request);
        }
        found
    }
}

fn request_for(id: SnapshotId, node: NodeRef<'_>, base: &Url) -> Option<SubresourceRequest> {
    match node.tag_str()? {
        "link" => stylesheet_request(node, base).map(SubresourceRequest::Stylesheet),
        "img" => image_request(id, node, base).map(SubresourceRequest::Image),
        _ => None,
    }
}

fn stylesheet_request(node: NodeRef<'_>, base: &Url) -> Option<StylesheetRequest> {
    if node.attribute("rel") != Some("stylesheet") {
        return None;
    }
    let href = node.attribute("href")?;
    base.join(href).ok().map(StylesheetRequest::new)
}

fn image_request(id: SnapshotId, node: NodeRef<'_>, base: &Url) -> Option<ImageRequest> {
    let src = node.attribute("src")?;
    let url = base.join(src).ok()?;
    let image_id = ImageId::new(u32::try_from(id.index()).unwrap_or(u32::MAX));
    Some(ImageRequest::new(image_id, url))
}
/// A 1×1, fully transparent placeholder.
///
/// `core/css` marks every replaced-element box (`<img>` included)
/// [`css::IntrinsicSize::Pending`] the moment it sees the tag — regardless of
/// whether the resource has loaded (`core/css/src/domain/computed/intrinsic.rs`)
/// — and `paint_box_tree` (`crate::application::paint`) always emits a
/// `DrawImage` command for a pending box. Every id [`discover`] finds must
/// therefore resolve to *something* from the very first paint, or rendering
/// fails outright before a single fetch has had a chance to complete.
// "genuinely impossible state" `.expect()` carve-out CLAUDE.md documents:
// 1x1 is a compile-time-known-valid, non-zero surface size (same pattern as
// core/css/src/infrastructure/ua_sheet.rs). `pub(crate)`, not `pub`: an
// impossible-state `.expect()` is only a defensible carve-out for a crate's
// own internal callers, never for an external API `missing_panics_doc` would
// otherwise rightly ask to document.
#[allow(clippy::expect_used)]
#[must_use]
pub(crate) fn placeholder_framebuffer() -> Framebuffer {
    let size = SurfaceSize::new(1, 1).expect("1×1 is always a valid, non-zero surface size");
    Framebuffer::filled(size, Color::TRANSPARENT)
        .expect("a freshly built 1×1 framebuffer always fits its own pixel buffer")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{MarkupDiscoverer, SubresourceDiscoverer, SubresourceRequest};
    use network::Url;

    fn discover(markup: &str) -> Vec<SubresourceRequest> {
        let tree = html::parse(markup).unwrap();
        let snapshot = css::snapshot(&tree, tree.document());
        let base = Url::parse("http://example.com/dir/page.html").unwrap();
        MarkupDiscoverer
            .discover(&snapshot, &base)
            .into_iter()
            .collect()
    }

    #[test]
    fn finds_stylesheets_and_images_resolved_against_the_base_in_document_order() {
        let found = discover(
            r#"<html><head><link rel="stylesheet" href="a.css"></head>
               <body><img src="/pic.png"></body></html>"#,
        );
        let [
            SubresourceRequest::Stylesheet(sheet),
            SubresourceRequest::Image(image),
        ] = found.as_slice()
        else {
            panic!("expected one stylesheet then one image, got {found:?}");
        };
        assert_eq!(sheet.url().to_string(), "http://example.com/dir/a.css");
        assert_eq!(image.url().to_string(), "http://example.com/pic.png");
    }

    #[test]
    fn ignores_non_stylesheet_links_and_tags_missing_their_reference() {
        let found = discover(
            r#"<html><head><link rel="icon" href="x.ico"><link rel="stylesheet"></head>
               <body><img></body></html>"#,
        );
        assert!(found.is_empty(), "nothing fetchable here, got {found:?}");
    }
}
