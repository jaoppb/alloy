//! Cascade semantics that only show up across a whole resolution — the chain
//! `dom::DomTree → snapshot → collect_style_sheets → UaCascade::resolve`:
//!
//! - custom properties and `var()` (CSS Variables L1): inheritance, fallback,
//!   case-sensitivity, `style=` precedence, invalid at computed-value time →
//!   `unset`, cycles, and the substitution expansion cap;
//! - logical properties (CSS Logical L1 §4): the element's **final**
//!   `writing-mode` / `direction` decide the mapping whatever the declaration
//!   order, `initial` / `inherit` reach every logical property, and the
//!   flow-relative box values are not inherited;
//! - text values relative to the right thing: `bolder` / `lighter` against the
//!   parent's weight (CSS Fonts 4 §2.2), and `em` / `%` line heights and
//!   spacings inherited as absolute lengths (CSS 2.1 §10.8.1, CSS Text L3 §8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use css::domain::computed::logical::Direction;
use css::domain::computed::text_advance::FontWeight;
use css::domain::computed::variables::{CustomPropertiesMap, VariableError, VariableName};
use css::infrastructure::cascade::variable_values::{
    MAX_SUBSTITUTED_BYTES, parse_custom_properties_block, resolve_variable, substitute_variables,
};
use css::{
    CascadeResolver, ComputedStyle, CssColor, DomSnapshot, Length, Sizing, StyledTree, UaCascade,
    collect_style_sheets, snapshot,
};
use graphics::Au;

const RED: CssColor = CssColor::rgb(0xFF, 0x00, 0x00);
const BLUE: CssColor = CssColor::rgb(0x00, 0x00, 0xFF);
const GREEN: CssColor = CssColor::rgb(0x00, 0x80, 0x00);

/// How long the hostile doubling chain may take to be rejected — generous for
/// a debug build on a loaded CI runner, yet orders of magnitude below what
/// expanding `2^30` copies would cost.
const HOSTILE_CHAIN_BUDGET: Duration = Duration::from_secs(1);

/// How deep the hostile doubling chain goes: thirty levels is `2^30` copies
/// of the leaf without the expansion cap.
const HOSTILE_CHAIN_DEPTH: usize = 30;

const fn au(pixels: i32) -> Au {
    Au::from_whole_px(pixels).unwrap()
}

/// The `style=` attributes of the two nested elements a test may decorate.
#[derive(Clone, Copy, Default)]
struct InlineStyles<'text> {
    outer: Option<&'text str>,
    inner: Option<&'text str>,
}

/// The resolved document:
///
/// ```html
/// <html><body>
///   <style>{sheet}</style>
///   <h1>Title</h1>
///   <div id="outer" style="{outer}"><p id="inner" style="{inner}"><span>Hi</span></p></div>
/// </body></html>
/// ```
struct Resolved {
    dom: DomSnapshot,
    styled: StyledTree,
}

impl Resolved {
    fn new(sheet: &str, inline: InlineStyles<'_>) -> Self {
        let (tree, root) = document(sheet, inline);
        let dom = snapshot(&tree, root);
        let sheets = collect_style_sheets(&dom).expect("the document's CSS is readable");
        let styled = UaCascade::new()
            .resolve(&dom, &sheets)
            .expect("the cascade resolves");
        Self { dom, styled }
    }

    /// The computed style of the first element named `tag`.
    fn style(&self, tag: &str) -> ComputedStyle {
        let tag_name = dom::TagName::new(tag).expect("a valid tag name");
        let id = self
            .dom
            .nodes_in_document_order()
            .find(|id| self.dom.node(*id).and_then(css::NodeRef::tag) == Some(&tag_name))
            .expect("the document has the requested tag");
        *self.styled.node(id).expect("the node is styled").style()
    }
}

fn style_of(sheet: &str, tag: &str) -> ComputedStyle {
    Resolved::new(sheet, InlineStyles::default()).style(tag)
}

