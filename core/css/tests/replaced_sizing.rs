//! How a replaced element gets its size (issue #83): the `width` / `height`
//! attributes as presentational hints (WHATWG HTML §15.4.3), with an author
//! declaration winning over them; the natural size of a loaded resource,
//! reached through the [`css::NaturalSizes`] port (CSS 2.1 §10.3.2, §10.6.2);
//! the attribute ratio before it loads; and the size it gets with neither.
//!
//! Text is measured with [`css::BlockLayout::monospace`]: `0.6 * 16px` per
//! glyph and `1.2 * 16px` per line.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;

use css::{
    BlockLayout, CascadeResolver, Display, DomSnapshot, IntrinsicSize, LayoutBox, LayoutBoxTree,
    LayoutEngine, NaturalSize, NaturalSizes, ReplacedResource, SnapshotId, StyledTree, UaCascade,
    ViewportConstraints, collect_style_sheets, snapshot,
};
use graphics::Au;

/// The resources a test has "loaded", keyed by the element's tag: the first
/// element with that tag gets it, every other replaced element is
/// [`ReplacedResource::Unavailable`].
#[derive(Default)]
struct Resources {
    by_tag: BTreeMap<&'static str, ReplacedResource>,
}

impl Resources {
    fn with(mut self, tag: &'static str, resource: ReplacedResource) -> Self {
        self.by_tag.insert(tag, resource);
        self
    }

    fn resolved(self, dom: &DomSnapshot) -> Resolved {
        let by_node = self
            .by_tag
            .into_iter()
            .filter_map(|(tag, resource)| first_tagged(dom, tag).map(|node| (node, resource)))
            .collect();
        Resolved { by_node }
    }
}

struct Resolved {
    by_node: BTreeMap<SnapshotId, ReplacedResource>,
}

impl NaturalSizes for Resolved {
    fn resource_of(&self, node: SnapshotId) -> ReplacedResource {
        self.by_node
            .get(&node)
            .copied()
            .unwrap_or(ReplacedResource::Unavailable)
    }
}

fn first_tagged(dom: &DomSnapshot, tag: &str) -> Option<SnapshotId> {
    dom.nodes_in_document_order()
        .find(|&node| dom.node(node).and_then(css::NodeRef::tag_str) == Some(tag))
}

fn loaded(width: u32, height: u32) -> ReplacedResource {
    ReplacedResource::Loaded(NaturalSize::from_pixels(width, height).unwrap())
}

struct Laid {
    dom: DomSnapshot,
    styled: StyledTree,
    boxes: LayoutBoxTree,
}

impl Laid {
    /// `body` (no UA body margin) laid out at an 800px viewport, with no
    /// resource loaded.
    fn body(body: &str) -> Self {
        Self::loading(body, Resources::default())
    }

    /// `body` laid out with `resources` behind the [`NaturalSizes`] port.
    fn loading(body: &str, resources: Resources) -> Self {
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
            .with_natural_sizes(resources.resolved(&dom))
            .layout(&styled, &viewport)
            .expect("layout succeeds");
        Self { dom, styled, boxes }
    }

    fn tagged(&self, tag: &str) -> SnapshotId {
        first_tagged(&self.dom, tag).unwrap_or_else(|| panic!("no <{tag}>"))
    }

    fn display_of(&self, tag: &str) -> Display {
        self.styled
            .node(self.tagged(tag))
            .expect("styled")
            .style()
            .display()
    }

    fn marker_of(&self, tag: &str) -> IntrinsicSize {
        self.box_of(tag).intrinsic_size()
    }

