//! Subresource discovery (v0.5 Phase I4).
//!
//! `<link rel="stylesheet">` and `<img>`, resolved against the page's base
//! URL. Fetching and coalescing live in [`crate::application::event_loop`] —
//! this module only answers "what does this document reference", a pure
//! function of a [`DomSnapshot`].

use css::{DomSnapshot, NodeRef, SnapshotId};
use graphics::{Color, Framebuffer, ImageId, SurfaceSize};
use network::Url;

/// One page's discovered subresources, already resolved against the page's
/// base URL.
///
/// A reference that fails to resolve (a relative URL with no scheme, an
/// unsupported one) is silently dropped — a broken subresource link must
/// never abort navigation.
#[derive(Clone, Debug, Default)]
pub struct Subresources {
    /// `<link rel="stylesheet" href>` targets, in document order.
    pub stylesheets: Vec<Url>,
    /// `<img src>` targets, tagged with the [`ImageId`] `paint_box_tree`
    /// (`crate::application::paint`) will look up for that same node.
    pub images: Vec<(ImageId, Url)>,
}

/// Walks `snapshot` in document order for the two subresource-bearing tags
/// this crate understands.
#[must_use]
pub fn discover(snapshot: &DomSnapshot, base: &Url) -> Subresources {
    let mut found = Subresources::default();
    for id in snapshot.nodes_in_document_order() {
        let Some(node) = snapshot.node(id) else {
            continue;
        };
        match node.tag() {
            Some("link") => discover_stylesheet(node, base, &mut found),
            Some("img") => discover_image(id, node, base, &mut found),
            _ => {}
        }
    }
    found
}

fn discover_stylesheet(node: NodeRef<'_>, base: &Url, found: &mut Subresources) {
    if node.attribute("rel") != Some("stylesheet") {
        return;
    }
    let Some(href) = node.attribute("href") else {
        return;
    };
    if let Ok(url) = base.join(href) {
        found.stylesheets.push(url);
    }
}

fn discover_image(id: SnapshotId, node: NodeRef<'_>, base: &Url, found: &mut Subresources) {
    let Some(src) = node.attribute("src") else {
        return;
    };
    let Ok(url) = base.join(src) else {
        return;
    };
    let image_id = ImageId::new(u32::try_from(id.index()).unwrap_or(u32::MAX));
    found.images.push((image_id, url));
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
