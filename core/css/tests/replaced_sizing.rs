//! How a replaced element gets its size (issue #83): the `width` / `height`
//! attributes as presentational hints (WHATWG HTML §15.4.3), with an author
//! declaration winning over them.
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
    BlockLayout, CascadeResolver, DomSnapshot, LayoutBox, LayoutBoxTree, LayoutEngine, SnapshotId,
    UaCascade, ViewportConstraints, collect_style_sheets, snapshot,
};
use graphics::Au;

struct Laid {
    dom: DomSnapshot,
    boxes: LayoutBoxTree,
}

impl Laid {
    /// `body` (no UA body margin) laid out at an 800px viewport.
    fn body(body: &str) -> Self {
        let markup = format!(
            "<!doctype html><html><head><style>body {{ margin: 0 }}</style></head><body>{body}</body></html>"
        );
        let tree = dom::parse(&markup).expect("the fixture parses").into_tree();
        let dom = snapshot(&tree, tree.document());
        let sheets = collect_style_sheets(&dom).expect("the fixture's CSS is readable");
        let styled = UaCascade::new()
            .resolve(&dom, &sheets)
            .expect("the cascade resolves");
        let viewport = ViewportConstraints::new(px(800), px(600));
        let boxes = BlockLayout::monospace()
            .layout(&styled, &viewport)
            .expect("layout succeeds");
        Self { dom, boxes }
    }

    fn tagged(&self, tag: &str) -> SnapshotId {
        self.dom
            .nodes_in_document_order()
            .find(|&node| self.dom.node(node).and_then(css::NodeRef::tag_str) == Some(tag))
            .unwrap_or_else(|| panic!("no <{tag}>"))
    }

    /// The one fragment of the first `<tag>`.
    fn box_of(&self, tag: &str) -> &LayoutBox {
        let fragments: Vec<&LayoutBox> = self.boxes.fragments_of(self.tagged(tag)).collect();
        assert_eq!(fragments.len(), 1, "<{tag}> is one box");
        fragments[0]
    }

    /// The content-box size of the first `<tag>`, in whole pixels.
    fn content_px(&self, tag: &str) -> (i32, i32) {
        let size = self.box_of(tag).content().size();
        (whole_px(size.width()), whole_px(size.height()))
    }
}

fn px(value: i32) -> Au {
    Au::from_whole_px(value).unwrap()
}

fn whole_px(value: Au) -> i32 {
    value.raw() / px(1).raw()
}

/// Every replaced element the dimension attributes apply to (HTML §15.4.3).
const DIMENSIONED: [&str; 6] = ["img", "video", "canvas", "iframe", "object", "embed"];

#[test]
fn the_width_and_height_attributes_size_every_replaced_element() {
    for tag in DIMENSIONED {
        let laid = Laid::body(&format!("<{tag} width=100 height=50></{tag}>"));
        assert_eq!(laid.content_px(tag), (100, 50), "<{tag}>");
    }
}

#[test]
fn an_author_declaration_beats_the_dimension_attributes() {
    let laid = Laid::body("<img width=100 height=50 style='width:30px;height:10px'>");
    assert_eq!(laid.content_px("img"), (30, 10));
}

#[test]
fn a_percentage_dimension_attribute_resolves_against_the_containing_block() {
    let laid = Laid::body("<div style='width:200px'><img width='50%' height=10></div>");
    assert_eq!(laid.content_px("img"), (100, 10));
}

#[test]
fn a_dimension_attribute_on_a_non_replaced_element_is_ignored() {
    let laid = Laid::body("<div width=100 height=50>x</div>");
    let size = laid.box_of("div").content().size();
    assert_eq!(whole_px(size.width()), 800, "a div still fills its line");
}
