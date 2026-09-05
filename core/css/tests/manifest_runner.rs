//! The conformance gate for the declared v0.5 cut
//! (`docs/reports/IMPLEMENTACAO-DETALHADA-V0-5.md` §2.8:350-354).
//!
//! `core/css/tests/data/MANIFEST.md`, the `css::SUPPORTED_PROPERTIES` /
//! `css::SUPPORTED_SELECTORS` registries and **what the parser actually does**
//! are three independent statements of the same cut, and this file fails loudly
//! whenever any two of them drift:
//!
//! 1. manifest ⇄ registry, in both directions;
//! 2. registry ⇄ probe table, in both directions — so a token can never be
//!    added to the registry without a probe that exercises it;
//! 3. probe ⇄ parser: every listed property really survives a parse *and*
//!    really changes the computed style; every listed selector really parses;
//!    and a battery of forms declared **out** is really refused, each with a
//!    `ParseNote`.
//!
//! There is deliberately **no bless path**. A generated manifest would agree
//! with the code by construction, which is the one thing this gate must not be
//! able to do; `MANIFEST.md` is hand-maintained and the `notes` column carries
//! reasons no generator could invent. B5 reuses this shape for `core/html`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use css::{
    CascadeResolver, ComputedStyle, DomSnapshot, Origin, SnapshotId, StyleSheetSet, UaCascade,
    collect_style_sheets, matches, parse_stylesheet, snapshot,
};

