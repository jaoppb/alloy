//! What the real [`dom::DomTreeSink`] builds when `html` reports recoverable parse errors
//! (ADR-0023): the tree still forms, duplicates keep the first, repeated `<body>` merges.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dom::{DomTree, NodeId, NodeKind, parse};
use html::{AttributeName, ParseErrorCode};

fn first_element(tree: &DomTree, tag: &str) -> NodeId {
    tree.descendants(tree.document())
        .find(|&node| matches!(tree.node_kind(node), Ok(NodeKind::Element(el)) if el.tag().as_str() == tag))
        .unwrap_or_else(|| panic!("<{tag}> must exist"))
}

fn attribute(tree: &DomTree, node: NodeId, name: &str) -> Option<String> {
    tree.attribute(node, &AttributeName::new(name).unwrap())
        .unwrap()
        .map(|value| value.as_str().to_owned())
}

#[test]
fn an_invalid_attribute_name_is_dropped_with_a_diagnostic_and_the_rest_is_kept() {
    let outcome = parse("<div a\"b=1 id=kept>x</div>").unwrap();
    let codes: Vec<_> = outcome
        .diagnostics()
        .iter()
        .map(html::ParseDiagnostic::code)
        .collect();
    assert_eq!(
        codes,
        [
            ParseErrorCode::UnexpectedCharacterInAttributeName,
            ParseErrorCode::InvalidAttributeName
        ]
    );
    let tree = outcome.tree();
    let div = first_element(tree, "div");
    assert_eq!(attribute(tree, div, "id").as_deref(), Some("kept"));
}

#[test]
fn an_invalid_tag_name_no_longer_aborts_the_document() {
    let outcome = parse("<p>a</p><my_widget>kept</my_widget><p>b</p>").unwrap();
    let tree = outcome.tree();
    let text: String = tree
        .descendants(tree.document())
        .filter_map(|node| match tree.node_kind(node) {
            Ok(NodeKind::Text(text)) => Some(text.as_str().to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "akeptb");
    assert_eq!(outcome.diagnostics().len(), 2);
}

#[test]
fn a_duplicate_attribute_keeps_the_first() {
    let outcome = parse("<div id=first id=second></div>").unwrap();
    let tree = outcome.tree();
    let div = first_element(tree, "div");
    assert_eq!(attribute(tree, div, "id").as_deref(), Some("first"));
}

#[test]
fn a_repeated_body_start_tag_merges_attributes_instead_of_dropping_them() {
    let outcome = parse("<body class=a><body id=b><p>x</p>").unwrap();
    let tree = outcome.tree();
    let body = first_element(tree, "body");
    assert_eq!(attribute(tree, body, "id").as_deref(), Some("b"));
    assert_eq!(attribute(tree, body, "class").as_deref(), Some("a"));
}
