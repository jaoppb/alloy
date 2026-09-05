//! Rectangle proofs for the box model (v0.5 B4): margin collapse (CSS 2.1
//! §8.3.1) and `box-sizing` (CSS Box Sizing L3 §5). Every fixture uses `div`
//! elements, which the embedded UA sheet (`core/css/assets/ua.css`) leaves at
//! `ComputedStyle::initial()` — block display, zero margin — so only the
//! author CSS each test declares is in play.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use css::{
    BlockLayout, CascadeResolver, DomSnapshot, LayoutBox, LayoutEngine, Origin, SnapshotId,
    UaCascade, ViewportConstraints, parse_stylesheet, snapshot,
};
use graphics::Au;

const fn au(pixels: i32) -> Au {
    Au::from_whole_px(pixels).unwrap()
}

fn element(tree: &mut dom::DomTree, parent: dom::NodeId, id: &str) -> dom::NodeId {
    let node = tree.create_element(dom::TagName::new("div").unwrap());
    tree.append_child(parent, node).unwrap();
    tree.set_attribute(
        node,
        dom::AttributeName::new("id").unwrap(),
        dom::AttributeValue::new(id),
    )
    .unwrap();
    node
}

fn layout_boxes(tree: &dom::DomTree, root: dom::NodeId, source: &str) -> css::LayoutBoxTree {
    let dom = snapshot(tree, root);
    let sheet = parse_stylesheet(source, Origin::Author).expect("author CSS parses");
    let styled = UaCascade::new()
        .resolve(&dom, &sheet)
        .expect("cascade resolves");
    BlockLayout::new()
        .layout(&styled, &ViewportConstraints::new(au(800), au(600)))
        .expect("layout succeeds")
}

/// The `SnapshotId` of the sole element carrying `id="<id>"`.
fn find(dom: &DomSnapshot, id: &str) -> SnapshotId {
    dom.nodes_in_document_order()
        .find(|&node| dom.node(node).and_then(|n| n.attribute("id")) == Some(id))
        .unwrap_or_else(|| panic!("no element with id=\"{id}\""))
}

fn box_of<'a>(boxes: &'a css::LayoutBoxTree, dom: &DomSnapshot, id: &str) -> &'a LayoutBox {
    boxes
        .box_of(find(dom, id))
        .unwrap_or_else(|| panic!("no box for id=\"{id}\""))
}

// ---- margin collapse: adjacent siblings ------------------------------------

#[test]
fn adjacent_sibling_margins_collapse_to_the_larger_one() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "container");
    element(&mut tree, container, "a");
    element(&mut tree, container, "b");

    let source = "
        #a { height: 10px; margin-bottom: 20px; }
        #b { height: 10px; margin-top: 10px; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);
    let a = box_of(&boxes, &dom, "a");
    let b = box_of(&boxes, &dom, "b");

    let gap = b.border_box().min_y().checked_sub(a.border_box().max_y());
    assert_eq!(
        gap,
        Some(au(20)),
        "20px and 10px adjoining margins collapse to the larger, 20px — not their 30px sum"
    );
}

// ---- margin collapse: parent / first child ---------------------------------

#[test]
fn a_parents_top_margin_collapses_with_its_first_childs() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let parent = element(&mut tree, root, "parent");
    element(&mut tree, parent, "child");

    let source = "
        #parent { margin-top: 5px; }
        #child { height: 10px; margin-top: 15px; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);
    let parent_box = box_of(&boxes, &dom, "parent");
    let child_box = box_of(&boxes, &dom, "child");

    assert_eq!(
        parent_box.border_box().min_y(),
        child_box.border_box().min_y(),
        "the child's 15px top margin escapes upward through the parent (no border/padding \
         separates them) instead of pushing the child down inside it"
    );
}

// ---- margin collapse: parent / last child -----------------------------------

#[test]
fn a_parents_bottom_margin_collapses_with_its_last_childs() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let parent = element(&mut tree, root, "parent");
    element(&mut tree, parent, "child");
    let after = element(&mut tree, root, "after");
    let _ = after;

    let source = "
        #child { height: 10px; margin-bottom: 12px; }
        #after { height: 5px; margin-top: 3px; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);
    let parent_box = box_of(&boxes, &dom, "parent");
    let child_box = box_of(&boxes, &dom, "child");
    let after_box = box_of(&boxes, &dom, "after");

    assert_eq!(
        parent_box.border_box().max_y(),
        child_box.border_box().max_y(),
        "the child's 12px bottom margin escapes downward through the parent (no border/padding, \
         no declared height, separates them)"
    );
    let gap = after_box
        .border_box()
        .min_y()
        .checked_sub(parent_box.border_box().max_y());
    assert_eq!(
        gap,
        Some(au(12)),
        "the escaped 12px then adjoins `after`'s 3px top margin as ordinary siblings, \
         collapsing to the larger 12px"
    );
}

// ---- box-sizing -------------------------------------------------------------

#[test]
fn box_sizing_decides_whether_padding_and_border_grow_the_box_or_eat_the_content() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    element(&mut tree, root, "content");
    element(&mut tree, root, "border");

    let source = "
        #content {
            width: 200px;
            padding: 10px;
            border-width: 5px;
            box-sizing: content-box;
        }
        #border {
            width: 200px;
            padding: 10px;
            border-width: 5px;
            box-sizing: border-box;
        }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);
    let content_box = box_of(&boxes, &dom, "content");
    let border_box = box_of(&boxes, &dom, "border");

    // content-box: the declared 200px is the content width; padding and
    // border are added on top, growing the border box to 230px.
    assert_eq!(content_box.content().size().width(), au(200));
    assert_eq!(content_box.border_box().size().width(), au(230));

    // border-box: the declared 200px is the border-box width itself; padding
    // and border are taken out of it, leaving 170px of content.
    assert_eq!(border_box.content().size().width(), au(170));
    assert_eq!(border_box.border_box().size().width(), au(200));
}
