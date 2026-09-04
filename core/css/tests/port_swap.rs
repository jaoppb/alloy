//! Guards `PRD-007:94-95`: a mock `CascadeResolver` swaps in and changes a
//! computed value, with no change to `core/dom` or `core/graphics` — and this
//! test names no type from either of them when reading the `StyledTree`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use css::{CascadeResolver, MockCascadeResolver, StyleSheetSet, UaCascade, snapshot};

/// Compiles only if the argument is a `css::CssColor` — proof that reading the
/// `StyledTree` yields `css` types, not `core/dom` / `core/graphics` ones.
const fn accept_css_color(_: css::CssColor) {}

fn body_with_paragraph() -> (dom::DomTree, dom::NodeId) {
    let mut tree = dom::DomTree::new();
    let document = tree.document();
    let body = tree.create_element(dom::TagName::new("body").unwrap());
    tree.append_child(document, body).unwrap();
    let paragraph = tree.create_element(dom::TagName::new("p").unwrap());
    tree.append_child(body, paragraph).unwrap();
    (tree, document)
}

#[test]
fn a_mock_resolver_swaps_in_and_changes_a_computed_colour() {
    let (tree, document) = body_with_paragraph();
    let dom = snapshot(&tree, document);
    let sheets = StyleSheetSet::new();

    let via_ua = UaCascade::new()
        .resolve(&dom, &sheets)
        .expect("ua cascade resolves");
    let via_mock = MockCascadeResolver::new()
        .resolve(&dom, &sheets)
        .expect("mock cascade resolves");

    let ua_colour = via_ua
        .node(via_ua.root())
        .expect("styled root")
        .style()
        .color();
    let mock_colour = via_mock
        .node(via_mock.root())
        .expect("styled root")
        .style()
        .color();

    assert_ne!(
        ua_colour, mock_colour,
        "swapping the resolver must change the computed colour"
    );
    assert_eq!(mock_colour, MockCascadeResolver::SENTINEL_COLOR);

    // The value read back is a `css::CssColor`: no `core/dom` or
    // `core/graphics` type is named anywhere in this test's use of the
    // `StyledTree` API.
    accept_css_color(mock_colour);
}

#[test]
fn swapping_the_resolver_needs_no_change_to_the_other_aggregates() {
    let (tree, document) = body_with_paragraph();
    let dom = snapshot(&tree, document);
    let sheets = StyleSheetSet::new();

    let via_ua = UaCascade::new().resolve(&dom, &sheets).expect("resolves");
    let via_mock = MockCascadeResolver::new()
        .resolve(&dom, &sheets)
        .expect("resolves");

    assert_eq!(
        via_ua.len(),
        via_mock.len(),
        "both resolvers style exactly the nodes of the one shared snapshot"
    );
    assert_eq!(via_ua.root(), via_mock.root());
}
