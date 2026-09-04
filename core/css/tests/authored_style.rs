//! Guards the B1 deliverable of `plano:430-431`: a `<style>` element and a
//! `style=` attribute are **observable** in the `StyledTree` a
//! `CascadeResolver` returns, ordered by specificity. B2 (`plano:435-443`)
//! extends the same chain: `!important`, `rgb()` / `rgba()`, and the parsed
//! `assets/ua.css` base layer.
//!
//! The chain under test is the whole one — `dom::DomTree → snapshot →
//! collect_style_sheets → UaCascade::resolve` — because every link of it is new
//! in B1 and a unit test of any single link would still pass if the next one
//! stopped applying what it was handed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use css::{
    CascadeResolver, ComputedStyle, CssColor, DomSnapshot, Length, LengthEdges, StyleSheetSet,
    StyledTree, UaCascade, collect_style_sheets, snapshot,
};

const RED: CssColor = CssColor::rgb(0xFF, 0x00, 0x00);
const BLUE: CssColor = CssColor::rgb(0x00, 0x00, 0xFF);
const GREEN: CssColor = CssColor::rgb(0x00, 0x80, 0x00);

/// `<html><body><style>{sheet}</style><p class="lead" id="first" {attribute}>Hi</p></body></html>`
fn document(sheet: &str, attribute: Option<&str>) -> (dom::DomTree, dom::NodeId) {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let html = child(&mut tree, root, "html");
    let body = child(&mut tree, html, "body");

    let style = child(&mut tree, body, "style");
    text(&mut tree, style, sheet);

    let paragraph = child(&mut tree, body, "p");
    attribute_of(&mut tree, paragraph, "class", "lead");
    attribute_of(&mut tree, paragraph, "id", "first");
    set_inline_style(&mut tree, paragraph, attribute);
    text(&mut tree, paragraph, "Hi");
    (tree, root)
}

fn set_inline_style(tree: &mut dom::DomTree, node: dom::NodeId, attribute: Option<&str>) {
    let Some(value) = attribute else {
        return;
    };
    attribute_of(tree, node, "style", value);
}

fn child(tree: &mut dom::DomTree, parent: dom::NodeId, name: &str) -> dom::NodeId {
    let node = tree.create_element(dom::TagName::new(name).unwrap());
    tree.append_child(parent, node).unwrap();
    node
}

fn attribute_of(tree: &mut dom::DomTree, node: dom::NodeId, name: &str, value: &str) {
    tree.set_attribute(
        node,
        dom::AttributeName::new(name).unwrap(),
        dom::AttributeValue::new(value),
    )
    .unwrap();
}

fn text(tree: &mut dom::DomTree, parent: dom::NodeId, content: &str) {
    let node = tree.create_text(dom::TextContent::new(content));
    tree.append_child(parent, node).unwrap();
}

/// The whole chain, from a document to the styled tree its own CSS produces.
fn resolve(sheet: &str, attribute: Option<&str>) -> (DomSnapshot, StyledTree) {
    let (tree, root) = document(sheet, attribute);
    let dom = snapshot(&tree, root);
    let sheets = collect_style_sheets(&dom).expect("the document's CSS is readable");
    let styled = UaCascade::new()
        .resolve(&dom, &sheets)
        .expect("the cascade resolves");
    (dom, styled)
}

/// The computed style of the one `<p>`.
fn paragraph_style(sheet: &str, attribute: Option<&str>) -> ComputedStyle {
    let (dom, styled) = resolve(sheet, attribute);
    let id = dom
        .nodes_in_document_order()
        .find(|id| dom.node(*id).and_then(css::NodeRef::tag) == Some("p"))
        .expect("the document has a paragraph");
    *styled.node(id).expect("the paragraph is styled").style()
}

// ---- the deliverable -----------------------------------------------------

#[test]
fn a_style_element_is_applied_through_the_cascade_resolver() {
    let style = paragraph_style("p { color: #0000ff; margin: 4px }", None);

    assert_eq!(style.color(), BLUE, "the author rule set the colour");
    assert_eq!(
        style.margin(),
        LengthEdges::uniform(Length::Pixels(4.0)),
        "the author rule replaced the UA `<p>` margin"
    );
}

#[test]
fn a_style_attribute_outranks_every_rule_that_selects_the_same_node() {
    let style = paragraph_style(
        "p#first.lead { color: #0000ff }",
        Some("color: #ff0000; margin-top: 9px"),
    );

    assert_eq!(
        style.color(),
        RED,
        "an inline declaration beats even an id+class rule (CSS Cascade L4 §6.4.3)"
    );
    assert_eq!(
        style.margin().top(),
        Length::Pixels(9.0),
        "a longhand from the inline block lands on its own side"
    );
}

#[test]
fn an_id_beats_a_class_which_beats_a_type_whatever_the_source_order() {
    // Source order runs weakest-last on purpose: only specificity can decide.
    let style = paragraph_style(
        "#first { color: #ff0000 } .lead { color: #008000 } p { color: #0000ff }",
        None,
    );
    assert_eq!(style.color(), RED, "`#first` wins");

    let without_id = paragraph_style(".lead { color: #008000 } p { color: #0000ff }", None);
    assert_eq!(without_id.color(), GREEN, "`.lead` wins over `p`");

    let type_only = paragraph_style("p { color: #0000ff }", None);
    assert_eq!(type_only.color(), BLUE);
}

