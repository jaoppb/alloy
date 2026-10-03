//! Guards the size of [`ComputedStyle`], the aggregate the cascade clones at
//! every step and stores once per [`StyledNode`] (PR #19 review finding #9,
//! schema 8).
//!
//! Schema 7 inlined the CSS Grid group — two 16-track lists, a 64-cell area
//! map and four named placements, ~2.9 KB — and the aggregate grew to ~3.7 KB
//! per node for data no layout reads yet. Schema 8 keeps the group behind a
//! shared `Arc` that stays empty while every grid property is `initial`. The
//! tests below pin both halves: the budget, and that the common grid-less node
//! carries no grid allocation at all.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use core::mem::size_of;

use css::domain::computed::grid::{GridGap, GridStyle, TrackList, TrackSize};
use css::{
    CascadeResolver, ComputedStyle, DomSnapshot, Length, StyledNode, StyledTree, UaCascade,
    collect_style_sheets, snapshot,
};

/// The most a [`ComputedStyle`] may occupy. Schema 8 measures a little under
/// 900 bytes; the budget leaves room for a few more small property groups and
/// fails long before a group the size of the grid one is inlined again.
const COMPUTED_STYLE_BUDGET: usize = 1024;

/// What a [`StyledNode`] adds on top of its style: tree links, the optional
/// text run and the intrinsic-size marker.
const STYLED_NODE_OVERHEAD_BUDGET: usize = 128;

fn non_initial_grid() -> GridStyle {
    let columns = TrackList::from_tracks(&[TrackSize::pixels(100.0), TrackSize::Auto]).unwrap();
    GridStyle::initial()
        .with_template_columns(columns)
        .with_gap(GridGap::uniform(Length::pixels(8.0)))
}

// ---- size budget ----------------------------------------------------------

#[test]
fn computed_style_stays_within_its_size_budget() {
    let size = size_of::<ComputedStyle>();
    assert!(
        size <= COMPUTED_STYLE_BUDGET,
        "ComputedStyle is {size} bytes; the budget is {COMPUTED_STYLE_BUDGET}"
    );
}

#[test]
fn styled_node_stays_within_the_style_budget_plus_its_links() {
    let size = size_of::<StyledNode>();
    let budget = COMPUTED_STYLE_BUDGET + STYLED_NODE_OVERHEAD_BUDGET;
    assert!(
        size <= budget,
        "StyledNode is {size} bytes; the budget is {budget}"
    );
}

// ---- grid storage ---------------------------------------------------------

#[test]
fn the_initial_grid_keeps_the_style_initial_and_unallocated() {
    let style = ComputedStyle::initial().with_grid(GridStyle::initial());
    assert_eq!(style, ComputedStyle::initial());
    assert_eq!(style.grid(), &GridStyle::initial());
    // Every grid-less style answers with the one shared initial value, never a
    // heap copy of it.
    assert!(core::ptr::eq(style.grid(), ComputedStyle::initial().grid()));
}

#[test]
fn a_non_initial_grid_round_trips() {
    let style = ComputedStyle::initial().with_grid(non_initial_grid());
    assert_eq!(style.grid(), &non_initial_grid());
    assert_ne!(style, ComputedStyle::initial());
    assert!(!core::ptr::eq(
        style.grid(),
        ComputedStyle::initial().grid()
    ));
    let reset = style.with_grid(GridStyle::initial());
    assert_eq!(reset, ComputedStyle::initial());
}

#[test]
fn a_styled_tree_can_cross_threads() {
    const fn assert_send_and_sync<T: Send + Sync>() {}
    assert_send_and_sync::<ComputedStyle>();
    assert_send_and_sync::<StyledTree>();
}

// ---- through the cascade --------------------------------------------------

/// `<html><body><p style="{inline}">Hi</p></body></html>`
fn resolve(inline: &str) -> (DomSnapshot, StyledTree) {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let html = child(&mut tree, root, "html");
    let body = child(&mut tree, html, "body");
    let paragraph = child(&mut tree, body, "p");
    tree.set_attribute(
        paragraph,
        dom::AttributeName::new("style").unwrap(),
        dom::AttributeValue::new(inline),
    )
    .unwrap();
    let dom = snapshot(&tree, root);
    let sheets = collect_style_sheets(&dom).expect("the document's CSS is readable");
    let styled = UaCascade::new()
        .resolve(&dom, &sheets)
        .expect("the cascade resolves");
    (dom, styled)
}

fn child(tree: &mut dom::DomTree, parent: dom::NodeId, name: &str) -> dom::NodeId {
    let node = tree.create_element(dom::TagName::new(name).unwrap());
    tree.append_child(parent, node).unwrap();
    node
}

fn style_of<'tree>(
    dom: &DomSnapshot,
    styled: &'tree StyledTree,
    tag: &dom::TagName,
) -> &'tree ComputedStyle {
    let id = dom
        .nodes_in_document_order()
        .find(|id| dom.node(*id).and_then(css::NodeRef::tag) == Some(tag))
        .expect("the document has the tag");
    styled.node(id).expect("the node is styled").style()
}

#[test]
fn the_cascade_stores_a_grid_only_where_one_is_declared() {
    let (dom, styled) = resolve("gap: 4px; grid-template-columns: 1fr 2fr");
    let paragraph = style_of(&dom, &styled, &dom::TagName::P);
    assert_eq!(paragraph.grid().row_gap(), Length::pixels(4.0));
    assert_eq!(paragraph.grid().template_columns().len(), 2);
    let body = style_of(&dom, &styled, &dom::TagName::new("body").unwrap());
    assert!(core::ptr::eq(body.grid(), ComputedStyle::initial().grid()));
}
