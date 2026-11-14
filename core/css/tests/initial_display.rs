//! The initial `display` is `inline` (CSS Display L3 §2; #136 of #81): an
//! element the UA sheet gives no display — `<u>`, `<sub>`, a custom element —
//! sits on the line with its neighbours instead of breaking it. A replaced
//! element keeps `inline` too, and layout lays it out as one atomic inline
//! (CSS 2.1 §9.2.2, #83), so `<img>` sits on the line and keeps its size. The
//! root element is blockified
//! (CSS Display L3 §2.7), whichever resolver cascaded it.
//!
//! Text is measured with [`css::BlockLayout::monospace`]: `0.6 * 16px` per
//! glyph and `1.2 * 16px` per line.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use css::{
    BlockLayout, CascadeResolver, ComputedStyle, Display, DomSnapshot, LayoutBox, LayoutBoxTree,
    LayoutEngine, SnapshotId, StyledTree, UaCascade, ViewportConstraints, collect_style_sheets,
    snapshot,
};
use graphics::Au;

struct Laid {
    dom: DomSnapshot,
    styled: StyledTree,
    boxes: LayoutBoxTree,
}

impl Laid {
    /// A whole document laid out at an 800px viewport.
    fn document(markup: &str) -> Self {
        let tree = dom::parse(markup).expect("the fixture parses").into_tree();
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

    /// `body` with no UA body margin.
    fn body(body: &str) -> Self {
        Self::document(&format!(
            "<!doctype html><html><head><style>body {{ margin: 0 }}</style></head><body>{body}</body></html>"
        ))
    }

    fn text(&self, text: &str) -> &LayoutBox {
        let node = self
            .dom
            .nodes_in_document_order()
            .find(|&node| self.dom.node(node).and_then(css::NodeRef::text) == Some(text))
            .unwrap_or_else(|| panic!("no text node \"{text}\""));
        let fragments: Vec<&LayoutBox> = self.boxes.fragments_of(node).collect();
        assert_eq!(fragments.len(), 1, "\"{text}\" fits on one line");
        fragments[0]
    }

    fn tagged(&self, tag: &str) -> SnapshotId {
        self.dom
            .nodes_in_document_order()
            .find(|&node| self.dom.node(node).and_then(css::NodeRef::tag_str) == Some(tag))
            .unwrap_or_else(|| panic!("no <{tag}>"))
    }

    fn display_of(&self, tag: &str) -> Display {
        self.styled
            .node(self.tagged(tag))
            .expect("styled")
            .style()
            .display()
    }

    fn assert_one_row(&self, before: &str, after: &str) {
        let first = self.text(before).content();
        let last = self.text(after).content();
        assert_eq!(
            first.min_y(),
            last.min_y(),
            "\"{before}\" and \"{after}\" share a row"
        );
        assert!(
            last.min_x() > first.min_x(),
            "\"{after}\" follows \"{before}\""
        );
    }
}

#[test]
fn the_initial_display_is_inline() {
    assert_eq!(ComputedStyle::initial().display(), Display::Inline);
    assert_eq!(Display::default(), Display::Inline);
}

#[test]
fn an_image_stays_on_the_line_of_its_neighbours() {
    let laid = Laid::body("aaa<img style='width:10px;height:10px'>bbb");
    laid.assert_one_row("aaa", "bbb");
}

#[test]
fn an_underline_stays_on_the_line_of_its_neighbours() {
    let laid = Laid::body("aa<u>bb</u>cc");
    laid.assert_one_row("aa", "cc");
    laid.assert_one_row("aa", "bb");
}

#[test]
fn a_subscript_stays_on_the_line_of_its_neighbours() {
    let laid = Laid::body("H<sub>2</sub>O");
    laid.assert_one_row("H", "O");
}

#[test]
fn a_custom_element_stays_on_the_line_of_its_neighbours() {
    let laid = Laid::body("aa<x-foo>bb</x-foo>cc");
    laid.assert_one_row("aa", "cc");
    assert_eq!(laid.display_of("x-foo"), Display::Inline);
}

#[test]
fn the_root_element_is_blockified() {
    let laid = Laid::document(
        "<!doctype html><html><head><style>html { display: inline } body { margin: 0 }</style>\
         </head><body><p>one</p><p>two</p></body></html>",
    );

    assert_eq!(
        laid.display_of("html"),
        Display::Block,
        "CSS Display L3 §2.7"
    );
    let one = laid.text("one").content();
    let two = laid.text("two").content();
    assert!(two.min_y() > one.max_y(), "the paragraphs still stack");
}

#[test]
fn an_inline_block_root_is_blockified_too() {
    let laid =
        Laid::document("<!doctype html><html style='display:inline-block'><body>x</body></html>");
    assert_eq!(laid.display_of("html"), Display::Block);
}

/// `embed[hidden]` is `inline` and 0×0 (WHATWG HTML §15.3.1): a zero-size
/// atomic box on its neighbours' line.
#[test]
fn a_hidden_embed_is_a_zero_size_box_on_the_line() {
    let laid = Laid::body("aaa<embed hidden>bbb");
    let embed: Vec<&LayoutBox> = laid.boxes.fragments_of(laid.tagged("embed")).collect();

    assert_eq!(embed.len(), 1, "the hidden embed still generates its box");
    assert!(embed[0].content().size().width().is_zero());
    assert!(embed[0].content().size().height().is_zero());
    laid.assert_one_row("aaa", "bbb");
}

#[test]
fn a_sized_image_keeps_its_box_as_an_atomic_inline() {
    let laid = Laid::body("aaa<img style='width:10px;height:10px'>bbb");
    let image: Vec<&LayoutBox> = laid.boxes.fragments_of(laid.tagged("img")).collect();

    assert_eq!(
        image.len(),
        1,
        "the image still generates a box, so it still paints"
    );
    let size = image[0].content().size();
    assert_eq!(size.width(), Au::from_whole_px(10).unwrap());
    assert_eq!(size.height(), Au::from_whole_px(10).unwrap());
}

/// Every replaced element, as `core/css/src/domain/replaced.rs` lists them.
const REPLACED: [&str; 7] = ["img", "video", "canvas", "iframe", "object", "embed", "svg"];

#[test]
fn every_replaced_element_keeps_its_box_and_hides_its_fallback() {
    for tag in REPLACED {
        assert_eq!(
            Laid::body(&format!("<{tag}></{tag}>")).display_of(tag),
            Display::Inline,
            "<{tag}> needs no UA display rule"
        );
        let laid = Laid::body(&format!(
            "aa<{tag} style='width:40px;height:20px'>fallback</{tag}>bb"
        ));
        let boxes: Vec<&LayoutBox> = laid.boxes.fragments_of(laid.tagged(tag)).collect();
        assert_eq!(boxes.len(), 1, "<{tag}> still generates its box");
        assert_eq!(
            boxes[0].content().size().width(),
            Au::from_whole_px(40).unwrap(),
            "<{tag}>"
        );
        assert_eq!(
            fallback_fragments(&laid, tag),
            0,
            "<{tag}>'s fallback content is not laid out"
        );
    }
}

/// How many fragments the text node `fallback` gets when it is a child of
/// `<tag>`. A void `<img>` has no children: its "fallback" is a sibling, so
/// it counts as none here.
fn fallback_fragments(laid: &Laid, tag: &str) -> usize {
    let element = laid.tagged(tag);
    let Some(fallback) = laid
        .dom
        .node(element)
        .and_then(|node| node.children().next())
    else {
        return 0;
    };
    laid.boxes.fragments_of(fallback).count()
}

#[test]
fn widgets_the_spec_renders_as_inline_blocks_are_inline_blocks() {
    for tag in ["marquee", "meter", "progress"] {
        let laid = Laid::body(&format!("<{tag}>x</{tag}>"));
        assert_eq!(
            laid.display_of(tag),
            Display::InlineBlock,
            "<{tag}>, WHATWG HTML §15.5"
        );
    }
}

#[test]
fn every_cascade_resolver_gets_a_blockified_root() {
    let tree = dom::parse("<!doctype html><html><body>x</body></html>")
        .expect("parses")
        .into_tree();
    let dom = snapshot(&tree, tree.document());
    // A resolver that applies no rule at all, like a script cascade's fallback.
    let styled = StyledTree::recompute_in_document_order(&dom, |_, _| ComputedStyle::initial());
    let html = dom
        .nodes_in_document_order()
        .find(|&node| dom.node(node).and_then(css::NodeRef::tag_str) == Some("html"))
        .unwrap();

    assert_eq!(styled.node(html).unwrap().style().display(), Display::Block);
}
