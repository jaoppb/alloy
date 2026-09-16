//! Guards the selector engine: that every form of the v0.5 cut
//! (`relatório §2.8:342-345`) parses, weighs and matches the nodes it should —
//! and that every form declared out is **refused**, not quietly accepted.
//!
//! Matching is asserted against a hand-built `DomSnapshot` rather than a mock,
//! because a combinator is a statement about tree shape and only a real tree can
//! falsify it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use css::{
    ComplexSelector, DomSnapshot, Origin, SelectorList, SnapshotId, Specificity, matches,
    parse_stylesheet, snapshot, strongest_match,
};

/// ```text
/// 0 #document
/// 1 └ html
/// 2   └ body#page
/// 3     ├ h1.title            "A"
/// 5     ├ p.lead.wide[data-role=intro]  "B"
/// 7     ├ p                   "C"
/// 9     ├ div
/// 10    │ └ span.lead         "D"
/// 12    └ p[lang=en]          "E"
/// ```
fn fixture() -> DomSnapshot {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let html = child(&mut tree, root, "html");
    let body = child(&mut tree, html, "body");
    attribute(&mut tree, body, "id", "page");

    let heading = child(&mut tree, body, "h1");
    attribute(&mut tree, heading, "class", "title");
    text(&mut tree, heading, "A");

    let lead = child(&mut tree, body, "p");
    attribute(&mut tree, lead, "class", "lead wide");
    attribute(&mut tree, lead, "data-role", "intro");
    text(&mut tree, lead, "B");

    let plain = child(&mut tree, body, "p");
    text(&mut tree, plain, "C");

    let wrapper = child(&mut tree, body, "div");
    let nested = child(&mut tree, wrapper, "span");
    attribute(&mut tree, nested, "class", "lead");
    text(&mut tree, nested, "D");

    let last = child(&mut tree, body, "p");
    attribute(&mut tree, last, "lang", "en");
    text(&mut tree, last, "E");

    snapshot(&tree, root)
}

