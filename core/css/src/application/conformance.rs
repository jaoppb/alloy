//! A resolver- and layout-engine-agnostic conformance suite — `ADR-0011`
//! item 6, guarding the `css-conformance` target of `PRD-007:86`.
//!
//! Ordinary library code, not `#[cfg(test)]`, so an adapter can call it from
//! its own `tests/` — the same shape and reason as
//! `core/graphics/src/application/conformance.rs` and
//! `core/engine/src/conformance.rs`, which their adapters both run.
//!
//! ```text
//! #[test]
//! fn my_adapters_pass_conformance() {
//!     css::conformance::run_css_conformance(&MyCascade::new(), &MyLayout::new());
//! }
//! ```
//!
//! What it pins is the **port contract** — determinism (`PRD-007:52`, `:79-80`),
//! whole-tree granularity (`PRD-007:78`), the no-foreign-type rule
//! (`PRD-007:83-84`), and graceful handling of an empty document. What it does
//! not pin is a particular adapter's rules: `UaCascade` and `MockCascadeResolver`
//! both pass it.

// An assertion suite that happens to be `pub` (so adapters can call it from
// their `tests/`) rather than `#[cfg(test)]`: it panics on the first violation
// by design. Same carve-out, same reason, as
// `core/graphics/src/application/conformance.rs:29`.
#![allow(clippy::panic, clippy::expect_used)]

use graphics::Au;

use crate::application::ports::{CascadeResolver, LayoutEngine};
use crate::application::snapshot::snapshot;
use crate::domain::dom_snapshot::DomSnapshot;
use crate::domain::stylesheet_set::StyleSheetSet;
use crate::domain::viewport::ViewportConstraints;

/// How many identical runs the determinism checks demand (`PRD-007:100`).
const DETERMINISM_RUNS: usize = 100;

/// Runs every rule a [`CascadeResolver`] / [`LayoutEngine`] pair must obey.
///
/// Panics on the first violation, naming the rule that was broken.
pub fn run_css_conformance(cascade: &dyn CascadeResolver, layout: &dyn LayoutEngine) {
    check_cascade_is_deterministic(cascade);
    check_layout_is_deterministic(cascade, layout);
    check_the_whole_tree_is_styled_in_one_call(cascade);
    check_no_foreign_type_escapes_the_styled_tree(cascade);
    check_an_empty_document_is_handled(cascade, layout);
}

const fn empty_sheets() -> StyleSheetSet {
    StyleSheetSet::new()
}

const fn reference_viewport() -> ViewportConstraints {
    ViewportConstraints::new(whole_px(800), whole_px(600))
}

const fn whole_px(pixels: i32) -> Au {
    Au::from_whole_px(pixels).expect("a small pixel count is always a valid Au")
}

/// `html > body > (h1 > "Title") + (p > "Hello, world")` — enough nodes for
/// inheritance, `display: none` (none here) and multiple siblings to matter.
fn fixture_snapshot() -> DomSnapshot {
    let mut tree = dom::DomTree::new();
    let document = tree.document();
    let html = append_element(&mut tree, document, "html");
    let body = append_element(&mut tree, html, "body");
    let heading = append_element(&mut tree, body, "h1");
    append_text(&mut tree, heading, "Title");
    let paragraph = append_element(&mut tree, body, "p");
    append_text(&mut tree, paragraph, "Hello, world");
    snapshot(&tree, document)
}

fn append_element(tree: &mut dom::DomTree, parent: dom::NodeId, name: &str) -> dom::NodeId {
    let element = tree.create_element(valid_tag(name));
    tree.append_child(parent, element)
        .expect("appending a fresh element to a container cannot fail");
    element
}

fn append_text(tree: &mut dom::DomTree, parent: dom::NodeId, content: &str) {
    let text = tree.create_text(dom::TextContent::new(content));
    tree.append_child(parent, text)
        .expect("appending a fresh text node to a container cannot fail");
}

fn valid_tag(name: &str) -> dom::TagName {
    dom::TagName::new(name).expect("the fixture uses only valid tag names")
}

fn check_cascade_is_deterministic(cascade: &dyn CascadeResolver) {
    let dom = fixture_snapshot();
    let sheets = empty_sheets();
    let first = cascade
        .resolve(&dom, &sheets)
        .expect("the reference input must resolve");
    for run in 1..DETERMINISM_RUNS {
        let again = cascade
            .resolve(&dom, &sheets)
            .expect("resolve must not begin failing partway through");
        assert_eq!(
            first, again,
            "cascade run {run} diverged from the first (PRD-007:52)"
        );
    }
}

fn check_layout_is_deterministic(cascade: &dyn CascadeResolver, layout: &dyn LayoutEngine) {
    let dom = fixture_snapshot();
    let styled = cascade
        .resolve(&dom, &empty_sheets())
        .expect("the reference input must resolve");
    let constraints = reference_viewport();
    let first = layout
        .layout(&styled, &constraints)
        .expect("the styled tree must lay out");
    for run in 1..DETERMINISM_RUNS {
        let again = layout
            .layout(&styled, &constraints)
            .expect("layout must not begin failing partway through");
        assert_eq!(
            first, again,
            "layout run {run} diverged from the first (PRD-007:79-80)"
        );
    }
}

fn check_the_whole_tree_is_styled_in_one_call(cascade: &dyn CascadeResolver) {
    // `PRD-007:78`: no per-node callback crosses the seam — one `resolve` call
    // is handed the whole snapshot and returns the whole styled tree.
    let dom = fixture_snapshot();
    let styled = cascade
        .resolve(&dom, &empty_sheets())
        .expect("the reference input must resolve");
    assert_eq!(
        styled.len(),
        dom.len(),
        "one resolve() call must produce one computed style per snapshot node"
    );
}

fn check_no_foreign_type_escapes_the_styled_tree(cascade: &dyn CascadeResolver) {
    // `PRD-007:83-84`: no `core/dom` / `core/graphics` internal type appears in
    // a boundary aggregate's API. Reading a computed value yields `css` types.
    let dom = fixture_snapshot();
    let styled = cascade
        .resolve(&dom, &empty_sheets())
        .expect("the reference input must resolve");
    let root = styled
        .node(styled.root())
        .expect("the root node is always styled");
    accept_css_color(root.style().color());
    accept_css_display(root.style().display());
}

const fn accept_css_color(_: crate::CssColor) {}

const fn accept_css_display(_: crate::Display) {}

fn check_an_empty_document_is_handled(cascade: &dyn CascadeResolver, layout: &dyn LayoutEngine) {
    // `PRD-007:82` spirit: a resolver / layout engine handed almost nothing
    // must not panic — the page still renders.
    let tree = dom::DomTree::new();
    let dom = snapshot(&tree, tree.document());
    let styled = cascade
        .resolve(&dom, &empty_sheets())
        .expect("an empty document must resolve");
    assert_eq!(
        styled.len(),
        dom.len(),
        "the lone Document node is still styled"
    );
    let boxes = layout
        .layout(&styled, &reference_viewport())
        .expect("an empty document must lay out");
    assert!(
        boxes.len() <= styled.len(),
        "layout cannot invent boxes for nodes that do not exist"
    );
}
