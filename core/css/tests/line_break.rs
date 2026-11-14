//! `<br>` as a forced line break (issue #82): the inline formatting context
//! ends the current line box at a `br` and keeps a strut-high empty line when
//! the line it ends holds nothing else (WHATWG HTML §15.3.4,
//! `br { display-outside: newline }`).
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
    BlockLayout, CascadeResolver, Display, DomSnapshot, LayoutBox, LayoutBoxTree, LayoutEngine,
    NodeRole, SnapshotId, StyledTree, UaCascade, ViewportConstraints, collect_style_sheets,
    snapshot,
};
use graphics::Au;

/// `1.2 * 16px` — one line's height, in `Au` raw units.
const LINE_HEIGHT: i32 = 1228;
/// `0.6 * 16px` — one glyph's advance, in `Au` raw units.
const CHAR_ADVANCE: i32 = 614;

struct Laid {
    dom: DomSnapshot,
    styled: StyledTree,
    boxes: LayoutBoxTree,
}

impl Laid {
    /// `body` inside a 400px-wide `div#container`, with `author` CSS and no UA
    /// margins.
    fn styled(author: &str, body: &str) -> Self {
        let markup = format!(
            "<!doctype html><html><head><style>body {{ margin: 0 }} {author}</style></head>\
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
        Self { dom, styled, boxes }
    }

    fn new(body: &str) -> Self {
        Self::styled("", body)
    }

    fn element(&self, id: &str) -> SnapshotId {
        self.dom
            .nodes_in_document_order()
            .find(|&node| self.dom.node(node).and_then(|n| n.attribute("id")) == Some(id))
            .unwrap_or_else(|| panic!("no element with id=\"{id}\""))
    }

    /// The one fragment of the text node whose text is exactly `text`.
    fn text_line(&self, text: &str) -> &LayoutBox {
        let node = self
            .dom
            .nodes_in_document_order()
            .find(|&node| self.dom.node(node).and_then(css::NodeRef::text) == Some(text))
            .unwrap_or_else(|| panic!("no text node \"{text}\""));
        let fragments: Vec<&LayoutBox> = self.boxes.fragments_of(node).collect();
        assert_eq!(fragments.len(), 1, "\"{text}\" fits on one line");
        fragments[0]
    }

    fn top_of(&self, text: &str) -> Au {
        self.text_line(text).content().min_y()
    }

    fn fragments(&self, id: &str) -> Vec<&LayoutBox> {
        self.boxes.fragments_of(self.element(id)).collect()
    }

    /// The content height of `div#container`.
    fn container_height(&self) -> Au {
        let container = self.fragments("container");
        container[0].content().size().height()
    }

    fn role_of(&self, id: &str) -> NodeRole {
        self.styled
            .node(self.element(id))
            .expect("the element is styled")
            .role()
    }

    fn display_of(&self, id: &str) -> Display {
        self.styled
            .node(self.element(id))
            .expect("the element is styled")
            .style()
            .display()
    }
}

const fn lines(count: i32) -> Au {
    Au::from_raw(LINE_HEIGHT.saturating_mul(count))
}

// ---- the six line-box cases ------------------------------------------------

#[test]
fn a_br_ends_the_line_it_sits_on() {
    let laid = Laid::new("aaa<br>bbb");

    assert_eq!(laid.top_of("aaa"), Au::ZERO);
    assert_eq!(laid.top_of("bbb"), lines(1));
    assert_eq!(laid.container_height(), lines(2));
}

#[test]
fn two_brs_leave_an_empty_line_between_the_words() {
    let laid = Laid::new("aaa<br><br>bbb");

    assert_eq!(
        laid.top_of("bbb"),
        lines(2),
        "the second br holds a strut-high empty line"
    );
    assert_eq!(laid.container_height(), lines(3));
}

#[test]
fn a_trailing_br_adds_no_line() {
    let laid = Laid::new("aaa<br>");

    assert_eq!(laid.container_height(), lines(1));
}

#[test]
fn a_br_alone_holds_one_empty_line() {
    let laid = Laid::new("<br>");

    assert_eq!(laid.container_height(), lines(1));
    assert_eq!(
        laid.fragments("container").len(),
        1,
        "the container is not vacant, so it is laid out like any block"
    );
}

#[test]
fn the_spaces_around_a_br_are_dropped() {
    let laid = Laid::new("aaa <br> bbb");

    assert_eq!(
        laid.text_line("aaa ").content().size().width(),
        Au::from_raw(CHAR_ADVANCE.saturating_mul(3)),
        "the space before the br does not hang on the first line"
    );
    let second = laid.text_line(" bbb").content();
    assert_eq!(second.min_y(), lines(1));
    assert_eq!(
        second.min_x(),
        Au::ZERO,
        "the space after the br does not open the second line"
    );
}

#[test]
fn a_br_inside_an_inline_box_breaks_the_line_the_same_way() {
    let laid = Laid::new("<span id=s>aaa<br>bbb</span>");

    assert_eq!(laid.top_of("bbb"), lines(1));
    let span = laid.fragments("s");
    assert_eq!(span.len(), 2, "the span gets one fragment per line");
    assert_eq!(span[1].content().min_y(), lines(1));
}

// ---- display on a br -------------------------------------------------------

#[test]
fn a_br_with_display_none_breaks_nothing() {
    let laid = Laid::styled("br { display: none }", "aaa<br>bbb");

    assert_eq!(laid.top_of("bbb"), Au::ZERO);
    assert_eq!(laid.container_height(), lines(1));
}

#[test]
fn a_br_with_display_block_still_breaks_as_an_inline_level_box() {
    // As two empty blocks, the brs would collapse to nothing between "aaa" and
    // "bbb"; as forced breaks the second one leaves an empty line.
    let laid = Laid::styled("br { display: block }", "aaa<br id=b><br>bbb");

    assert_eq!(laid.top_of("bbb"), lines(2));
    assert_eq!(laid.display_of("b"), Display::Inline);
}

#[test]
fn a_br_with_display_inline_block_still_breaks() {
    let laid = Laid::styled("br { display: inline-block }", "aaa<br><br>bbb");

    assert_eq!(laid.top_of("bbb"), lines(2));
}

// ---- the styled tree -------------------------------------------------------

#[test]
fn only_a_br_has_the_line_break_role() {
    let laid = Laid::new("<span id=s>aaa</span><br id=b>");

    assert_eq!(laid.role_of("b"), NodeRole::LineBreak);
    assert_eq!(laid.role_of("s"), NodeRole::Ordinary);
    assert_eq!(laid.role_of("container"), NodeRole::Ordinary);
}

#[test]
fn a_hidden_br_keeps_display_none() {
    let laid = Laid::styled("br { display: none }", "<br id=b>");

    assert_eq!(laid.display_of("b"), Display::None);
}