fn document(sheet: &str, inline: InlineStyles<'_>) -> (dom::DomTree, dom::NodeId) {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let html = child(&mut tree, root, "html");
    let body = child(&mut tree, html, "body");
    let style = child(&mut tree, body, "style");
    text(&mut tree, style, sheet);
    let heading = child(&mut tree, body, "h1");
    text(&mut tree, heading, "Title");
    let outer = child(&mut tree, body, "div");
    decorate(&mut tree, outer, "outer", inline.outer);
    let inner = child(&mut tree, outer, "p");
    decorate(&mut tree, inner, "inner", inline.inner);
    let leaf = child(&mut tree, inner, "span");
    text(&mut tree, leaf, "Hi");
    (tree, root)
}

fn decorate(tree: &mut dom::DomTree, node: dom::NodeId, id: &str, inline: Option<&str>) {
    attribute_of(tree, node, "id", id);
    let Some(declarations) = inline else {
        return;
    };
    attribute_of(tree, node, "style", declarations);
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

/// `--v0: x; --v1: var(--v0) var(--v0); …; --v{depth}: …` — every level twice
/// the length of the one below it.
fn doubling_chain(depth: usize) -> String {
    let mut declarations = String::from("--v0: x;");
    for (below, level) in (1..=depth).enumerate() {
        write!(
            declarations,
            " --v{level}: var(--v{below}) var(--v{below});"
        )
        .unwrap();
    }
    declarations
}

// ---- custom properties and var() (CSS Variables L1) ------------------------

#[test]
fn a_custom_property_on_the_root_reaches_a_descendant_through_var() {
    assert_eq!(
        style_of("html { --c: red } p { color: var(--c) }", "p").color(),
        RED
    );
}

#[test]
fn a_custom_property_is_inherited_from_an_ancestor_no_rule_selects_twice() {
    let span = style_of("div { --c: blue } span { color: var(--c) }", "span");
    assert_eq!(span.color(), BLUE);
}

#[test]
fn an_undefined_variable_falls_back_to_the_var_fallback() {
    assert_eq!(
        style_of("p { color: var(--missing, blue) }", "p").color(),
        BLUE
    );
}

#[test]
fn custom_property_names_are_case_sensitive() {
    let sheet = "div { --C: red; --c: blue } p { color: var(--c) } span { color: var(--C) }";
    let resolved = Resolved::new(sheet, InlineStyles::default());
    assert_eq!(resolved.style("p").color(), BLUE);
    assert_eq!(resolved.style("span").color(), RED);
}

#[test]
fn an_inline_custom_property_overrides_the_sheet() {
    let inline = InlineStyles {
        inner: Some("--c: green"),
        ..InlineStyles::default()
    };
    let resolved = Resolved::new("p { --c: red; color: var(--c) }", inline);
    assert_eq!(resolved.style("p").color(), GREEN);
}

#[test]
fn an_unresolvable_var_on_an_inherited_property_computes_to_the_parent_value() {
    // `p { color: red }` loses to the higher-specificity `#inner` rule even
    // though that rule's value is invalid at computed-value time: the
    // property becomes `unset`, i.e. inherits the `div`'s blue.
    let sheet = "div { color: blue } p { color: red } #inner { color: var(--missing) }";
    assert_eq!(style_of(sheet, "p").color(), BLUE);
}

#[test]
fn an_unresolvable_var_on_a_non_inherited_property_computes_to_initial() {
    let sheet = "p { width: 50px } #inner { width: var(--missing) }";
    assert_eq!(style_of(sheet, "p").width(), Sizing::Auto);
}

#[test]
fn a_substituted_value_that_does_not_parse_is_invalid_at_computed_value_time() {
    let sheet = "p { --word: banana; width: 50px } #inner { width: var(--word) }";
    assert_eq!(style_of(sheet, "p").width(), Sizing::Auto);
}

#[test]
fn a_reference_cycle_makes_the_property_unset() {
    let sheet = "div { color: blue } \
                 p { --a: var(--b); --b: var(--a); width: 50px; color: red } \
                 #inner { width: var(--a); color: var(--b) }";
    let paragraph = style_of(sheet, "p");
    assert_eq!(paragraph.width(), Sizing::Auto);
    assert_eq!(paragraph.color(), BLUE);
}

#[test]
fn the_initial_keyword_on_a_custom_property_removes_it() {
    let sheet = "div { --c: red } p { --c: initial; color: var(--c, blue) }";
    assert_eq!(style_of(sheet, "p").color(), BLUE);
}

#[test]
fn a_custom_property_substitutes_its_references_where_it_is_declared() {
    // CSS Variables L1 §2: the computed value of `--a` on `html` is `1px`, and
    // that computed value is what the `p` inherits — the `div`'s own `--b`
    // does not reach back into it.
    let sheet = "html { --a: var(--b); --b: 1px } div { --b: 2px } p { width: var(--a) }";
    assert_eq!(
        style_of(sheet, "p").width(),
        Sizing::Fixed(Length::Pixels(1.0))
    );
}

#[test]
fn a_doubling_reference_chain_fails_fast_with_the_expansion_limit() {
    let map: CustomPropertiesMap =
        parse_custom_properties_block(&doubling_chain(HOSTILE_CHAIN_DEPTH)).unwrap();
    let deepest = VariableName::new(&format!("--v{HOSTILE_CHAIN_DEPTH}")).unwrap();
    let started = Instant::now();
    let resolution = resolve_variable(&deepest, &map);
    let substitution = substitute_variables(&format!("var({deepest})"), &map);
    assert!(started.elapsed() < HOSTILE_CHAIN_BUDGET);
    let expected = VariableError::ExpansionLimit {
        limit_bytes: MAX_SUBSTITUTED_BYTES,
    };
    assert_eq!(resolution, Err(expected.clone()));
    assert_eq!(substitution, Err(expected));
}

#[test]
fn a_doubling_reference_chain_in_a_sheet_leaves_the_property_unset() {
    let sheet = format!(
        "p {{ width: 50px }} #inner {{ {} width: var(--v{HOSTILE_CHAIN_DEPTH}) }}",
        doubling_chain(HOSTILE_CHAIN_DEPTH)
    );
    let started = Instant::now();
    let paragraph = style_of(&sheet, "p");
    assert!(started.elapsed() < HOSTILE_CHAIN_BUDGET);
    assert_eq!(paragraph.width(), Sizing::Auto);
}

#[test]
fn a_shallow_chain_below_the_cap_still_resolves() {
    let map = parse_custom_properties_block(&doubling_chain(3)).unwrap();
    let deepest = VariableName::new("--v3").unwrap();
    assert_eq!(resolve_variable(&deepest, &map).unwrap(), "x x x x x x x x");
}

// ---- logical properties (CSS Logical L1) -----------------------------------

#[test]
fn a_logical_margin_maps_through_a_direction_declared_after_it() {
    let outer = style_of("div { margin-inline-start: 10px; direction: rtl }", "div");
    assert_eq!(outer.margin().right(), Length::Pixels(10.0));
    assert_eq!(outer.margin().left(), Length::ZERO);
}

#[test]
fn a_logical_margin_maps_through_a_direction_from_a_stronger_rule() {
    let outer = style_of(
        "div { margin-inline-start: 10px } #outer { direction: rtl }",
        "div",
    );
    assert_eq!(outer.margin().right(), Length::Pixels(10.0));
    assert_eq!(outer.margin().left(), Length::ZERO);
}

#[test]
fn writing_mode_initial_takes_effect() {
    let sheet = "div { writing-mode: vertical-rl } \
                 p { writing-mode: initial; margin-block-start: 10px }";
    let paragraph = style_of(sheet, "p");
    assert_eq!(paragraph.margin().top(), Length::Pixels(10.0));
    assert_eq!(paragraph.margin().right(), Length::ZERO);
}

#[test]
fn direction_inherit_takes_effect() {
    let sheet = "div { direction: rtl } p { direction: ltr } \
                 #inner { direction: inherit; margin-inline-start: 10px }";
    let paragraph = style_of(sheet, "p");
    assert_eq!(paragraph.logical().context().direction(), Direction::Rtl);
    assert_eq!(paragraph.margin().right(), Length::Pixels(10.0));
    assert_eq!(paragraph.margin().left(), Length::ZERO);
}

#[test]
fn margin_inline_start_initial_takes_effect() {
    let sheet = "p { margin-inline-start: 10px } #inner { margin-inline-start: initial }";
    let paragraph = style_of(sheet, "p");
    assert_eq!(paragraph.margin().left(), Length::ZERO);
    assert_eq!(paragraph.logical().margin().inline_start(), Length::ZERO);
}

#[test]
fn margin_inline_start_inherit_reads_the_parent_through_its_own_direction() {
    // The parent's inline-start is its left; the right-to-left child's
    // inline-start is its right — `inherit` carries the value across.
    let sheet = "div { margin-inline-start: 7px } \
                 p { direction: rtl; margin-inline-start: inherit }";
    let paragraph = style_of(sheet, "p");
    assert_eq!(paragraph.margin().right(), Length::Pixels(7.0));
    assert_eq!(paragraph.margin().left(), Length::ZERO);
}

#[test]
fn inset_and_size_logical_keywords_take_effect() {
    let sheet = "p { inset-inline-start: 4px; inline-size: 30px } \
                 #inner { inset-inline-start: initial; inline-size: initial }";
    let paragraph = style_of(sheet, "p");
    assert_eq!(paragraph.position().left(), Sizing::Auto);
    assert_eq!(paragraph.width(), Sizing::Auto);
}

#[test]
fn flow_relative_box_values_are_not_inherited_but_the_writing_context_is() {
    let paragraph = style_of("div { direction: rtl; margin-inline-start: 5px }", "p");
    assert_eq!(paragraph.logical().margin().inline_start(), Length::ZERO);
    assert_eq!(paragraph.logical().context().direction(), Direction::Rtl);
}

// ---- text values relative to the right thing --------------------------------

#[test]
fn bolder_is_relative_to_the_parent_weight_not_the_ua_bold() {
    let heading = style_of("h1 { font-weight: bolder }", "h1");
    assert_eq!(heading.text_advance().font_weight(), FontWeight::BOLD);
}

#[test]
fn lighter_is_relative_to_the_parent_weight_not_the_ua_bold() {
    let heading = style_of("h1 { font-weight: lighter }", "h1");
    assert_eq!(
        heading.text_advance().font_weight(),
        FontWeight::new(100).unwrap()
    );
}

#[test]
fn an_em_line_height_is_inherited_as_an_absolute_length() {
    let sheet = "div { font-size: 10px; line-height: 2em } span { font-size: 20px }";
    let span = style_of(sheet, "span");
    assert_eq!(
        span.text_advance().line_height().resolve_to_au(au(20)),
        Some(au(20))
    );
}

#[test]
fn a_percentage_line_height_is_inherited_as_an_absolute_length() {
    let sheet = "div { font-size: 10px; line-height: 150% } span { font-size: 20px }";
    let span = style_of(sheet, "span");
    assert_eq!(
        span.text_advance().line_height().resolve_to_au(au(20)),
        Some(au(15))
    );
}

#[test]
fn a_unitless_line_height_still_scales_with_the_descendant_font_size() {
    let sheet = "div { font-size: 10px; line-height: 2 } span { font-size: 20px }";
    let span = style_of(sheet, "span");
    assert_eq!(
        span.text_advance().line_height().resolve_to_au(au(20)),
        Some(au(40))
    );
}

#[test]
fn em_spacings_are_inherited_as_absolute_lengths() {
    let sheet = "div { font-size: 10px; letter-spacing: 0.5em; word-spacing: 1em } \
                 span { font-size: 20px }";
    let text_advance = style_of(sheet, "span").text_advance();
    assert_eq!(
        text_advance.letter_spacing().resolve_to_au(au(20)),
        Some(au(5))
    );
    assert_eq!(
        text_advance.word_spacing().resolve_to_au(au(20)),
        Some(au(10))
    );
}

#[test]
fn an_em_line_height_resolves_against_the_element_own_computed_font_size() {
    // The `p`'s font size is `2em` of the `div`'s `10px`: `20px`, so its own
    // `1em` line height computes to `20px`, which the `span` inherits.
    let sheet = "div { font-size: 10px } p { font-size: 2em; line-height: 1em } \
                 span { font-size: 40px }";
    let span = style_of(sheet, "span");
    assert_eq!(
        span.text_advance().line_height().resolve_to_au(au(40)),
        Some(au(20))
    );
}
