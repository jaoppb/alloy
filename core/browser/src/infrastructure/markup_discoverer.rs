//! The tag-based [`SubresourceDiscoverer`]: `<link rel="stylesheet">` and
//! `<img>`, the two subresource-bearing tags v0.5 understands.

use css::{DomSnapshot, NodeRef, SnapshotId};
use graphics::ImageId;
use network::Url;

use crate::application::SubresourceDiscoverer;
use crate::domain::{ImageRequest, StylesheetRequest, SubresourceRequest, Subresources};

/// Discovers `<link rel="stylesheet">` and `<img>` from the markup.
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
    if !rel_is_stylesheet(node) {
        return None;
    }
    let href = node.attribute("href")?;
    let media = css::parse_media_attribute(node.attribute(html::AttributeName::media().as_str()));
    base.join(href)
        .ok()
        .map(|url| StylesheetRequest::new(url, media))
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

fn image_request(id: SnapshotId, node: NodeRef<'_>, base: &Url) -> Option<ImageRequest> {
    let src = node.attribute("src")?;
    let url = base.join(src).ok()?;
    let image_id = ImageId::new(u32::try_from(id.index()).unwrap_or(u32::MAX));
    Some(ImageRequest::new(image_id, url))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::MarkupDiscoverer;
    use crate::application::{SubresourceDiscoverer, document_base_url};
    use crate::domain::SubresourceRequest;
    use network::Url;

    fn discover_from(markup: &str, navigation_url: &str) -> Vec<SubresourceRequest> {
        let tree = dom::parse(markup).unwrap().into_tree();
        let snapshot = css::snapshot(&tree, tree.document());
        let base = document_base_url(&snapshot, &Url::parse(navigation_url).unwrap());
        MarkupDiscoverer
            .discover(&snapshot, &base)
            .into_iter()
            .collect()
    }

    fn discover(markup: &str) -> Vec<SubresourceRequest> {
        discover_from(markup, "http://example.com/dir/page.html")
    }

    fn stylesheet_urls(found: &[SubresourceRequest]) -> Vec<String> {
        found
            .iter()
            .filter_map(|request| match request {
                SubresourceRequest::Stylesheet(sheet) => Some(sheet.url().to_string()),
                SubresourceRequest::Image(_) => None,
            })
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

    #[test]
    fn a_multi_token_rel_still_counts_as_a_stylesheet_link() {
        let found = discover_from(
            "<link rel=\"preload stylesheet\" href=\"/a.css\">\
             <link rel=\"Stylesheet\" href=\"/b.css\">\
             <link rel=\"preload\" href=\"/c.css\">",
            "https://example.com/index.html",
        );
        assert_eq!(
            stylesheet_urls(&found),
            vec![
                "https://example.com/a.css".to_owned(),
                "https://example.com/b.css".to_owned(),
            ],
            "`preload stylesheet` and `Stylesheet` match; bare `preload` does not"
        );
    }

    #[test]
    fn a_base_href_element_moves_the_origin_relative_links_resolve_against() {
        let found = discover_from(
            "<base href=\"https://cdn.example/assets/\">\
             <link rel=\"stylesheet\" href=\"site.css\">",
            "https://example.com/index.html",
        );
        assert_eq!(
            stylesheet_urls(&found),
            vec!["https://cdn.example/assets/site.css".to_owned()],
            "`<base href>` redirects the relative stylesheet to the CDN origin"
        );
    }

    #[test]
    fn a_relative_base_href_resolves_against_the_navigation_url_exactly_once() {
        let found = discover_from(
            "<base href=\"app/\"><link rel=\"stylesheet\" href=\"site.css\">",
            "https://example.com/index.html",
        );
        assert_eq!(
            stylesheet_urls(&found),
            vec!["https://example.com/app/site.css".to_owned()],
            "`app/` is joined to the navigation URL once, never to itself again"
        );
    }

    #[test]
    fn without_a_base_element_the_document_base_is_the_navigation_url() {
        let tree = dom::parse("<p>no base here</p>").unwrap().into_tree();
        let snapshot = css::snapshot(&tree, tree.document());
        let navigation_url = Url::parse("https://example.com/dir/page.html").unwrap();
        assert_eq!(
            document_base_url(&snapshot, &navigation_url),
            navigation_url
        );
    }

    #[test]
    fn stylesheet_link_media_attribute_is_carried_on_request() {
        let found = discover(
            r#"<html><head>
                 <link rel="stylesheet" media="print" href="print.css">
                 <link rel="stylesheet" media="(min-width: 300px)" href="wide.css">
               </head></html>"#,
        );
        let [
            SubresourceRequest::Stylesheet(print_sheet),
            SubresourceRequest::Stylesheet(wide_sheet),
        ] = found.as_slice()
        else {
            panic!("expected two stylesheet requests, got {found:?}");
        };
        assert!(print_sheet.media().is_never());
        assert_eq!(wide_sheet.media().condition_count(), 1);
        assert!(!wide_sheet.media().is_never());
    }
}