fn child(tree: &mut dom::DomTree, parent: dom::NodeId, name: &str) -> dom::NodeId {
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

/// The selector list of `source`, which must have parsed as one rule.
fn selectors(source: &str) -> SelectorList {
    let sheets = parse_stylesheet(&format!("{source} {{ color: red }}"), Origin::Author)
        .expect("the probe is not hostile");
    let (_, rule) = sheets
        .rules()
        .next()
        .unwrap_or_else(|| panic!("`{source}` should have parsed as a rule"));
    rule.selectors().clone()
}

fn only(source: &str) -> ComplexSelector {
    selectors(source)
        .iter()
        .next()
        .expect("one selector")
        .clone()
}

/// The snapshot indices `source` selects, in document order.
fn selected(source: &str, dom: &DomSnapshot) -> Vec<usize> {
    let list = selectors(source);
    dom.nodes_in_document_order()
        .filter(|id| node_is_selected(&list, *id, dom))
        .map(SnapshotId::index)
        .collect()
}

fn node_is_selected(list: &SelectorList, id: SnapshotId, dom: &DomSnapshot) -> bool {
    let Some(node) = dom.node(id) else {
        return false;
    };
    list.iter().any(|selector| matches(selector, node, dom))
}

/// The rule the parser refused, as the note it left behind.
fn refusal(source: &str) -> String {
    let sheets = parse_stylesheet(&format!("{source} {{ color: red }}"), Origin::Author)
        .expect("a refused selector is still a readable source");
    assert!(sheets.is_empty(), "`{source}` must not produce a rule");
    sheets
        .notes()
        .iter()
        .map(|note| note.message().to_owned())
        .next()
        .unwrap_or_else(|| panic!("`{source}` was dropped without a note"))
}

// ---- simple selectors ----------------------------------------------------

#[test]
fn a_type_selector_matches_every_element_of_that_tag() {
    let dom = fixture();
    assert_eq!(selected("p", &dom), vec![5, 7, 12]);
    assert_eq!(selected("h1", &dom), vec![3]);
    assert_eq!(selected("q", &dom), Vec::<usize>::new());
}

#[test]
fn the_universal_selector_matches_every_element_and_no_text_node() {
    let dom = fixture();
    assert_eq!(selected("*", &dom), vec![1, 2, 3, 5, 7, 9, 10, 12]);
}

#[test]
fn a_class_selector_reads_the_whitespace_separated_class_list() {
    let dom = fixture();
    assert_eq!(selected(".lead", &dom), vec![5, 10]);
    assert_eq!(selected(".wide", &dom), vec![5]);
    assert_eq!(selected(".lead.wide", &dom), vec![5]);
    assert_eq!(
        selected(".lea", &dom),
        Vec::<usize>::new(),
        "a class name matches whole, never as a prefix"
    );
}

#[test]
fn an_id_selector_matches_the_id_attribute() {
    let dom = fixture();
    assert_eq!(selected("#page", &dom), vec![2]);
    assert_eq!(selected("body#page", &dom), vec![2]);
    assert_eq!(selected("#absent", &dom), Vec::<usize>::new());
}

#[test]
fn attribute_selectors_cover_existence_and_exact_value() {
    let dom = fixture();
    assert_eq!(selected("[data-role]", &dom), vec![5]);
    assert_eq!(selected("[data-role=intro]", &dom), vec![5]);
    assert_eq!(selected("[data-role=\"intro\"]", &dom), vec![5]);
    assert_eq!(selected("[data-role=other]", &dom), Vec::<usize>::new());
    assert_eq!(selected("[lang]", &dom), vec![12]);
}

#[test]
fn a_selector_list_is_the_union_of_its_selectors() {
    let dom = fixture();
    assert_eq!(selected("h1, div", &dom), vec![3, 9]);
    assert_eq!(selectors("h1, div").len(), 2);
}

// ---- combinators ---------------------------------------------------------

#[test]
fn the_descendant_combinator_reaches_any_depth() {
    let dom = fixture();
    assert_eq!(selected("body p", &dom), vec![5, 7, 12]);
    assert_eq!(selected("body span", &dom), vec![10]);
    assert_eq!(selected("html body span", &dom), vec![10]);
    assert_eq!(selected("p span", &dom), Vec::<usize>::new());
}

#[test]
fn the_child_combinator_reaches_exactly_one_level() {
    let dom = fixture();
    assert_eq!(selected("body > p", &dom), vec![5, 7, 12]);
    assert_eq!(
        selected("body > span", &dom),
        Vec::<usize>::new(),
        "the span is a grandchild, not a child"
    );
    assert_eq!(selected("div > span", &dom), vec![10]);
}

#[test]
fn the_next_sibling_combinator_takes_only_the_immediately_following_element() {
    let dom = fixture();
    assert_eq!(selected("h1 + p", &dom), vec![5]);
    assert_eq!(selected("p + p", &dom), vec![7]);
    assert_eq!(selected("div + p", &dom), vec![12]);
    assert_eq!(
        selected("h1 + div", &dom),
        Vec::<usize>::new(),
        "the div is three element siblings later"
    );
}

#[test]
fn the_subsequent_sibling_combinator_takes_every_later_element_sibling() {
    let dom = fixture();
    assert_eq!(selected("h1 ~ p", &dom), vec![5, 7, 12]);
    assert_eq!(selected("div ~ p", &dom), vec![12]);
    assert_eq!(selected("p ~ h1", &dom), Vec::<usize>::new());
}

#[test]
fn combinators_chain_right_to_left_across_three_compounds() {
    let dom = fixture();
    assert_eq!(selected("html > body p", &dom), vec![5, 7, 12]);
    assert_eq!(selected("body > div > span", &dom), vec![10]);
    assert_eq!(selected("html div span", &dom), vec![10]);
}

// ---- pseudo-classes ------------------------------------------------------

#[test]
fn structural_pseudo_classes_count_element_siblings_one_based() {
    let dom = fixture();
    assert_eq!(selected("body > :first-child", &dom), vec![3]);
    assert_eq!(selected("body > :last-child", &dom), vec![12]);
    assert_eq!(
        selected("body > :nth-child(2)", &dom),
        vec![5],
        "text nodes are not counted (CSS Selectors L4 §6.6.3)"
    );
}

#[test]
fn nth_child_accepts_every_an_plus_b_form_of_the_cut() {
    let dom = fixture();
    // body's element children are h1(3) p(5) p(7) div(9) p(12) — positions 1..5.
    assert_eq!(selected("body > :nth-child(2n+1)", &dom), vec![3, 7, 12]);
    assert_eq!(selected("body > :nth-child(odd)", &dom), vec![3, 7, 12]);
    assert_eq!(selected("body > :nth-child(even)", &dom), vec![5, 9]);
    assert_eq!(selected("body > :nth-child(2n)", &dom), vec![5, 9]);
    assert_eq!(selected("body > :nth-child(-n+3)", &dom), vec![3, 5, 7]);
    assert_eq!(selected("body > :nth-child(n)", &dom), vec![3, 5, 7, 9, 12]);
    assert_eq!(selected("body > :nth-child(3)", &dom), vec![7]);
    assert_eq!(selected("body > :nth-child(2n - 1)", &dom), vec![3, 7, 12]);
}

#[test]
fn the_interaction_pseudo_classes_parse_and_weigh_but_never_match() {
    let dom = fixture();
    for source in ["p:hover", "p:active", "p:focus"] {
        assert_eq!(
            selected(source, &dom),
            Vec::<usize>::new(),
            "a DomSnapshot carries no interaction state (PRD-007:35-36)"
        );
        assert_eq!(
            only(source).specificity(),
            Specificity::new(0, 1, 1),
            "`{source}` still weighs as one class-like plus one type"
        );
    }
}

// ---- specificity ---------------------------------------------------------

#[test]
fn an_id_outweighs_a_class_which_outweighs_a_type() {
    let identifier = only("#page").specificity();
    let class = only(".lead").specificity();
    let type_name = only("p").specificity();

    assert_eq!(identifier, Specificity::new(1, 0, 0));
    assert_eq!(class, Specificity::new(0, 1, 0));
    assert_eq!(type_name, Specificity::new(0, 0, 1));
    assert!(identifier > class);
    assert!(class > type_name);
    assert!(type_name > Specificity::ZERO);
}

#[test]
fn specificity_sums_across_compounds_and_the_universal_selector_adds_nothing() {
    assert_eq!(only("*").specificity(), Specificity::ZERO);
    assert_eq!(only("*.lead").specificity(), Specificity::new(0, 1, 0));
    assert_eq!(
        only("body#page > p.lead[data-role]:first-child").specificity(),
        Specificity::new(1, 3, 2)
    );
    assert_eq!(
        only("div span").specificity(),
        Specificity::new(0, 0, 2),
        "a descendant combinator adds no weight of its own"
    );
}

#[test]
fn a_list_reports_the_strongest_of_its_matching_selectors() {
    let dom = fixture();
    let body = dom
        .nodes_in_document_order()
        .nth(2)
        .expect("body is node #2");
    let node = dom.node(body).expect("the body node");
    let list = selectors("body, #page, .absent");

    assert_eq!(
        strongest_match(&list, node, &dom),
        Some(Specificity::new(1, 0, 0)),
        "`#page` wins over `body`, and `.absent` does not match at all"
    );
    assert_eq!(
        strongest_match(&selectors(".absent"), node, &dom),
        None,
        "a list nothing in which matches has no weight"
    );
}

#[test]
fn a_selector_round_trips_through_its_own_display() {
    for source in [
        "p",
        "*",
        ".lead",
        "#page",
        "[data-role]",
        "[data-role=\"intro\"]",
        "h1, div",
        "body p",
        "body > p",
        "h1 + p",
        "h1 ~ p",
        "p:hover",
        "p:first-child",
        "p:nth-child(2n+1)",
    ] {
        assert_eq!(
            selectors(source).to_string(),
            source,
            "`{source}` must print as it was written"
        );
    }
}

// ---- what is declared out is refused, not ignored -------------------------

#[test]
fn every_form_outside_the_cut_is_refused_with_a_reason() {
    assert!(refusal(":has(p)").contains("`:has`"));
    assert!(refusal("p:not(.lead)").contains("`:not`"));
    assert!(refusal("p::before").contains("pseudo-element"));
    assert!(refusal("p::after").contains("pseudo-element"));
    assert!(refusal("svg|rect").contains("namespace"));
    assert!(refusal("[href^=\"http\"]").contains("only `[attr]`"));
    assert!(refusal("[href$=\".png\"]").contains("only `[attr]`"));
    assert!(refusal("[class~=\"lead\"]").contains("only `[attr]`"));
    assert!(refusal("p:nth-of-type(1)").contains("outside the v0.5 cut"));
}

#[test]
fn a_malformed_selector_is_refused_rather_than_half_read() {
    assert!(refusal(".").contains("class name"));
    assert!(refusal("[]").contains("attribute name"));
    assert!(refusal("[data-role").contains("only `[attr]`"));
    assert!(refusal("p:").contains("pseudo-class"));
}
