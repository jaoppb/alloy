//! Guards the end-to-end path `dom::DomTree -> snapshot -> CascadeResolver ->
//! StyledTree -> LayoutEngine -> LayoutBoxTree` with rectangle assertions
//! (`roadmap §5`, `PRD-007:79-80`): the built-in adapters produce sane,
//! non-overlapping boxes in document order.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use css::{
    BlockLayout, CascadeResolver, LayoutEngine, StyleSheetSet, UaCascade, ViewportConstraints,
    snapshot,
};
use graphics::Au;

const fn au(pixels: i32) -> Au {
    Au::from_whole_px(pixels).unwrap()
}

/// `html > body > (h1 > "Alloy") + (p > "First pixel")`.
fn document() -> (dom::DomTree, dom::NodeId) {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let html = child(&mut tree, root, "html");
    let body = child(&mut tree, html, "body");
    let heading = child(&mut tree, body, "h1");
    text(&mut tree, heading, "Alloy");
    let paragraph = child(&mut tree, body, "p");
    text(&mut tree, paragraph, "First pixel");
    (tree, root)
}

fn child(tree: &mut dom::DomTree, parent: dom::NodeId, name: &str) -> dom::NodeId {
    let node = tree.create_element(dom::TagName::new(name).unwrap());
    tree.append_child(parent, node).unwrap();
    node
}

fn text(tree: &mut dom::DomTree, parent: dom::NodeId, content: &str) {
    let node = tree.create_text(dom::TextContent::new(content));
    tree.append_child(parent, node).unwrap();
}

#[test]
fn the_reference_pipeline_stacks_block_boxes_in_document_order() {
    let (tree, root) = document();
    let dom = snapshot(&tree, root);

    let styled = UaCascade::new()
        .resolve(&dom, &StyleSheetSet::new())
        .expect("cascade resolves");
    let boxes = BlockLayout::new()
        .layout(&styled, &ViewportConstraints::new(au(800), au(600)))
        .expect("layout succeeds");

    assert!(
        boxes.len() >= 3,
        "html, body, h1 and p all generate block boxes"
    );

    let ordered: Vec<_> = boxes.boxes_in_document_order().collect();
    for pair in ordered.windows(2) {
        let earlier = pair[0].content();
        let later = pair[1].content();
        assert!(
            earlier.min_y() <= later.min_y(),
            "a box earlier in document order is never lower on the page"
        );
    }

    for laid_out in boxes.boxes_in_document_order() {
        assert!(
            laid_out.content().max_x() <= au(800),
            "every box fits inside the viewport width"
        );
    }
}

#[test]
fn a_heading_is_laid_out_above_the_following_paragraph() {
    let (tree, root) = document();
    let dom = snapshot(&tree, root);
    let styled = UaCascade::new()
        .resolve(&dom, &StyleSheetSet::new())
        .expect("cascade resolves");
    let boxes = BlockLayout::new()
        .layout(&styled, &ViewportConstraints::new(au(800), au(600)))
        .expect("layout succeeds");

    // The h1 is the third block box (html, body, h1); the p is the fourth.
    let ordered: Vec<_> = boxes.boxes_in_document_order().collect();
    let heading = ordered.get(2).expect("an h1 box");
    let paragraph = ordered.get(3).expect("a p box");

    assert!(
        heading.content().max_y() <= paragraph.content().min_y(),
        "the heading's bottom edge is at or above the paragraph's top edge"
    );
}
