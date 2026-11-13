//! One fragment per line for inline boxes (#134, part of #81): an inline
//! element or a text node that wraps onto several lines gets one rectangle per
//! line it touches, never one rectangle spanning them, and a text fragment
//! carries exactly the text layout placed on its line — which is what paint
//! draws.
//!
//! Every fixture measures text with [`css::BlockLayout::monospace`]: `0.6 *
//! font-size` per glyph and `1.2 * font-size` per line, so the `16px` initial
//! size gives exact `Au` values.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use css::{
    BlockLayout, CascadeResolver, DomSnapshot, LayoutBox, LayoutBoxTree, LayoutEngine, SnapshotId,
    UaCascade, ViewportConstraints, collect_style_sheets, snapshot,
};
use graphics::Au;

/// `0.6 * 16px` — one glyph's advance, in `Au` raw units.
const CHAR_ADVANCE: i32 = 614;
/// `1.2 * 16px` — one line's height, in `Au` raw units.
const LINE_HEIGHT: i32 = 1228;

struct Laid {
    dom: DomSnapshot,
    boxes: LayoutBoxTree,
}

impl Laid {
    /// `body` laid out at an 800px viewport with no UA body margin.
    fn new(body: &str) -> Self {
        let markup = format!(
            "<!doctype html><html><head><style>body {{ margin: 0 }}</style></head><body>{body}</body></html>"
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

    /// The first child of `#id`, which every fixture makes a text node.
    fn text_of(&self, id: &str) -> SnapshotId {
        let element = self.element(id);
        self.dom
            .node(element)
            .and_then(|node| node.children().next())
            .unwrap_or_else(|| panic!("id=\"{id}\" has no child"))
    }

    fn fragments(&self, node: SnapshotId) -> Vec<&LayoutBox> {
        self.boxes.fragments_of(node).collect()
    }
}

fn texts(fragments: &[&LayoutBox]) -> Vec<String> {
    fragments
        .iter()
        .map(|fragment| {
            fragment
                .text()
                .map(|text| text.as_str().to_owned())
                .unwrap_or_default()
        })
        .collect()
}

#[test]
fn an_inline_element_that_wraps_gets_one_fragment_per_line() {
    // 47px fits "aaaa" (4 glyphs, ~38px) but not "aaaa bbbb" (9 glyphs).
    let laid = Laid::new("<div style='width:47px'><span id=s>aaaa bbbb</span></div>");
    let fragments = laid.fragments(laid.element("s"));

    assert_eq!(fragments.len(), 2, "one fragment per line");
    let first = fragments[0].content();
    let second = fragments[1].content();
    assert_eq!(first.size().height(), Au::from_raw(LINE_HEIGHT));
    assert_eq!(second.size().height(), Au::from_raw(LINE_HEIGHT));
    assert_eq!(
        first.max_y(),
        second.min_y(),
        "the second line starts where the first ends: neither covers the other"
    );
    assert_eq!(first.size().width(), Au::from_raw(4 * CHAR_ADVANCE));
    assert_eq!(second.size().width(), Au::from_raw(4 * CHAR_ADVANCE));
}

#[test]
fn a_text_fragment_carries_the_text_of_its_line() {
    let laid = Laid::new("<div id=t style='width:47px'>aaaa bbbb</div>");
    let fragments = laid.fragments(laid.text_of("t"));

    assert_eq!(texts(&fragments), ["aaaa", "bbbb"]);
}

#[test]
fn a_run_that_starts_mid_line_keeps_its_first_fragment_where_it_starts() {
    // "xx " is 3 glyphs; the span's "bbbb" follows it on the first line and
    // "cccc" wraps to the second.
    let laid = Laid::new("<div style='width:80px'>xx <span id=s>bbbb cccc</span></div>");
    let span = laid.fragments(laid.element("s"));
    let text = laid.fragments(laid.text_of("s"));

    assert_eq!(span.len(), 2);
    assert_eq!(
        span[0].content().min_x(),
        Au::from_raw(3 * CHAR_ADVANCE),
        "the first line of the span starts after \"xx \", not at the container's edge"
    );
    assert_eq!(span[1].content().min_x(), Au::ZERO);
    assert_eq!(
        texts(&text),
        ["bbbb", "cccc"],
        "the space a wrap replaces is dropped, not left at the end of the first line"
    );
}

#[test]
fn a_preformatted_run_gets_one_fragment_per_source_line() {
    let laid = Laid::new("<pre id=p>a  b\nc</pre>");
    let fragments = laid.fragments(laid.text_of("p"));

    assert_eq!(texts(&fragments), ["a  b", "c"]);
}

#[test]
fn an_element_box_carries_no_text_and_a_one_line_inline_has_one_fragment() {
    let laid = Laid::new("<div id=d><span id=s>one line</span></div>");

    assert_eq!(laid.fragments(laid.element("s")).len(), 1);
    let block = laid.fragments(laid.element("d"));
    assert_eq!(block.len(), 1);
    assert!(block[0].text().is_none(), "only text fragments carry text");
}