/// What a probe expects of the parser and the matcher.
#[derive(Clone, Copy)]
enum SelectorProbe {
    /// A selector that must parse and must select at least one fixture element.
    Selects(&'static str),
    /// A selector that must parse but is never expected to match: interaction
    /// state has no projection (`PRD-007:35-36`).
    ParsesOnly(&'static str),
    /// A whole sheet whose rule must survive carrying a media condition.
    Gated(&'static str),
}

/// One probe per `css::SUPPORTED_SELECTORS` entry, cross-checked both ways.
const SELECTOR_PROBES: [(&str, SelectorProbe); 19] = [
    ("E", SelectorProbe::Selects("p")),
    ("*", SelectorProbe::Selects("*")),
    (".class", SelectorProbe::Selects(".lead")),
    ("#id", SelectorProbe::Selects("#first")),
    ("[attr]", SelectorProbe::Selects("[data-role]")),
    ("[attr=value]", SelectorProbe::Selects("[data-role=intro]")),
    ("E, F", SelectorProbe::Selects("q, p")),
    ("E F", SelectorProbe::Selects("body p")),
    ("E > F", SelectorProbe::Selects("body > p")),
    ("E + F", SelectorProbe::Selects("h1 + p")),
    ("E ~ F", SelectorProbe::Selects("h1 ~ p")),
    (":hover", SelectorProbe::ParsesOnly("p:hover")),
    (":active", SelectorProbe::ParsesOnly("p:active")),
    (":focus", SelectorProbe::ParsesOnly("p:focus")),
    (":first-child", SelectorProbe::Selects(":first-child")),
    (":last-child", SelectorProbe::Selects(":last-child")),
    (":nth-child()", SelectorProbe::Selects("p:nth-child(2n)")),
    (
        "@media (min-width)",
        SelectorProbe::Gated("@media (min-width: 1px) { p { color: red } }"),
    ),
    (
        "@media (max-width)",
        SelectorProbe::Gated("@media (max-width: 9999px) { p { color: red } }"),
    ),
];

/// One probe per `css::SUPPORTED_PROPERTIES` entry: a value inside the cut that
/// is **different** from the value the UA sheet gives the fixture paragraph, so
/// "the cascade honoured it" is observable rather than assumed.
const PROPERTY_PROBES: [(&str, &str); 33] = [
    ("display", "inline"),
    ("color", "#ff0000"),
    ("background-color", "silver"),
    ("margin", "1px 2px 3px 4px"),
    ("margin-top", "5px"),
    ("margin-right", "6px"),
    ("margin-bottom", "7px"),
    ("margin-left", "8px"),
    ("padding", "1px 2px"),
    ("padding-top", "9px"),
    ("padding-right", "10px"),
    ("padding-bottom", "11px"),
    ("padding-left", "12px"),
    ("font-size", "20px"),
    ("border-width", "1px 2px 3px 4px"),
    ("border-top-width", "2px"),
    ("border-right-width", "3px"),
    ("border-bottom-width", "4px"),
    ("border-left-width", "5px"),
    ("width", "100px"),
    ("height", "50px"),
    ("box-sizing", "border-box"),
    ("text-align", "center"),
    ("white-space", "pre"),
    ("flex-direction", "column"),
    ("flex-wrap", "wrap"),
    ("justify-content", "center"),
    ("align-items", "center"),
    ("align-content", "center"),
    ("align-self", "center"),
    ("flex-grow", "2"),
    ("flex-shrink", "0"),
    ("flex-basis", "10px"),
];

/// Forms the cut declares **out**. Each must leave zero rules and at least one
/// note — refused, never ignored (§2.8:350-354).
const REFUSED_SHEETS: [&str; 12] = [
    ":has(p) { color: red }",
    "p::before { color: red }",
    "p::after { color: red }",
    "p:not(.lead) { color: red }",
    "p:nth-of-type(1) { color: red }",
    "svg|rect { color: red }",
    "[href^=\"http\"] { color: red }",
    "[class~=\"lead\"] { color: red }",
    "@supports (display: flex) { p { color: red } }",
    "@font-face { font-family: serif }",
    "@import url(other.css);",
    "@keyframes spin { from { color: red } }",
];

/// Properties the cut declares out. Each must be dropped on its own, with a
/// note, leaving the rule and its other declarations standing.
const REFUSED_PROPERTIES: [&str; 4] = ["float", "position", "border", "z-index"];

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("MANIFEST.md")
}

fn read_manifest() -> String {
    std::fs::read_to_string(manifest_path())
        .unwrap_or_else(|error| panic!("could not read {} ({error})", manifest_path().display()))
}

fn names(entries: impl IntoIterator<Item = &'static str>) -> BTreeSet<String> {
    entries.into_iter().map(str::to_owned).collect()
}

// ---- manifest table parsing ----------------------------------------------

/// The backtick-quoted first cell of every row of the **first** table under
/// `## <section>`. The "declared out" table that follows it is prose for a
/// human, not a registry, so the scan stops at the blank line after the first.
fn parse_section(markdown: &str, section: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut inside = false;
    for line in markdown.lines() {
        inside = section_state(line, section, inside, &tokens);
        collect_row(line, inside, &mut tokens);
    }
    tokens
}

/// Whether the scanner is inside the section's first table after `line`.
fn section_state(line: &str, section: &str, inside: bool, tokens: &BTreeSet<String>) -> bool {
    if let Some(heading) = line.strip_prefix("## ") {
        return heading.trim() == section;
    }
    if inside && !tokens.is_empty() && !line.trim_start().starts_with('|') {
        return false;
    }
    inside
}

fn collect_row(line: &str, inside: bool, tokens: &mut BTreeSet<String>) {
    let Some(token) = row_token(line).filter(|_| inside) else {
        return;
    };
    tokens.insert(token);
}

/// The first cell of a table row, when it is a backticked token. The header row
/// (`| token | since | notes |`) and the separator row have no backticks, so
/// both fall out here without a special case.
fn row_token(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let cell = trimmed.strip_prefix('|')?.split('|').next()?.trim();
    let quoted = cell.strip_prefix('`')?.strip_suffix('`')?;
    Some(quoted.replace("\\|", "|"))
}

/// Every way two statements of the cut disagree, in both directions.
fn divergence(left: &BTreeSet<String>, right: &BTreeSet<String>, subject: &str) -> Vec<String> {
    let missing = right
        .difference(left)
        .map(|name| format!("{subject}: `{name}` is in the code but not in the manifest"));
    let extra = left
        .difference(right)
        .map(|name| format!("{subject}: `{name}` is in the manifest but not in the code"));
    missing.chain(extra).collect()
}

// ---- the fixture the probes run against ----------------------------------

/// `<html><body><h1>T</h1><p id="first" class="lead" data-role="intro">Hi</p></body></html>`
fn fixture() -> DomSnapshot {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let html = element(&mut tree, root, "html");
    let body = element(&mut tree, html, "body");

    let heading = element(&mut tree, body, "h1");
    text(&mut tree, heading, "T");

    let paragraph = element(&mut tree, body, "p");
    attribute(&mut tree, paragraph, "id", "first");
    attribute(&mut tree, paragraph, "class", "lead");
    attribute(&mut tree, paragraph, "data-role", "intro");
    text(&mut tree, paragraph, "Hi");

    snapshot(&tree, root)
}

fn element(tree: &mut dom::DomTree, parent: dom::NodeId, name: &str) -> dom::NodeId {
    let node = tree.create_element(dom::TagName::new(name).unwrap());
    tree.append_child(parent, node).unwrap();
    node
}

fn attribute(tree: &mut dom::DomTree, node: dom::NodeId, name: &str, value: &str) {
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

fn paragraph_of(dom: &DomSnapshot) -> SnapshotId {
    dom.nodes_in_document_order()
        .find(|id| dom.node(*id).and_then(css::NodeRef::tag) == Some("p"))
        .expect("the fixture has a paragraph")
}

fn author_sheet(source: &str) -> StyleSheetSet {
    parse_stylesheet(source, Origin::Author).unwrap_or_else(|error| {
        panic!("`{source}` should be readable, got {error}");
    })
}

/// The computed style the fixture paragraph ends up with under `source`.
fn paragraph_style(source: &str) -> ComputedStyle {
    let dom = fixture();
    let styled = UaCascade::new()
        .resolve(&dom, &author_sheet(source))
        .expect("the cascade resolves");
    *styled
        .node(paragraph_of(&dom))
        .expect("the paragraph is styled")
        .style()
}

// ---- 1. manifest ⇄ registry ----------------------------------------------

#[test]
fn the_manifest_and_the_support_registries_name_the_same_cut() {
    let markdown = read_manifest();
    let gaps: Vec<String> = divergence(
        &parse_section(&markdown, "Properties"),
        &names(css::SUPPORTED_PROPERTIES),
        "property",
    )
    .into_iter()
    .chain(divergence(
        &parse_section(&markdown, "Selectors"),
        &names(css::SUPPORTED_SELECTORS),
        "selector",
    ))
    .collect();

    assert!(
        gaps.is_empty(),
        "MANIFEST.md and core/css disagree:\n{}",
        gaps.join("\n")
    );
}

// ---- 2. registry ⇄ probe table -------------------------------------------

#[test]
fn every_registry_entry_has_exactly_one_probe() {
    let property_gaps = divergence(
        &names(PROPERTY_PROBES.map(|(token, _)| token)),
        &names(css::SUPPORTED_PROPERTIES),
        "property probe",
    );
    let selector_gaps = divergence(
        &names(SELECTOR_PROBES.map(|(token, _)| token)),
        &names(css::SUPPORTED_SELECTORS),
        "selector probe",
    );
    let gaps: Vec<String> = property_gaps.into_iter().chain(selector_gaps).collect();

    assert!(
        gaps.is_empty(),
        "add the missing row to the probe table in this file:\n{}",
        gaps.join("\n")
    );
}

// ---- 3. probe ⇄ parser ---------------------------------------------------

#[test]
fn every_listed_property_parses_and_changes_the_computed_style() {
    let baseline = paragraph_style("");
    for (property, value) in PROPERTY_PROBES {
        let source = format!("p {{ {property}: {value} }}");
        let sheets = author_sheet(&source);

        assert_eq!(sheets.len(), 1, "`{source}` must produce one rule");
        assert!(
            sheets.notes().is_empty(),
            "`{source}` must parse without recovery, got {:?}",
            sheets.notes().iter().next().map(ToString::to_string)
        );
        assert_ne!(
            paragraph_style(&source),
            baseline,
            "`{property}: {value}` is listed but the cascade ignores it"
        );
    }
}

#[test]
fn every_listed_selector_parses_and_selects_what_it_claims() {
    let dom = fixture();
    for (token, probe) in SELECTOR_PROBES {
        run_selector_probe(token, probe, &dom);
    }
}

fn run_selector_probe(token: &str, probe: SelectorProbe, dom: &DomSnapshot) {
    match probe {
        SelectorProbe::Selects(selector) => assert_selects(token, selector, dom),
        SelectorProbe::ParsesOnly(selector) => assert_parses_only(token, selector, dom),
        SelectorProbe::Gated(sheet) => assert_gated(token, sheet),
    }
}

fn assert_selects(token: &str, selector: &str, dom: &DomSnapshot) {
    assert!(
        selected_count(selector, dom) > 0,
        "`{token}` is listed, and its probe `{selector}` selects nothing"
    );
}

fn assert_parses_only(token: &str, selector: &str, dom: &DomSnapshot) {
    assert_eq!(
        selected_count(selector, dom),
        0,
        "`{token}` is documented as parsing without ever matching"
    );
}

fn assert_gated(token: &str, sheet: &str) {
    let sheets = author_sheet(sheet);
    assert_eq!(sheets.len(), 1, "`{token}` must keep its rule");
    assert!(sheets.notes().is_empty(), "`{token}` must parse cleanly");
    assert!(
        sheets.rules().all(|(_, rule)| !rule.media().is_always()),
        "`{token}` must leave its rule gated for the producer to discharge"
    );
}

/// How many fixture elements `selector` selects, after a real parse.
fn selected_count(selector: &str, dom: &DomSnapshot) -> usize {
    let sheets = author_sheet(&format!("{selector} {{ color: red }}"));
    assert_eq!(sheets.len(), 1, "`{selector}` must parse as one rule");
    assert!(sheets.notes().is_empty(), "`{selector}` must parse cleanly");
    let list = sheets
        .rules()
        .next()
        .map(|(_, rule)| rule.selectors().clone())
        .expect("one rule");
    dom.nodes_in_document_order()
        .filter(|id| node_is_selected(&list, *id, dom))
        .count()
}

fn node_is_selected(list: &css::SelectorList, id: SnapshotId, dom: &DomSnapshot) -> bool {
    let Some(node) = dom.node(id) else {
        return false;
    };
    list.iter().any(|selector| matches(selector, node, dom))
}

#[test]
fn every_form_declared_out_is_refused_with_a_note() {
    for source in REFUSED_SHEETS {
        let sheets = author_sheet(source);
        assert!(
            sheets.is_empty(),
            "`{source}` is declared out of the cut but produced a rule"
        );
        assert!(
            !sheets.notes().is_empty(),
            "`{source}` was dropped without a note — a silent shrinkage of the cut"
        );
    }
}

#[test]
fn every_property_declared_out_drops_only_its_own_declaration() {
    for property in REFUSED_PROPERTIES {
        let source = format!("p {{ {property}: 1px; color: red }}");
        let sheets = author_sheet(&source);
        let (_, rule) = sheets.rules().next().expect("the rule survives");

        assert_eq!(
            rule.declarations().len(),
            1,
            "`{property}` must be dropped and `color` must survive"
        );
        assert!(rule.declarations().last_of("color").is_some());
        assert!(!sheets.notes().is_empty(), "`{property}` must leave a note");
    }
}

#[test]
fn the_cut_holds_through_the_document_collection_path_too() {
    let dom = fixture();
    let collected = collect_style_sheets(&dom).expect("a document with no CSS is readable");
    assert!(
        collected.is_empty(),
        "the fixture carries no `<style>` and no `style=`"
    );
}

// ---- the gate's own machinery --------------------------------------------

#[test]
fn divergence_is_reported_in_both_directions() {
    let manifest = names(["display", "color"]);
    let code = names(["display", "margin"]);
    let gaps = divergence(&manifest, &code, "property");

    assert!(
        gaps.iter()
            .any(|line| line.contains("`margin` is in the code")),
        "an unlisted code capability must be reported"
    );
    assert!(
        gaps.iter()
            .any(|line| line.contains("`color` is in the manifest")),
        "an unsupported manifest entry must be reported"
    );
    assert!(divergence(&manifest, &manifest, "property").is_empty());
}

#[test]
fn the_table_parser_reads_only_the_first_table_of_a_section() {
    let markdown = "## Properties\n\n\
         | token | since | notes |\n\
         | ----- | ----- | ----- |\n\
         | `kept` | B1 | in the registry |\n\n\
         Prose that ends the table.\n\n\
         | form | why |\n\
         | ---- | --- |\n\
         | `declared-out` | not a registry entry |\n";

    assert_eq!(
        parse_section(markdown, "Properties"),
        names(["kept"]),
        "the `declared out` table is documentation, not a second registry"
    );
    assert!(parse_section(markdown, "Selectors").is_empty());
}