    /// The single fragment of the text node `text`.
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

const fn px(value: i32) -> Au {
    Au::from_whole_px(value).unwrap()
}

const fn whole_px(value: Au) -> i32 {
    value.raw().checked_div(px(1).raw()).unwrap()
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

// ---- the natural size (CSS 2.1 §10.3.2, §10.6.2) -------------------------

#[test]
fn a_loaded_image_with_no_size_from_css_takes_its_natural_size() {
    let laid = Laid::loading("<img>", Resources::default().with("img", loaded(40, 20)));
    assert_eq!(laid.content_px("img"), (40, 20));
}

#[test]
fn a_css_width_alone_scales_the_height_by_the_natural_ratio() {
    let laid = Laid::loading(
        "<img style='width:80px'>",
        Resources::default().with("img", loaded(40, 20)),
    );
    assert_eq!(laid.content_px("img"), (80, 40));
}

#[test]
fn a_css_height_alone_scales_the_width_by_the_natural_ratio() {
    let laid = Laid::loading(
        "<img style='height:10px'>",
        Resources::default().with("img", loaded(40, 20)),
    );
    assert_eq!(laid.content_px("img"), (20, 10));
}

#[test]
fn both_css_axes_stretch_the_image_whatever_its_ratio() {
    let laid = Laid::loading(
        "<img style='width:10px;height:10px'>",
        Resources::default().with("img", loaded(40, 20)),
    );
    assert_eq!(laid.content_px("img"), (10, 10));
}

#[test]
fn a_block_level_image_takes_its_natural_width_not_its_containing_blocks() {
    let laid = Laid::loading(
        "<img style='display:block'>",
        Resources::default().with("img", loaded(40, 20)),
    );
    assert_eq!(laid.content_px("img"), (40, 20));
}

// ---- before the resource arrives ------------------------------------------

#[test]
fn the_attribute_ratio_sizes_an_image_that_is_still_loading() {
    let laid = Laid::loading(
        "<img width=100 height=50 style='width:40px;height:auto'>",
        Resources::default().with("img", ReplacedResource::Loading),
    );
    assert_eq!(laid.content_px("img"), (40, 20));
}

#[test]
fn the_natural_ratio_beats_the_attribute_ratio_once_loaded() {
    let laid = Laid::loading(
        "<img width=100 height=50 style='width:40px;height:auto'>",
        Resources::default().with("img", loaded(30, 30)),
    );
    assert_eq!(laid.content_px("img"), (40, 40));
}

#[test]
fn an_unsized_image_that_is_loading_or_unavailable_is_empty() {
    let loading = Laid::loading(
        "<img>",
        Resources::default().with("img", ReplacedResource::Loading),
    );
    assert_eq!(loading.content_px("img"), (0, 0));
    let unavailable = Laid::body("<img>");
    assert_eq!(unavailable.content_px("img"), (0, 0));
}

#[test]
fn an_unsized_embedded_element_gets_the_default_object_size() {
    for tag in ["video", "canvas", "iframe", "object", "embed"] {
        let laid = Laid::body(&format!("<{tag}></{tag}>"));
        assert_eq!(laid.content_px(tag), (300, 150), "<{tag}>");
    }
}

// ---- an inline replaced element is an atomic inline (CSS 2.1 §9.2.2) -----

#[test]
fn an_inline_image_is_an_atomic_inline_on_its_neighbours_line() {
    let laid = Laid::body("aa<img width=10 height=10>bb");
    assert_eq!(laid.display_of("img"), Display::Inline, "no UA stopgap");
    assert_eq!(laid.content_px("img"), (10, 10));
    let before = laid.text("aa").content();
    let after = laid.text("bb").content();
    assert_eq!(before.min_y(), after.min_y(), "one line box");
    assert!(
        after.min_x() > before.max_x(),
        "the image sits between them"
    );
}

#[test]
fn a_block_inside_an_inline_replaced_element_does_not_split_the_line() {
    let laid = Laid::body("aa<video width=40 height=20>x<div>fallback</div>y</video>bb");
    assert_eq!(laid.content_px("video"), (40, 20));
    let before = laid.text("aa").content();
    let after = laid.text("bb").content();
    assert_eq!(
        before.min_y(),
        after.min_y(),
        "the fallback block splits nothing"
    );
}

// ---- what layout reports about the resource -------------------------------

#[test]
fn a_loaded_image_reports_its_natural_size() {
    let laid = Laid::loading(
        "<img style='width:80px'>",
        Resources::default().with("img", loaded(40, 20)),
    );
    let natural = NaturalSize::from_pixels(40, 20).unwrap();
    assert_eq!(laid.marker_of("img"), IntrinsicSize::Natural(natural));
}

#[test]
fn a_loading_image_is_pending_until_both_axes_are_pinned() {
    let resources = || Resources::default().with("img", ReplacedResource::Loading);
    let unpinned = Laid::loading("<img style='width:20px'>", resources());
    assert_eq!(unpinned.marker_of("img"), IntrinsicSize::Pending);
    let pinned = Laid::loading("<img width=20 height=10>", resources());
    assert_eq!(pinned.marker_of("img"), IntrinsicSize::Resolved);
}

#[test]
fn an_unavailable_image_is_resolved_its_size_will_not_change() {
    let laid = Laid::body("<img>");
    assert_eq!(laid.marker_of("img"), IntrinsicSize::Resolved);
}

// ---- `alt` text (WHATWG HTML §15.4.2) -------------------------------------

impl Laid {
    /// Every fragment of the first `<tag>`: its box first, then the lines of
    /// any text it lays out itself.
    fn fragments(&self, tag: &str) -> Vec<&LayoutBox> {
        self.boxes.fragments_of(self.tagged(tag)).collect()
    }

    /// The text the first `<tag>` draws itself, line by line.
    fn own_text(&self, tag: &str) -> Vec<String> {
        self.fragments(tag)
            .iter()
            .filter_map(|fragment| fragment.text())
            .map(|text| text.as_str().to_owned())
            .collect()
    }
}

#[test]
fn an_unavailable_image_shows_its_alt_text_in_a_box_that_fits_it() {
    let laid = Laid::body("<img alt='hello world'>");
    assert_eq!(laid.own_text("img"), ["hello world"]);
    let own = laid.fragments("img")[0].content().size();
    // 11 glyphs at 0.6 × 16px, one 1.2 × 16px line.
    assert_eq!(own.width(), Au::from_raw(px(1).raw() * 96 / 10 * 11));
    assert_eq!(own.height(), Au::from_raw(px(1).raw() * 192 / 10));
}

#[test]
fn a_sized_unavailable_image_keeps_its_size_and_shows_its_alt_inside() {
    let laid = Laid::body("<img width=100 height=50 alt='hi'>");
    assert_eq!(laid.own_text("img"), ["hi"]);
    let own = laid.fragments("img")[0].content().size();
    assert_eq!((whole_px(own.width()), whole_px(own.height())), (100, 50));
}

#[test]
fn an_empty_alt_shows_nothing() {
    let laid = Laid::body("<img alt=''>");
    assert!(laid.own_text("img").is_empty());
    assert_eq!(laid.content_px("img"), (0, 0));
}

#[test]
fn a_loaded_image_never_shows_its_alt_text() {
    let laid = Laid::loading(
        "<img alt='hello'>",
        Resources::default().with("img", loaded(40, 20)),
    );
    assert!(laid.own_text("img").is_empty());
    assert_eq!(laid.content_px("img"), (40, 20));
}

#[test]
fn a_loading_image_shows_no_alt_text_yet() {
    let laid = Laid::loading(
        "<img alt='hello'>",
        Resources::default().with("img", ReplacedResource::Loading),
    );
    assert!(laid.own_text("img").is_empty());
    assert_eq!(laid.content_px("img"), (0, 0));
}

#[test]
fn an_alt_text_sits_on_its_neighbours_line() {
    let laid = Laid::body("aa<img alt='hi'>bb");
    let before = laid.text("aa").content();
    let after = laid.text("bb").content();
    assert_eq!(before.min_y(), after.min_y(), "one line box");
    let alt = laid.fragments("img")[0].content();
    assert!(alt.min_x() >= before.max_x() && after.min_x() >= alt.max_x());
}

#[test]
fn every_fragment_of_an_image_showing_its_alt_is_resolved() {
    let laid = Laid::body("<img alt='hello world'>");
    let markers: Vec<IntrinsicSize> = laid
        .fragments("img")
        .iter()
        .map(|fragment| fragment.intrinsic_size())
        .collect();
    assert!(markers.len() > 1, "a box and its text line");
    assert!(
        markers
            .iter()
            .all(|marker| *marker == IntrinsicSize::Resolved)
    );
}

#[test]
fn a_dimension_attribute_reads_the_same_with_or_without_px() {
    let resources = || Resources::default().with("img", ReplacedResource::Loading);
    let style = "style='width:40px;height:auto'";
    let bare = Laid::loading(&format!("<img width=100 height=50 {style}>"), resources());
    let suffixed = Laid::loading(
        &format!("<img width=100px height=50px {style}>"),
        resources(),
    );
    assert_eq!(bare.content_px("img"), (40, 20));
    assert_eq!(suffixed.content_px("img"), (40, 20));
    assert_eq!(
        Laid::body("<img width=100px height=50px>").content_px("img"),
        (100, 50)
    );
}
