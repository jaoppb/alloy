//! Determinism gate for the v0.5 B4 layout engine (`ADR-0016`): the same
//! `StyledTree` and `ViewportConstraints`, laid out 100 times, must produce a
//! byte-for-byte (here: field-for-field) identical `LayoutBoxTree` every time
//! — the same guarantee `core/graphics/tests/text_rendering.rs`'s
//! `a_hundred_renders_of_the_text_scene_are_byte_identical` proves for the
//! rasterizer.
//!
//! The fixture exercises all three formatting contexts B4 shipped: margin
//! collapse (two stacked paragraphs), the inline formatting context (a
//! wrapping run of text), and Flexbox (a row of grown/shrunk/wrapped items).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use css::{
    BlockLayout, CascadeResolver, LayoutEngine, Origin, UaCascade, ViewportConstraints,
    parse_stylesheet, snapshot,
};
use graphics::Au;

const fn au(pixels: i32) -> Au {
    Au::from_whole_px(pixels).unwrap()
}

fn element(tree: &mut dom::DomTree, parent: dom::NodeId, tag: &str) -> dom::NodeId {
    let node = tree.create_element(dom::TagName::new(tag).unwrap());
    tree.append_child(parent, node).unwrap();
    node
}

fn text(tree: &mut dom::DomTree, parent: dom::NodeId, content: &str) {
    let node = tree.create_text(dom::TextContent::new(content));
    tree.append_child(parent, node).unwrap();
}

fn class(tree: &mut dom::DomTree, node: dom::NodeId, value: &str) {
    tree.set_attribute(
        node,
        dom::AttributeName::new("class").unwrap(),
        dom::AttributeValue::new(value),
    )
    .unwrap();
}

/// `html > body > (p, p, div.flex > (div, div, div))`, margins, wrapping text
/// and Flexbox all in the same document.
fn document() -> (dom::DomTree, dom::NodeId) {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let html = element(&mut tree, root, "html");
    let body = element(&mut tree, html, "body");

    let first = element(&mut tree, body, "p");
    text(
        &mut tree,
        first,
        "A paragraph long enough to wrap across more than one line of text.",
    );
    let second = element(&mut tree, body, "p");
    text(
        &mut tree,
        second,
        "A second paragraph, right after the first one, margins collapsing.",
    );

    let flex = element(&mut tree, body, "div");
    class(&mut tree, flex, "flex");
    let a = element(&mut tree, flex, "div");
    let b = element(&mut tree, flex, "div");
    let c = element(&mut tree, flex, "div");
    let _ = (a, b, c);

    (tree, root)
}

const SOURCE: &str = "
    p { width: 220px; margin: 10px 0; }
    div.flex { display: flex; flex-wrap: wrap; width: 180px; }
    div.flex > div { width: 70px; height: 20px; flex-grow: 1; flex-shrink: 1; }
";

fn layout_once() -> css::LayoutBoxTree {
    let (tree, root) = document();
    let dom = snapshot(&tree, root);
    let sheet = parse_stylesheet(SOURCE, Origin::Author).expect("author CSS parses");
    let styled = UaCascade::new()
        .resolve(&dom, &sheet)
        .expect("cascade resolves");
    BlockLayout::new()
        .layout(&styled, &ViewportConstraints::new(au(800), au(600)))
        .expect("layout succeeds")
}

#[test]
fn a_hundred_layouts_of_the_same_document_are_field_for_field_identical() {
    let reference = layout_once();
    for attempt in 0..100 {
        let candidate = layout_once();
        assert_eq!(
            candidate, reference,
            "layout {attempt} diverged from the reference — box-model, inline or Flexbox is not deterministic"
        );
    }
}
