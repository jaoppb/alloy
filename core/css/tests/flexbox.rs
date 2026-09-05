//! Rectangle proofs for Flexbox (v0.5 B4, CSS Flexbox L1 §9): one proof per
//! property named in the phase's `Definition of Done`. Every fixture uses
//! `div` elements (the UA sheet leaves them at `ComputedStyle::initial()`)
//! sized with plain pixel lengths, so the expected positions below are exact
//! integer arithmetic, never text measurement.

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

// ---- flex-direction ----------------------------------------------------------

#[test]
fn flex_direction_row_spreads_items_along_x_and_column_along_y() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let row = element(&mut tree, root, "row");
    element(&mut tree, row, "row-a");
    element(&mut tree, row, "row-b");
    let column = element(&mut tree, root, "column");
    element(&mut tree, column, "column-a");
    element(&mut tree, column, "column-b");

    let source = "
        #row { display: flex; flex-direction: row; width: 300px; }
        #row-a, #row-b { width: 50px; height: 20px; }
        #column { display: flex; flex-direction: column; width: 300px; }
        #column-a, #column-b { width: 50px; height: 20px; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);

    let row_a = box_of(&boxes, &dom, "row-a").content();
    let row_b = box_of(&boxes, &dom, "row-b").content();
    assert_eq!(
        row_a.min_y(),
        row_b.min_y(),
        "row keeps both items on the same cross line"
    );
    assert!(row_a.min_x() < row_b.min_x(), "row spreads items along x");

    let column_a = box_of(&boxes, &dom, "column-a").content();
    let column_b = box_of(&boxes, &dom, "column-b").content();
    assert_eq!(
        column_a.min_x(),
        column_b.min_x(),
        "column keeps both items at the same cross start"
    );
    assert!(
        column_a.min_y() < column_b.min_y(),
        "column spreads items along y"
    );
}

// ---- flex-wrap -----------------------------------------------------------------

#[test]
fn flex_wrap_breaks_a_line_that_nowrap_would_overflow() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let single = element(&mut tree, root, "single");
    element(&mut tree, single, "single-a");
    element(&mut tree, single, "single-b");
    let wrapped = element(&mut tree, root, "wrapped");
    element(&mut tree, wrapped, "wrapped-a");
    element(&mut tree, wrapped, "wrapped-b");

    let source = "
        #single { display: flex; flex-wrap: nowrap; width: 100px; }
        #single-a, #single-b { width: 60px; height: 20px; }
        #wrapped { display: flex; flex-wrap: wrap; width: 100px; }
        #wrapped-a, #wrapped-b { width: 60px; height: 20px; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);

    let single_a = box_of(&boxes, &dom, "single-a").content();
    let single_b = box_of(&boxes, &dom, "single-b").content();
    assert_eq!(
        single_a.min_y(),
        single_b.min_y(),
        "nowrap keeps both 60px items on one line even though 120px overflows the 100px container"
    );

    let wrapped_a = box_of(&boxes, &dom, "wrapped-a").content();
    let wrapped_b = box_of(&boxes, &dom, "wrapped-b").content();
    assert!(
        wrapped_b.min_y() > wrapped_a.min_y(),
        "wrap breaks the second 60px item onto a new line"
    );
    assert_eq!(
        wrapped_a.min_x(),
        wrapped_b.min_x(),
        "each wrapped line starts again from the main-start"
    );
}

// ---- justify-content -------------------------------------------------------

#[test]
fn justify_content_distributes_free_main_axis_space_six_ways() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    for id in ["start", "end", "center", "between", "around", "evenly"] {
        let container = element(&mut tree, root, id);
        element(&mut tree, container, &format!("{id}-a"));
        element(&mut tree, container, &format!("{id}-b"));
    }

    // Two 50px items in a 300px row leave 200px of free space.
    let source = "
        #start, #end, #center, #between, #around, #evenly { display: flex; width: 300px; }
        #start-a, #start-b, #end-a, #end-b, #center-a, #center-b,
        #between-a, #between-b, #around-a, #around-b, #evenly-a, #evenly-b { width: 50px; }
        #start { justify-content: flex-start; }
        #end { justify-content: flex-end; }
        #center { justify-content: center; }
        #between { justify-content: space-between; }
        #around { justify-content: space-around; }
        #evenly { justify-content: space-evenly; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);
    let min_x = |id: &str| box_of(&boxes, &dom, id).content().min_x();

    assert_eq!(
        (min_x("start-a"), min_x("start-b")),
        (au(0), au(50)),
        "flex-start packs at the start"
    );
    assert_eq!(
        (min_x("end-a"), min_x("end-b")),
        (au(200), au(250)),
        "flex-end packs all 200px before the items"
    );
    assert_eq!(
        (min_x("center-a"), min_x("center-b")),
        (au(100), au(150)),
        "center splits the 200px evenly"
    );
    assert_eq!(
        (min_x("between-a"), min_x("between-b")),
        (au(0), au(250)),
        "space-between puts the whole 200px gap between the two items"
    );
    assert_eq!(
        (min_x("around-a"), min_x("around-b")),
        (au(50), au(200)),
        "space-around gives the two ends a half share (50px) and the one gap a full share (100px)"
    );
    // 200px = 12800 raw `Au` units split into three shares of 12800/3: the
    // distributor works in raw units, not whole pixels, so the shares are
    // 4267, 4267 and 4266 (remainder handed out first, CSS Flexbox L1 §9.4).
    assert_eq!(
        (min_x("evenly-a"), min_x("evenly-b")),
        (Au::from_raw(4267), Au::from_raw(4267 + 3200 + 4267)),
        "space-evenly splits 200px into three equal ~66.67px shares"
    );
}