#[test]
fn two_rules_of_equal_specificity_are_decided_by_source_order() {
    let style = paragraph_style(".lead { color: #0000ff } .lead { color: #ff0000 }", None);
    assert_eq!(
        style.color(),
        RED,
        "the later rule wins (CSS Cascade L4 §6.4.4)"
    );
}

#[test]
fn the_author_colour_is_inherited_by_a_descendant_that_no_rule_selects() {
    let (dom, styled) = resolve("body { color: #008000 }", None);
    let paragraph = dom
        .nodes_in_document_order()
        .find(|id| dom.node(*id).and_then(css::NodeRef::tag) == Some("p"))
        .expect("a paragraph");

    assert_eq!(
        styled.node(paragraph).expect("styled").style().color(),
        GREEN,
        "`color` inherits, so the body rule reaches the paragraph"
    );
}

#[test]
fn a_media_gated_rule_only_applies_once_the_producer_has_discharged_it() {
    let (tree, root) = document("@media (min-width: 600px) { p { color: #0000ff } }", None);
    let dom = snapshot(&tree, root);
    let sheets = collect_style_sheets(&dom).expect("readable");

    let ungated = UaCascade::new().resolve(&dom, &sheets).expect("resolves");
    let paragraph = dom
        .nodes_in_document_order()
        .find(|id| dom.node(*id).and_then(css::NodeRef::tag) == Some("p"))
        .expect("a paragraph");
    assert_eq!(
        ungated.node(paragraph).expect("styled").style().color(),
        CssColor::BLACK,
        "a resolver receives no viewport, so it skips an unevaluated condition (PRD-007:56-60)"
    );

    let wide = css::ViewportConstraints::new(whole_px(800), whole_px(600));
    let discharged = UaCascade::new()
        .resolve(&dom, &sheets.matching_viewport(&wide))
        .expect("resolves");
    assert_eq!(
        discharged.node(paragraph).expect("styled").style().color(),
        BLUE,
        "once the producer has evaluated the query, the rule applies"
    );
}

const fn whole_px(pixels: i32) -> graphics::Au {
    graphics::Au::from_whole_px(pixels).expect("a small pixel count fits")
}

// ---- B2: `!important`, `rgb()` / `rgba()`, `assets/ua.css` ---------------

#[test]
fn an_important_declaration_wins_over_a_higher_specificity_normal_one() {
    // `#first.lead` is `(1,1,0)`, strictly stronger than the bare-type `p`
    // rule's `(0,0,1)` — and it still loses, because `!important` is a
    // separate, stronger precedence tier (CSS Cascade L4 §4.2).
    let style = paragraph_style(
        "p { color: #0000ff !important } #first.lead { color: #ff0000 }",
        None,
    );

    assert_eq!(
        style.color(),
        BLUE,
        "the `!important` type-selector rule outranks a normal id+class rule"
    );
}

#[test]
fn important_only_beats_important_by_the_normal_cascade_rules() {
    // Two `!important` declarations still order by specificity, exactly like
    // two normal ones do — `!important` moves a declaration to a stronger
    // tier, it does not disable ordering within that tier.
    let style = paragraph_style(
        "p { color: #0000ff !important } #first.lead { color: #ff0000 !important }",
        None,
    );

    assert_eq!(
        style.color(),
        RED,
        "both declarations are `!important`, so specificity decides again"
    );
}

#[test]
fn rgb_and_rgba_colours_apply_through_the_cascade_resolver() {
    let style = paragraph_style(
        "p { color: rgb(0, 0, 255); background-color: rgba(255, 0, 0, 0.5) }",
        None,
    );

    assert_eq!(style.color(), BLUE, "`rgb()` parses and applies");
    assert_eq!(
        style.background_color(),
        CssColor::rgba(0xFF, 0x00, 0x00, 0x80),
        "`rgba()`'s float alpha resolves to the nearest 8-bit level"
    );
}

#[test]
fn rgb_clamps_an_out_of_range_component_instead_of_refusing_it() {
    let style = paragraph_style("p { color: rgb(300, -10, 128) }", None);

    assert_eq!(
        style.color(),
        CssColor::rgb(0xFF, 0x00, 0x80),
        "300 clamps to 255 and -10 clamps to 0 (CSS Color L4 §5.1), never a refusal"
    );
}

#[test]
fn the_initial_keyword_resets_a_property_ignoring_inheritance() {
    let style = paragraph_style("body { color: #008000 } p { color: initial }", None);

    assert_eq!(
        style.color(),
        CssColor::BLACK,
        "`initial` takes the CSS initial value, not the inherited body colour"
    );
}

#[test]
fn the_inherit_keyword_forces_inheritance_of_a_non_inherited_property() {
    let style = paragraph_style(
        "body { background-color: #008000 } p { background-color: inherit }",
        None,
    );

    assert_eq!(
        style.background_color(),
        GREEN,
        "`background-color` does not normally inherit, but `inherit` forces it"
    );
}

