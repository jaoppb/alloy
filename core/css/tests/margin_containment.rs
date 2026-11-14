//! Margin collapsing around runs that hold no line box, and the boxes that
//! contain their children's margins (CSS 2.1 §8.3.1, §9.4.1; #135 of #81).
//!
//! A whitespace-only run between blocks, like an empty `<span>`, generates no
//! line box, so margins collapse through it. A box that starts a new block
//! formatting context — an `inline-block`, a flex item, a scroll container,
//! the root element — never collapses its own margins with its children's.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use css::{
    BlockLayout, CascadeResolver, DomSnapshot, LayoutBoxTree, LayoutEngine, SnapshotId, UaCascade,
    ViewportConstraints, collect_style_sheets, snapshot,
};
use graphics::{Au, Rect};

struct Laid {
    dom: DomSnapshot,
    boxes: LayoutBoxTree,
}

impl Laid {
    /// `body` at an 800px viewport, with no UA margins on `body` or `p`.
    fn new(body: &str) -> Self {
        let markup = format!(
            "<!doctype html><html><head><style>body, p {{ margin: 0 }}</style></head>\
             <body>{body}</body></html>"
        );
        let tree = dom::parse(&markup).expect("the fixture parses").into_tree();
        let dom = snapshot(&tree, tree.document());
        let sheets = collect_style_sheets(&dom).expect("the fixture's CSS is readable");
        let styled = UaCascade::new()
            .resolve(&dom, &sheets)
            .expect("the cascade resolves");
        let viewport = ViewportConstraints::new(
            Au::from_whole_px(800).unwrap(),
            Au::from_whole_px(600).unwrap(),
        );
        let boxes = BlockLayout::monospace()
            .layout(&styled, &viewport)
            .expect("layout succeeds");
        Self { dom, boxes }
    }

    fn element(&self, id: &str) -> SnapshotId {
        self.dom
            .nodes_in_document_order()
            .find(|&node| self.dom.node(node).and_then(|n| n.attribute("id")) == Some(id))
            .unwrap_or_else(|| panic!("no element with id=\"{id}\""))
    }

    fn border_box(&self, id: &str) -> Rect {
        self.boxes
            .box_of(self.element(id))
            .unwrap_or_else(|| panic!("#{id} has no box"))
            .border_box()
    }
}

const fn px(pixels: i32) -> Au {
    Au::from_whole_px(pixels).unwrap()
}

#[test]
fn whitespace_between_two_blocks_does_not_stop_their_margins_collapsing() {
    let laid = Laid::new(
        "<div id=a style='height:10px;margin-bottom:30px'></div>\n  \n\
         <div id=b style='height:10px;margin-top:20px'></div>",
    );

    assert_eq!(
        laid.border_box("b").min_y(),
        laid.border_box("a").max_y().saturating_add(px(30))
    );
}

#[test]
fn a_first_blocks_margin_escapes_its_parent_past_leading_whitespace() {
    let laid = Laid::new(
        "<div id=parent>\n  <div id=child style='height:10px;margin-top:40px'></div>\n</div>",
    );

    assert_eq!(
        laid.border_box("parent").min_y(),
        laid.border_box("child").min_y()
    );
}

#[test]
fn an_empty_block_holding_only_whitespace_collapses_through() {
    let laid = Laid::new(
        "<div id=a style='height:10px;margin-bottom:30px'></div>\
         <div style='margin:20px 0'> </div>\
         <div id=b style='height:10px;margin-top:10px'></div>",
    );

    assert_eq!(
        laid.border_box("b").min_y(),
        laid.border_box("a").max_y().saturating_add(px(30)),
        "30, 20, 20 and 10 all adjoin: the largest, 30, wins"
    );
}

#[test]
fn an_inline_block_keeps_its_childs_margins_inside() {
    let laid = Laid::new(
        "<span id=ib style='display:inline-block'>\n  <p id=p style='margin:40px 0'>x</p>\n</span>",
    );
    let inline_block = laid.border_box("ib");
    let child = laid.border_box("p");

    assert_eq!(child.min_y(), inline_block.min_y().saturating_add(px(40)));
    assert_eq!(inline_block.max_y(), child.max_y().saturating_add(px(40)));
}

#[test]
fn a_flex_item_keeps_its_childs_margins_inside() {
    let laid = Laid::new(
        "<div style='display:flex'><div id=item>\n  <p id=p style='margin:40px 0'>x</p>\n</div></div>\
         <div id=after style='height:10px'></div>",
    );
    let item = laid.border_box("item");
    let child = laid.border_box("p");

    assert_eq!(child.min_y(), item.min_y().saturating_add(px(40)));
    assert_eq!(item.max_y(), child.max_y().saturating_add(px(40)));
    assert_eq!(laid.border_box("after").min_y(), item.max_y());
}

