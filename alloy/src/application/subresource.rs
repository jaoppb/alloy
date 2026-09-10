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
pub fn discover(snapshot: &DomSnapshot, nav_url: &Url) -> Subresources {
    let base = effective_base(snapshot, nav_url);
    let mut found = Subresources::default();
    for id in snapshot.nodes_in_document_order() {
        let Some(node) = snapshot.node(id) else {
            continue;
        };
        match node.tag() {
            Some("link") => discover_stylesheet(node, &base, &mut found),
            Some("img") => discover_image(id, node, &base, &mut found),
            _ => {}
        }
    }
    found
}

/// The URL relative references resolve against: the first `<base href>` in
/// document order resolved against the navigation URL (WHATWG HTML §4.2.3),
/// or the navigation URL itself when there is none. Performance-tuned sites
/// point `<base>` at a CDN, so ignoring it fetches every subresource from the
/// wrong origin.
fn effective_base(snapshot: &DomSnapshot, nav_url: &Url) -> Url {
    for id in snapshot.nodes_in_document_order() {
        let Some(node) = snapshot.node(id) else {
            continue;
        };
        if node.tag() != Some("base") {
            continue;
        }
        let Some(href) = node.attribute("href") else {
            continue;
        };
        return nav_url.join(href).unwrap_or_else(|_| nav_url.clone());
    }
    nav_url.clone()
}

fn discover_stylesheet(node: NodeRef<'_>, base: &Url, found: &mut Subresources) {
    if !rel_is_stylesheet(node) {
        return;
    }
    let Some(href) = node.attribute("href") else {
        return;
    };
    if let Ok(url) = base.join(href) {
        found.stylesheets.push(url);
    }
}

/// `rel` is a space-separated, case-insensitive token set: `rel="stylesheet"`
/// but also `rel="preload stylesheet"` and `rel="Stylesheet"`. The
/// `<link rel="preload" ... onload="this.rel='stylesheet'">` swap pattern is
/// still missed — it needs script execution (v0.7).
fn rel_is_stylesheet(node: NodeRef<'_>) -> bool {
    node.attribute("rel").is_some_and(|rel| {
        rel.split_ascii_whitespace()
            .any(|token| token.eq_ignore_ascii_case("stylesheet"))
    })
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use css::snapshot;
    use network::Url;

    use super::discover;

    fn stylesheets_of(html: &str, nav_url: &str) -> Vec<String> {
        let tree = html::parse(html).expect("the fixture parses");
        let dom = snapshot(&tree, tree.document());
        let base = Url::parse(nav_url).expect("a valid navigation URL");
        discover(&dom, &base)
            .stylesheets
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn a_multi_token_rel_still_counts_as_a_stylesheet_link() {
        let found = stylesheets_of(
            "<link rel=\"preload stylesheet\" href=\"/a.css\">\
             <link rel=\"Stylesheet\" href=\"/b.css\">\
             <link rel=\"preload\" href=\"/c.css\">",
            "https://example.com/index.html",
        );
        assert_eq!(
            found,
            vec![
                "https://example.com/a.css".to_owned(),
                "https://example.com/b.css".to_owned(),
            ],
            "`preload stylesheet` and `Stylesheet` match; bare `preload` does not"
        );
    }

    #[test]
    fn a_base_href_element_moves_the_origin_relative_links_resolve_against() {
        let found = stylesheets_of(
            "<base href=\"https://cdn.example/assets/\">\
             <link rel=\"stylesheet\" href=\"site.css\">",
            "https://example.com/index.html",
        );
        assert_eq!(
            found,
            vec!["https://cdn.example/assets/site.css".to_owned()],
            "`<base href>` redirects the relative stylesheet to the CDN origin"
        );
    }
}