#[test]
fn the_ua_sheet_still_gives_a_heading_and_a_paragraph_their_classic_shape() {
    // Proves `assets/ua.css`, parsed by `UaCascade::new`, reproduces exactly
    // what B1's hard-coded `style_for_tag` used to set — with no author CSS
    // at all.
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let html = child(&mut tree, root, "html");
    let body = child(&mut tree, html, "body");
    let heading = child(&mut tree, body, "h1");
    text(&mut tree, heading, "Alloy");
    let paragraph = child(&mut tree, body, "p");
    text(&mut tree, paragraph, "Hi");

    let dom = snapshot(&tree, root);
    let sheets = collect_style_sheets(&dom).expect("readable");
    let styled = UaCascade::new().resolve(&dom, &sheets).expect("resolves");

    let heading_style = styled.node(styled_id(&dom, "h1")).expect("styled").style();
    assert_eq!(heading_style.display(), css::Display::Block);
    assert_eq!(heading_style.font_size(), Length::Em(2.0));
    assert_eq!(
        heading_style.margin(),
        LengthEdges::vertical(Length::Em(0.67))
    );

    let paragraph_style = styled.node(styled_id(&dom, "p")).expect("styled").style();
    assert_eq!(
        paragraph_style.margin(),
        LengthEdges::vertical(Length::Pixels(16.0)),
        "the UA `<p>` margin still comes through, now from assets/ua.css"
    );
}

fn styled_id(dom: &DomSnapshot, tag: &str) -> css::SnapshotId {
    dom.nodes_in_document_order()
        .find(|id| dom.node(*id).and_then(css::NodeRef::tag) == Some(tag))
        .expect("the document has the requested tag")
}

// ---- collection itself ---------------------------------------------------

#[test]
fn collection_finds_the_style_element_and_the_style_attribute_separately() {
    let (tree, root) = document("p { color: #0000ff }", Some("margin: 2px"));
    let dom = snapshot(&tree, root);
    let sheets = collect_style_sheets(&dom).expect("readable");

    assert_eq!(sheets.len(), 1, "one author rule");
    assert_eq!(sheets.inline().len(), 1, "one inline block");
    assert!(sheets.notes().is_empty(), "nothing needed recovering");
}

#[test]
fn a_document_with_no_css_collects_an_empty_set() {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let html = child(&mut tree, root, "html");
    child(&mut tree, html, "body");

    let sheets = collect_style_sheets(&snapshot(&tree, root)).expect("readable");
    assert_eq!(sheets, StyleSheetSet::new());
}

#[test]
fn an_empty_style_attribute_records_no_inline_block() {
    let (tree, root) = document("", Some(""));
    let sheets = collect_style_sheets(&snapshot(&tree, root)).expect("readable");
    assert!(sheets.inline().is_empty());
}

#[test]
fn the_style_element_itself_is_never_painted() {
    let (dom, styled) = resolve("style { color: #ff0000 }", None);
    let element = dom
        .nodes_in_document_order()
        .find(|id| dom.node(*id).and_then(css::NodeRef::tag) == Some("style"))
        .expect("the style element");

    assert!(
        styled
            .node(element)
            .expect("styled")
            .style()
            .display()
            .is_none(),
        "the UA `display: none` for `<style>` is the base an author rule that \
         sets only `color` does not overwrite"
    );
}

#[test]
fn a_recovered_rule_leaves_a_note_without_costing_the_rules_around_it() {
    let (tree, root) = document(
        "p { color: #0000ff } :has(x) { color: #ff0000 } p { margin: 3px }",
        None,
    );
    let dom = snapshot(&tree, root);
    let sheets = collect_style_sheets(&dom).expect("readable");

    assert_eq!(sheets.len(), 2, "both `p` rules survive");
    assert_eq!(sheets.notes().len(), 1, "the `:has()` rule left one note");

    let styled = UaCascade::new().resolve(&dom, &sheets).expect("resolves");
    let paragraph = dom
        .nodes_in_document_order()
        .find(|id| dom.node(*id).and_then(css::NodeRef::tag) == Some("p"))
        .expect("a paragraph");
    let style = styled.node(paragraph).expect("styled").style();

    assert_eq!(style.color(), BLUE);
    assert_eq!(style.margin(), LengthEdges::uniform(Length::Pixels(3.0)));
}

#[test]
fn the_cascade_is_deterministic_across_repeated_resolutions() {
    let (tree, root) = document(
        "#first { color: #ff0000 } .lead { color: #008000 } p { color: #0000ff }",
        Some("margin: 1px"),
    );
    let dom = snapshot(&tree, root);
    let sheets = collect_style_sheets(&dom).expect("readable");
    let first = UaCascade::new().resolve(&dom, &sheets).expect("resolves");

    for _run in 0..100 {
        let again = UaCascade::new().resolve(&dom, &sheets).expect("resolves");
        assert_eq!(again, first, "the sort key is total (PRD-007:100)");
    }
}