#[test]
fn a_scroll_container_keeps_its_childs_margins_inside() {
    let laid =
        Laid::new("<div id=box style='overflow:hidden'><p id=p style='margin:40px 0'>x</p></div>");
    let container = laid.border_box("box");

    assert_eq!(
        laid.border_box("p").min_y(),
        container.min_y().saturating_add(px(40))
    );
}

#[test]
fn the_root_element_keeps_its_childs_margins_inside() {
    let laid = Laid::new("<div id=first style='height:10px;margin-top:40px'></div>");
    let html = laid
        .boxes
        .box_of(
            laid.dom
                .nodes_in_document_order()
                .find(|&node| laid.dom.node(node).and_then(css::NodeRef::tag_str) == Some("html"))
                .unwrap(),
        )
        .unwrap()
        .border_box();

    assert_eq!(
        html.min_y(),
        Au::ZERO,
        "the root box sits at the top of the canvas"
    );
    assert_eq!(laid.border_box("first").min_y(), px(40));
}

#[test]
fn bodys_overflow_belongs_to_the_viewport_so_its_margins_still_collapse() {
    let markup = "<!doctype html><html><head><style>body { margin: 8px; overflow-x: hidden }</style></head>\
                  <body id=body><h1 id=h1>x</h1></body></html>";
    let tree = dom::parse(markup).expect("the fixture parses").into_tree();
    let dom = snapshot(&tree, tree.document());
    let sheets = collect_style_sheets(&dom).expect("readable");
    let styled = UaCascade::new().resolve(&dom, &sheets).expect("resolves");
    let viewport = ViewportConstraints::new(px(800), px(600));
    let boxes = BlockLayout::monospace()
        .layout(&styled, &viewport)
        .expect("lays out");
    let laid = Laid { dom, boxes };

    assert_eq!(
        laid.border_box("body").min_y(),
        laid.border_box("h1").min_y(),
        "body's overflow propagates to the viewport (CSS Overflow L3 §3.3): body is not a \
         scroll container, so h1's top margin collapses with body's"
    );
}

#[test]
fn an_empty_scroll_container_does_not_collapse_its_own_margins_through() {
    let laid = Laid::new(
        "<div id=a style='height:10px'></div>\
         <div style='overflow:hidden;margin:20px 0'></div>\
         <div id=b style='height:10px'></div>",
    );

    assert_eq!(
        laid.border_box("b").min_y(),
        laid.border_box("a").max_y().saturating_add(px(40)),
        "a box that starts a formatting context keeps its top and bottom margins apart"
    );
}

#[test]
fn a_child_inheriting_bodys_overflow_is_a_scroll_container() {
    let markup = "<!doctype html><html><head><style>body { margin: 0; overflow: hidden } \
                  #c { overflow: inherit } p { margin: 30px 0 }</style></head>\
                  <body><div id=c><p id=a>x</p></div></body></html>";
    let tree = dom::parse(markup).expect("the fixture parses").into_tree();
    let dom = snapshot(&tree, tree.document());
    let sheets = collect_style_sheets(&dom).expect("readable");
    let styled = UaCascade::new().resolve(&dom, &sheets).expect("resolves");
    let viewport = ViewportConstraints::new(px(800), px(600));
    let boxes = BlockLayout::monospace()
        .layout(&styled, &viewport)
        .expect("lays out");
    let laid = Laid { dom, boxes };
    let container = laid.border_box("c");

    assert_eq!(
        laid.border_box("a").min_y(),
        container.min_y().saturating_add(px(30)),
        "body's computed overflow stays `hidden` — only its used value goes to the viewport — \
         so #c inherits `hidden` and keeps a's margin inside"
    );
}

#[test]
fn body_keeps_its_overflow_when_the_root_has_its_own() {
    let markup = "<!doctype html><html><head><style>html { overflow: hidden } \
                  body { margin: 8px; overflow: hidden } h1 { margin: 20px 0 }</style></head>\
                  <body id=body><h1 id=h1>x</h1></body></html>";
    let tree = dom::parse(markup).expect("the fixture parses").into_tree();
    let dom = snapshot(&tree, tree.document());
    let sheets = collect_style_sheets(&dom).expect("readable");
    let styled = UaCascade::new().resolve(&dom, &sheets).expect("resolves");
    let viewport = ViewportConstraints::new(px(800), px(600));
    let boxes = BlockLayout::monospace()
        .layout(&styled, &viewport)
        .expect("lays out");
    let laid = Laid { dom, boxes };

    assert_eq!(
        laid.border_box("h1").min_y(),
        laid.border_box("body").min_y().saturating_add(px(20)),
        "the root's own overflow is not visible, so body's stays on body (CSS Overflow L3 §3.3): \
         body is a scroll container and keeps h1's margin inside"
    );
}