// ---- align-items --------------------------------------------------------------

#[test]
fn align_items_positions_a_shorter_item_on_the_cross_axis() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    for id in ["stretch", "start", "center"] {
        let container = element(&mut tree, root, id);
        element(&mut tree, container, &format!("{id}-item"));
    }

    let source = "
        #stretch, #start, #center { display: flex; width: 100px; height: 100px; }
        #start-item, #center-item { width: 50px; height: 20px; }
        #stretch-item { width: 50px; }
        #stretch { align-items: stretch; }
        #start { align-items: flex-start; }
        #center { align-items: center; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);

    // The three containers are themselves ordinary block siblings stacked
    // vertically, so every item's absolute `min_y` carries its own
    // container's top offset — measure relative to that, not to the page.
    let relative_top = |container: &str, item: &str| {
        let container_top = box_of(&boxes, &dom, container).content().min_y();
        let item_top = box_of(&boxes, &dom, item).content().min_y();
        item_top
            .checked_sub(container_top)
            .expect("item sits at or below its container's top")
    };

    // `align-items: stretch` only stretches an item whose own cross-size
    // property (`height`, on a row) is `auto` (CSS Flexbox L1 §8.3) — hence
    // no `height` on `#stretch-item`, unlike its siblings below.
    assert_eq!(relative_top("stretch", "stretch-item"), au(0));
    assert_eq!(
        box_of(&boxes, &dom, "stretch-item")
            .content()
            .size()
            .height(),
        au(100),
        "stretch fills the 100px cross size"
    );

    assert_eq!(relative_top("start", "start-item"), au(0));
    assert_eq!(
        box_of(&boxes, &dom, "start-item").content().size().height(),
        au(20),
        "flex-start keeps the item's own 20px height"
    );

    assert_eq!(
        relative_top("center", "center-item"),
        au(40),
        "center splits the 80px of leftover cross space evenly"
    );
}

// ---- align-self -----------------------------------------------------------

#[test]
fn align_self_overrides_align_items_for_one_item() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "container");
    element(&mut tree, container, "plain");
    element(&mut tree, container, "overridden");

    let source = "
        #container { display: flex; align-items: flex-start; width: 100px; height: 100px; }
        #plain, #overridden { width: 50px; height: 20px; }
        #overridden { align-self: center; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);

    assert_eq!(
        box_of(&boxes, &dom, "plain").content().min_y(),
        au(0),
        "no align-self defers to the container"
    );
    assert_eq!(
        box_of(&boxes, &dom, "overridden").content().min_y(),
        au(40),
        "align-self: center overrides the container's flex-start for this one item"
    );
}

// ---- align-content --------------------------------------------------------

#[test]
fn align_content_distributes_leftover_cross_space_across_lines() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "container");
    element(&mut tree, container, "line-one");
    element(&mut tree, container, "line-two");

    // width: 60px + wrap forces the two 60px items onto two lines, each 20px
    // tall; height: 200px leaves 160px of leftover cross space across them.
    let source = "
        #container {
            display: flex;
            flex-wrap: wrap;
            align-content: center;
            width: 60px;
            height: 200px;
        }
        #line-one, #line-two { width: 60px; height: 20px; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);

    let line_one = box_of(&boxes, &dom, "line-one").content();
    let line_two = box_of(&boxes, &dom, "line-two").content();

    assert_eq!(
        line_one.min_y(),
        au(80),
        "center pushes the two 20px lines down by half of the 160px leftover"
    );
    assert_eq!(
        line_two.min_y(),
        au(100),
        "the second line starts right after the first, no extra gap for center"
    );
}

// ---- flex-grow / flex-shrink ------------------------------------------------

#[test]
fn flex_grow_distributes_free_space_proportionally() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "container");
    element(&mut tree, container, "a");
    element(&mut tree, container, "b");

    // Both items start at a 0px basis; the 300px of free space splits 1:3.
    let source = "
        #container { display: flex; width: 300px; }
        #a, #b { flex-basis: 0px; }
        #a { flex-grow: 1; }
        #b { flex-grow: 3; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);

    let a_width = box_of(&boxes, &dom, "a").content().size().width();
    let b_width = box_of(&boxes, &dom, "b").content().size().width();

    assert_eq!(a_width, au(75));
    assert_eq!(b_width, au(225));
}

#[test]
fn flex_shrink_takes_more_from_a_higher_factor() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let container = element(&mut tree, root, "container");
    element(&mut tree, container, "a");
    element(&mut tree, container, "b");

    // Two 80px items overflow a 100px container by 60px; shrink weights
    // (factor * basis) are 80 and 240 — a 1:3 ratio, same as the shrink.
    let source = "
        #container { display: flex; width: 100px; }
        #a, #b { flex-basis: 80px; }
        #a { flex-shrink: 1; }
        #b { flex-shrink: 3; }
    ";
    let dom = snapshot(&tree, root);
    let boxes = layout_boxes(&tree, root, source);

    let a_width = box_of(&boxes, &dom, "a").content().size().width();
    let b_width = box_of(&boxes, &dom, "b").content().size().width();

    assert_eq!(a_width, au(65), "shrinks by 15px");
    assert_eq!(b_width, au(35), "shrinks by 45px — three times as much");
}
