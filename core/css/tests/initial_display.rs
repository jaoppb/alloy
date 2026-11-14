//! The initial `display` is `inline` (CSS Display L3 §2; #136 of #81): an
//! element the UA sheet gives no display — `<img>`, `<u>`, `<sub>`, a custom
//! element — sits on the line with its neighbours instead of breaking it. The
//! root element is blockified (CSS Display L3 §2.7), so no author rule can put
//! a whole page into one line run.
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

#[test]
fn a_sized_image_keeps_its_box_until_83_sizes_replaced_elements() {
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
