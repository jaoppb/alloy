//! Rectangle proofs for `display: inline-block` (CSS 2.1 §9.2.4, §10.3.9):
//! an atomic inline-level box sits on a line next to its siblings instead of
//! starting a block of its own, is as wide as its `width` or — when that is
//! `auto` — as its contents (shrink-to-fit), and paints the `<input>` label
//! the styled tree synthesizes inside its own box (issues #2 / #3).
//!
//! Text is measured by the deterministic monospace measurer: `0.6 * 16px` per
//! glyph and `1.2 * 16px` per line, exact in `Au` raw units.

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

/// One glyph's advance under the monospace measurer, in `Au` raw units.
const CHAR_ADVANCE: i32 = 614;
/// One line's height under the monospace measurer, in `Au` raw units.
const LINE_HEIGHT: i32 = 1228;

const fn glyphs(count: i32) -> Au {
    Au::from_raw(CHAR_ADVANCE.saturating_mul(count))
}

fn element(tree: &mut dom::DomTree, parent: dom::NodeId, tag: &str, id: &str) -> dom::NodeId {
    let node = tree.create_element(dom::TagName::new(tag).unwrap());
    tree.append_child(parent, node).unwrap();
    attribute(tree, node, "id", id);
    node
}

fn attribute(tree: &mut dom::DomTree, node: dom::NodeId, name: &str, value: &str) {
    tree.set_attribute(
        node,
        dom::AttributeName::new(name).unwrap(),
        dom::AttributeValue::new(value),
    )
    .unwrap();
}

fn text(tree: &mut dom::DomTree, parent: dom::NodeId, content: &str) {
    let node = tree.create_text(dom::TextContent::new(content));
    tree.append_child(parent, node).unwrap();
}

fn layout_boxes(tree: &dom::DomTree, root: dom::NodeId, source: &str) -> css::LayoutBoxTree {
    let dom = snapshot(tree, root);
    let sheet = parse_stylesheet(source, Origin::Author).expect("author CSS parses");
    let styled = UaCascade::new()
        .resolve(&dom, &sheet)
        .expect("cascade resolves");
    BlockLayout::monospace()
        .layout(&styled, &ViewportConstraints::new(au(800), au(600)))
        .expect("layout succeeds")
}

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

// ---- atomic inlines share a line ---------------------------------------------

#[test]
fn two_submit_inputs_sit_side_by_side_on_one_line() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "div", "container");
    let first = element(&mut tree, container, "input", "first");
    attribute(&mut tree, first, "type", "submit");
    let second = element(&mut tree, container, "input", "second");
    attribute(&mut tree, second, "type", "submit");

    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, "");
    let first_box = box_of(&boxes, &dom, "first").border_box();
    let second_box = box_of(&boxes, &dom, "second").border_box();

    assert_eq!(
        first_box.min_y(),
        second_box.min_y(),
        "both inline-block inputs share one line box"
    );
    assert!(
        second_box.min_x() > first_box.min_x(),
        "the second input follows the first on the line"
    );
    assert_eq!(
        second_box.min_x(),
        first_box.max_x(),
        "nothing separates two adjacent atomic inlines"
    );
    assert_eq!(
        first_box.size().width(),
        glyphs(12),
        "shrink-to-fit: the input is exactly as wide as `Submit Query`"
    );
}

#[test]
fn an_inline_block_with_an_explicit_width_gets_that_content_box_after_preceding_text() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "div", "container");
    text(&mut tree, container, "ab");
    let atom = element(&mut tree, container, "div", "atom");
    text(&mut tree, atom, "x");

    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(
        &tree,
        root,
        "#atom { display: inline-block; width: 100px; }",
    );
    let atom_box = box_of(&boxes, &dom, "atom");

    assert_eq!(atom_box.content().size().width(), au(100));
    assert_eq!(
        atom_box.content().min_x(),
        glyphs(2),
        "the atom follows the two glyphs of text on the same line"
    );
    assert_eq!(atom_box.content().min_y(), Au::ZERO);
    assert_eq!(
        atom_box.content().size().height(),
        Au::from_raw(LINE_HEIGHT)
    );
}

