//! Block-in-inline (CSS 2.1 §9.2.1.1, #135 of #81): a block-level box inside
//! an inline box splits the inline run. The content before it becomes one
//! anonymous block of lines, the block is stacked like a sibling, and the
//! content after it becomes a second anonymous block; the inline ancestor
//! gets fragments on each side, never one across the block.
//!
//! Text is measured with [`css::BlockLayout::monospace`]: `0.6 * 16px` per
//! glyph and `1.2 * 16px` per line, so the values below are exact.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use css::{
    BlockLayout, CascadeResolver, DomSnapshot, LayoutBox, LayoutBoxTree, LayoutEngine, SnapshotId,
    UaCascade, ViewportConstraints, collect_style_sheets, snapshot,
};
use graphics::Au;

/// `1.2 * 16px` — one line's height, in `Au` raw units.
const LINE_HEIGHT: i32 = 1228;

struct Laid {
    dom: DomSnapshot,
    boxes: LayoutBoxTree,
}

impl Laid {
    /// `body` inside a 400px-wide `div` at an 800px viewport, no UA margins.
    fn new(body: &str) -> Self {
        let markup = format!(
            "<!doctype html><html><head><style>body {{ margin: 0 }} div {{ margin: 0 }}</style></head>\
             <body><div id=container style='width:400px'>{body}</div></body></html>"
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

    /// The fragments of the text node whose text is exactly `text`.
    fn text(&self, text: &str) -> Vec<&LayoutBox> {
        let node = self
            .dom
            .nodes_in_document_order()
            .find(|&node| self.dom.node(node).and_then(css::NodeRef::text) == Some(text))
            .unwrap_or_else(|| panic!("no text node \"{text}\""));
        self.boxes.fragments_of(node).collect()
    }

    fn only_text_line(&self, text: &str) -> &LayoutBox {
        let fragments = self.text(text);
        assert_eq!(fragments.len(), 1, "\"{text}\" fits on one line");
        fragments[0]
    }

    fn fragments(&self, id: &str) -> Vec<&LayoutBox> {
        self.boxes.fragments_of(self.element(id)).collect()
    }
}

const fn line() -> Au {
    Au::from_raw(LINE_HEIGHT)
}

#[test]
fn a_block_inside_an_inline_box_splits_the_run_into_three_rows() {
    let laid = Laid::new("<span id=s>aa<div id=d>bb</div>cc</span>");
    let before = laid.only_text_line("aa").content();
    let inside = laid.only_text_line("bb").content();
    let after = laid.only_text_line("cc").content();

    assert_eq!(before.min_y(), Au::ZERO);
    assert_eq!(
        inside.min_y(),
        line(),
        "the block starts on the row after \"aa\""
    );
    assert_eq!(
        after.min_y(),
        line().saturating_add(line()),
        "\"cc\" follows the block"
    );

    let block = laid.fragments("d");
    assert_eq!(block.len(), 1);
    assert_eq!(
        block[0].content().size().width(),
        Au::from_whole_px(400).unwrap(),
        "the block spans the containing block, not the inline box"
    );

    let span = laid.fragments("s");
    assert_eq!(span.len(), 2, "one fragment on each side of the split");
    assert_eq!(span[0].content().min_y(), before.min_y());
    assert_eq!(span[1].content().min_y(), after.min_y());
}

#[test]
fn the_split_reaches_through_nested_inline_boxes() {
    let laid = Laid::new("<a id=a href=x><span id=s>aa<div id=d>bb</div>cc</span></a>");

    assert_eq!(laid.only_text_line("bb").content().min_y(), line());
    assert_eq!(
        laid.only_text_line("cc").content().min_y(),
        line().saturating_add(line())
    );
    assert_eq!(laid.fragments("a").len(), 2);
    assert_eq!(laid.fragments("s").len(), 2);
}

#[test]
fn two_blocks_inside_one_inline_box_make_five_rows() {
    let laid = Laid::new("<span id=s>aa<div>bb</div>cc<div>dd</div>ee</span>");
    let rows: Vec<Au> = ["aa", "bb", "cc", "dd", "ee"]
        .iter()
        .map(|text| laid.only_text_line(text).content().min_y())
        .collect();

    let expected: Vec<Au> = (0..5)
        .map(|row| Au::from_raw(LINE_HEIGHT.saturating_mul(row)))
        .collect();
    assert_eq!(rows, expected);
    assert_eq!(
        laid.fragments("s").len(),
        3,
        "before, between and after the blocks"
    );
}

#[test]
fn inline_content_after_the_split_inline_box_continues_its_last_line() {
    let laid = Laid::new("<span>aa<div>bb</div>cc</span>dd");
    let after = laid.only_text_line("cc").content();
    let next = laid.only_text_line("dd").content();

    assert_eq!(
        next.min_y(),
        after.min_y(),
        "\"dd\" shares the row \"cc\" is on"
    );
    assert_eq!(next.min_x(), after.max_x());
}

#[test]
fn an_inline_box_holding_only_a_block_gets_no_line_fragment() {
    let laid = Laid::new("<span id=s><div id=d>bb</div></span>");

    assert_eq!(laid.only_text_line("bb").content().min_y(), Au::ZERO);
    assert!(
        laid.fragments("s").is_empty(),
        "no content before or after the block, so no line holds any of the span"
    );
}

#[test]
fn a_split_block_collapses_margins_with_its_sibling_through_the_empty_runs() {
    let laid = Laid::new(
        "<div id=first style='height:10px;margin-bottom:30px'></div>\
         <span><div id=d style='height:10px;margin-top:30px'></div></span>",
    );
    let first = laid.fragments("first")[0].content();
    let split = laid.fragments("d")[0].content();

    assert_eq!(
        split.min_y().saturating_sub(first.max_y()),
        Au::from_whole_px(30).unwrap(),
        "30px and 30px collapse to 30px (CSS 2.1 §8.3.1), as for siblings"
    );
}

#[test]
fn a_split_blocks_top_margin_escapes_its_parent_through_an_empty_run() {
    let laid = Laid::new(
        "<div id=parent><span><div id=d style='height:10px;margin-top:40px'></div></span></div>",
    );
    let parent = laid.fragments("parent")[0].content();
    let split = laid.fragments("d")[0].content();

    assert_eq!(
        split.min_y(),
        parent.min_y(),
        "the parent starts where its first in-flow child does: the margin escapes"
    );
}

#[test]
fn a_block_inside_an_inline_block_or_hidden_does_not_split_the_outer_run() {
    let laid = Laid::new(
        "<span>aa<span style='display:inline-block;width:40px'><div>bb</div></span>cc\
         <div style='display:none'>xx</div>dd</span>",
    );
    let row = laid.only_text_line("aa").content().min_y();

    assert_eq!(laid.only_text_line("cc").content().min_y(), row);
    assert_eq!(laid.only_text_line("dd").content().min_y(), row);
}
