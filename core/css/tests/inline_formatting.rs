//! Rectangle proofs for the inline formatting context (v0.5 B4): overflow
//! forcing a line break, `text-align: justify`, and `white-space: pre`.
//!
//! Every fixture measures text through [`css::BlockLayout::new`]'s default
//! [`css::MonospaceMetrics`-backed] measurer: `0.6 * font-size` per glyph,
//! `1.2 * font-size` per line, both exact integer fractions of the `16px`
//! initial font size, so the expected `Au` raw units below are exact, not
//! approximate.

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

/// `0.6 * 16px` — one glyph's advance under the deterministic monospace
/// measurer, in `Au` raw units (`1024 * 3 / 5`).
const CHAR_ADVANCE: i32 = 614;
/// `1.2 * 16px` — one line's height, in `Au` raw units (`1024 * 6 / 5`).
const LINE_HEIGHT: i32 = 1228;

fn element(
    tree: &mut dom::DomTree,
    parent: dom::NodeId,
    id: &str,
    text_content: &str,
) -> dom::NodeId {
    let node = tree.create_element(dom::TagName::new("div").unwrap());
    tree.append_child(parent, node).unwrap();
    tree.set_attribute(
        node,
        dom::AttributeName::new("id").unwrap(),
        dom::AttributeValue::new(id),
    )
    .unwrap();
    let text = tree.create_text(dom::TextContent::new(text_content));
    tree.append_child(node, text).unwrap();
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

fn find(dom: &DomSnapshot, id: &str) -> SnapshotId {
    dom.nodes_in_document_order()
        .find(|&node| dom.node(node).and_then(|n| n.attribute("id")) == Some(id))
        .unwrap_or_else(|| panic!("no element with id=\"{id}\""))
}

/// The text node's own box — the fragment `inline.rs` emits for the run,
/// found through the `id`-bearing `div` that contains it.
fn text_box<'a>(boxes: &'a css::LayoutBoxTree, dom: &DomSnapshot, id: &str) -> &'a LayoutBox {
    let element_id = find(dom, id);
    let text_id = dom
        .node(element_id)
        .and_then(|node| node.children().next())
        .unwrap_or_else(|| panic!("id=\"{id}\" has no text child"));
    boxes
        .box_of(text_id)
        .unwrap_or_else(|| panic!("no box for the text under id=\"{id}\""))
}

// ---- overflow forces a line break -------------------------------------------

#[test]
fn a_container_narrower_than_the_text_wraps_to_a_second_line() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    element(&mut tree, root, "wide", "aaaa bbbb");
    element(&mut tree, root, "narrow", "aaaa bbbb");

    // "aaaa bbbb" unbroken is 9 glyphs wide: 9 * 614 = 5526 raw units (~86px).
    let source = "
        #wide { width: 100px; }
        #narrow { width: 47px; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);

    let wide = text_box(&boxes, &dom, "wide");
    let narrow = text_box(&boxes, &dom, "narrow");

    assert_eq!(
        wide.content().size().height(),
        Au::from_raw(LINE_HEIGHT),
        "100px comfortably fits \"aaaa bbbb\" on one line"
    );
    assert_eq!(
        narrow.content().size().height(),
        Au::from_raw(2 * LINE_HEIGHT),
        "47px fits \"aaaa\" but not \" bbbb\" too, forcing a second line"
    );
}

// ---- text-align: justify -----------------------------------------------------

#[test]
fn justify_widens_the_gaps_of_every_line_but_the_last() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    element(&mut tree, root, "left", "aaaa bbbb cccc");
    element(&mut tree, root, "justified", "aaaa bbbb cccc");

    // 120px = 7680 raw units. "aaaa bbbb" is 5526 raw units (fits); adding
    // " cccc" (3070 more) would reach 8596, which does not — so both wrap
    // after "bbbb", leaving "aaaa bbbb" (one gap) as the non-last line.
    let source = "
        #left { width: 120px; text-align: left; }
        #justified { width: 120px; text-align: justify; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);

    let left = text_box(&boxes, &dom, "left");
    let justified = text_box(&boxes, &dom, "justified");

    assert_eq!(
        left.content().size().width(),
        Au::from_raw(5526),
        "text-align: left leaves the first line at its natural width"
    );
    assert_eq!(
        justified.content().size().width(),
        au(120),
        "justify spreads the line's one gap until the line fills the 120px container exactly"
    );
}

// ---- white-space: pre --------------------------------------------------------

#[test]
fn white_space_pre_preserves_runs_of_space_and_line_breaks() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    element(&mut tree, root, "normal", "a  b\nc");
    element(&mut tree, root, "preserved", "a  b\nc");

    let source = "
        #normal { width: 400px; white-space: normal; }
        #preserved { width: 400px; white-space: pre; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);

    let normal = text_box(&boxes, &dom, "normal");
    let preserved = text_box(&boxes, &dom, "preserved");

    // `normal` collapses every run of white space (including the `\n`) to one
    // space each: "a b c" — one line, 5 glyphs wide (3 letters + 2 spaces).
    assert_eq!(normal.content().size().height(), Au::from_raw(LINE_HEIGHT));
    assert_eq!(
        normal.content().size().width(),
        Au::from_raw(5 * CHAR_ADVANCE)
    );

    // `pre` keeps both spaces of "a  b" as one 4-glyph line, and the `\n`
    // forces a second line holding just "c" — two lines, the widest being
    // "a  b" at 4 glyphs.
    assert_eq!(
        preserved.content().size().height(),
        Au::from_raw(2 * LINE_HEIGHT)
    );
    assert_eq!(
        preserved.content().size().width(),
        Au::from_raw(4 * CHAR_ADVANCE)
    );
}