#[test]
fn a_centred_inline_block_still_shrinks_to_its_text() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "div", "container");
    let atom = element(&mut tree, container, "div", "atom");
    text(&mut tree, atom, "abc");

    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(
        &tree,
        root,
        "#atom { display: inline-block; text-align: center; }",
    );
    let atom_box = box_of(&boxes, &dom, "atom");
    let text_id = dom
        .node(find(&dom, "atom"))
        .unwrap()
        .children()
        .next()
        .unwrap();
    let text_box = boxes.box_of(text_id).expect("the text has a box");

    assert_eq!(atom_box.content().size().width(), glyphs(3));
    assert_eq!(
        text_box.content().min_x(),
        atom_box.content().min_x(),
        "at the fitted width centring leaves no free space"
    );
}

// ---- the synthesized <input> label ------------------------------------------

#[test]
fn the_submit_label_is_laid_out_inside_the_inline_block_input() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "div", "container");
    let input = element(&mut tree, container, "input", "go");
    attribute(&mut tree, input, "type", "submit");
    attribute(&mut tree, input, "value", "Go");

    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, "#go { padding: 4px; }");
    let input_id = find(&dom, "go");
    let input_boxes: Vec<&LayoutBox> = boxes
        .boxes_in_document_order()
        .filter(|laid_out| laid_out.node() == input_id)
        .collect();

    assert_eq!(
        input_boxes.len(),
        2,
        "the input's own box, then its label's line box"
    );
    let own = input_boxes[0];
    let label = input_boxes[1];
    assert_eq!(own.content().size().width(), glyphs(2));
    assert_eq!(
        own.border_box().size().width(),
        glyphs(2).saturating_add(au(8))
    );
    assert_eq!(
        label.content(),
        own.content(),
        "the label fills exactly the input's content box"
    );
}

// ---- display: none inside an inline ---------------------------------------------

#[test]
fn a_display_none_descendant_of_an_inline_takes_no_room_on_the_line() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "div", "container");
    let outer = element(&mut tree, container, "span", "outer");
    text(&mut tree, outer, "ab");
    let hidden = element(&mut tree, outer, "span", "hidden");
    text(&mut tree, hidden, "hidden");
    text(&mut tree, outer, "cd");

    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, "#hidden { display: none; }");

    assert_eq!(
        box_of(&boxes, &dom, "outer").content().size().width(),
        glyphs(4),
        "only `ab` and `cd` are on the line"
    );
    assert!(
        boxes.box_of(find(&dom, "hidden")).is_none(),
        "a display: none element generates no box"
    );
}

// ---- cost ---------------------------------------------------------------------

/// Shrink-to-fit measures before it lays out; without remembering the
/// measurement, `n` nested inline-blocks would cost `2^n` layouts and this test
/// would never finish. Each level puts a glyph before the next level, so an
/// inner atom is offered more room in its parent's final pass than it needs —
/// the case where a second, narrower layout would otherwise be redone.
#[test]
fn deeply_nested_inline_blocks_shrink_without_exponential_relayout() {
    const LEVELS: i32 = 64;
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "div", "container");
    let mut parent = container;
    for level in 0..LEVELS {
        parent = element(&mut tree, parent, "span", &format!("level{level}"));
        attribute(&mut tree, parent, "class", "atom");
        text(&mut tree, parent, "x");
    }

    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, ".atom { display: inline-block; }");

    assert_eq!(
        box_of(&boxes, &dom, "level0").content().size().width(),
        glyphs(LEVELS),
        "the outermost atom shrinks to the one glyph of every level"
    );
    assert_eq!(
        box_of(&boxes, &dom, "level63").content().size().width(),
        glyphs(1)
    );
}
