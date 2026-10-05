//! Conformance gate for the declared HTML5 v0.5 cut.
//!
//! Mirrors `core/css/tests/manifest_runner.rs`:
//! Checks three-way consistency between `MANIFEST.md`, the registries in `lib.rs`,
//! and actual parser execution.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "dom")]

use std::collections::BTreeSet;
use std::path::PathBuf;

use html::{ParseErrorCode, SUPPORTED_PARSE_ERRORS, SUPPORTED_SYNTAX, SUPPORTED_TAGS, parse};

const MANIFEST_REL: &str = "tests/data/MANIFEST.md";

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(MANIFEST_REL)
}

fn parse_markdown_table_tokens(section_name: &str) -> BTreeSet<String> {
    let content = std::fs::read_to_string(manifest_path()).expect("MANIFEST.md must exist");
    let mut in_section = false;
    let mut tokens = BTreeSet::new();

    for line in content.lines() {
        if line.starts_with("## ") {
            in_section = line.contains(section_name);
            continue;
        }
        if !in_section {
            continue;
        }
        if line.starts_with('|') && !line.contains("---") {
            let parts: Vec<&str> = line.split('|').collect();
            if let Some(raw_token) = parts.get(1) {
                let token = raw_token.trim().trim_matches('`');
                if !token.is_empty() && token != "token" {
                    tokens.insert(token.to_string());
                }
            }
        }
    }

    tokens
}

#[test]
fn manifest_and_tag_registry_match_in_both_directions() {
    let from_manifest = parse_markdown_table_tokens("Tags");
    let from_registry: BTreeSet<String> = SUPPORTED_TAGS.iter().map(|&s| s.to_string()).collect();

    let manifest_only: Vec<_> = from_manifest.difference(&from_registry).collect();
    assert!(
        manifest_only.is_empty(),
        "Tokens in MANIFEST.md under ## Tags missing from SUPPORTED_TAGS: {manifest_only:?}"
    );

    let registry_only: Vec<_> = from_registry.difference(&from_manifest).collect();
    assert!(
        registry_only.is_empty(),
        "Tokens in SUPPORTED_TAGS missing from MANIFEST.md under ## Tags: {registry_only:?}"
    );
}

#[test]
fn manifest_and_syntax_registry_match_in_both_directions() {
    let from_manifest = parse_markdown_table_tokens("Syntax");
    let from_registry: BTreeSet<String> = SUPPORTED_SYNTAX.iter().map(|&s| s.to_string()).collect();

    let manifest_only: Vec<_> = from_manifest.difference(&from_registry).collect();
    assert!(
        manifest_only.is_empty(),
        "Tokens in MANIFEST.md under ## Syntax missing from SUPPORTED_SYNTAX: {manifest_only:?}"
    );

    let registry_only: Vec<_> = from_registry.difference(&from_manifest).collect();
    assert!(
        registry_only.is_empty(),
        "Tokens in SUPPORTED_SYNTAX missing from MANIFEST.md under ## Syntax: {registry_only:?}"
    );
}

fn make_tag_probe(tag: &str) -> String {
    if html::is_void_tag(tag) {
        return format!("<{tag}>");
    }
    format!("<{tag}>test</{tag}>")
}

#[test]
fn every_supported_tag_has_a_passing_probe() {
    for &tag in SUPPORTED_TAGS {
        let snippet = make_tag_probe(tag);
        let tree = parse(&snippet)
            .unwrap_or_else(|err| panic!("Failed to parse probe for <{tag}>: {err}"))
            .into_tree();

        let found = tree
            .descendants(tree.document())
            .any(|node| match tree.node_kind(node) {
                Ok(dom::NodeKind::Element(elem)) => elem.tag().as_str() == tag,
                _ => false,
            });

        assert!(found, "Tag <{tag}> was parsed but not found in DomTree");
    }
}

#[test]
fn test_syntax_attributes() {
    // double-quoted attribute
    let tree = parse("<div class=\"main\"></div>").unwrap().into_tree();
    let div = tree
        .descendants(tree.document())
        .find(|&n| match tree.node_kind(n) {
            Ok(dom::NodeKind::Element(el)) => el.tag().as_str() == "div",
            _ => false,
        })
        .unwrap();
    let class_val = tree.node_kind(div).unwrap();
    match class_val {
        dom::NodeKind::Element(el) => {
            assert_eq!(
                el.attributes()
                    .get(&dom::AttributeName::new("class").unwrap())
                    .unwrap()
                    .as_str(),
                "main"
            );
        }
        _ => panic!("Expected element"),
    }

    // single-quoted attribute
    let tree = parse("<div class='sidebar'></div>").unwrap().into_tree();
    let div = tree
        .descendants(tree.document())
        .find(|&n| match tree.node_kind(n) {
            Ok(dom::NodeKind::Element(el)) => el.tag().as_str() == "div",
            _ => false,
        })
        .unwrap();
    match tree.node_kind(div).unwrap() {
        dom::NodeKind::Element(el) => {
            assert_eq!(
                el.attributes()
                    .get(&dom::AttributeName::new("class").unwrap())
                    .unwrap()
                    .as_str(),
                "sidebar"
            );
        }
        _ => panic!("Expected element"),
    }

    // unquoted attribute
    let tree = parse("<div id=header></div>").unwrap().into_tree();
    let div = tree
        .descendants(tree.document())
        .find(|&n| match tree.node_kind(n) {
            Ok(dom::NodeKind::Element(el)) => el.tag().as_str() == "div",
            _ => false,
        })
        .unwrap();
    match tree.node_kind(div).unwrap() {
        dom::NodeKind::Element(el) => {
            assert_eq!(
                el.attributes()
                    .get(&dom::AttributeName::new("id").unwrap())
                    .unwrap()
                    .as_str(),
                "header"
            );
        }
        _ => panic!("Expected element"),
    }

    // boolean attribute
    let tree = parse("<input disabled>").unwrap().into_tree();
    let input = tree
        .descendants(tree.document())
        .find(|&n| match tree.node_kind(n) {
            Ok(dom::NodeKind::Element(el)) => el.tag().as_str() == "input",
            _ => false,
        })
        .unwrap();
    match tree.node_kind(input).unwrap() {
        dom::NodeKind::Element(el) => {
            assert!(
                el.attributes()
                    .get(&dom::AttributeName::new("disabled").unwrap())
                    .is_some()
            );
        }
        _ => panic!("Expected element"),
    }
}

#[test]
fn test_syntax_doctype_and_tags() {
    // 1. DOCTYPE
    let tree = parse("<!DOCTYPE html><html><body><p>Hi</p></body></html>")
        .unwrap()
        .into_tree();
    assert!(tree.descendants(tree.document()).count() >= 4);

    // 2. self-closing tag
    let tree = parse("<br />").unwrap().into_tree();
    assert!(
        tree.descendants(tree.document())
            .any(|n| match tree.node_kind(n) {
                Ok(dom::NodeKind::Element(el)) => el.tag().as_str() == "br",
                _ => false,
            })
    );

    // 3. comments
    let tree = parse("<!-- note --><div></div>").unwrap().into_tree();
    assert!(
        tree.descendants(tree.document())
            .any(|n| matches!(tree.node_kind(n), Ok(dom::NodeKind::Comment(_))))
    );
}

#[test]
fn test_syntax_entities() {
    let tree = parse("<p>&copy; &amp; &#60; &#x3e;</p>")
        .unwrap()
        .into_tree();
    let text = tree
        .descendants(tree.document())
        .find_map(|n| match tree.node_kind(n) {
            Ok(dom::NodeKind::Text(t)) => Some(t.as_str().to_string()),
            _ => None,
        })
        .unwrap();
    assert!(text.contains('©'));
    assert!(text.contains('&'));
    assert!(text.contains('<'));
    assert!(text.contains('>'));
}

#[test]
fn test_syntax_rawtext_and_omissions() {
    // 1. script rawtext
    let tree = parse("<script>const markup = '<div>inside</div>';</script>")
        .unwrap()
        .into_tree();
    let has_inner_div = tree
        .descendants(tree.document())
        .any(|n| match tree.node_kind(n) {
            Ok(dom::NodeKind::Element(el)) => el.tag().as_str() == "div",
            _ => false,
        });
    assert!(
        !has_inner_div,
        "<script> rawtext must not parse inner markup as elements"
    );

    // 2. p tag omission
    let tree = parse("<p>First<p>Second").unwrap().into_tree();
    let p_nodes: Vec<_> = tree
        .descendants(tree.document())
        .filter(|&n| match tree.node_kind(n) {
            Ok(dom::NodeKind::Element(el)) => el.tag().as_str() == "p",
            _ => false,
        })
        .collect();
    assert_eq!(p_nodes.len(), 2);
    assert_ne!(tree.parent(p_nodes[1]).unwrap(), Some(p_nodes[0]));

    // 3. li tag omission
    let tree = parse("<ul><li>One<li>Two</ul>").unwrap().into_tree();
    let li_nodes: Vec<_> = tree
        .descendants(tree.document())
        .filter(|&n| match tree.node_kind(n) {
            Ok(dom::NodeKind::Element(el)) => el.tag().as_str() == "li",
            _ => false,
        })
        .collect();
    assert_eq!(li_nodes.len(), 2);
    assert_ne!(tree.parent(li_nodes[1]).unwrap(), Some(li_nodes[0]));
}

#[test]
fn manifest_and_parse_error_registry_match_in_both_directions() {
    let from_manifest = parse_markdown_table_tokens("Parse errors");
    let from_registry: BTreeSet<String> = SUPPORTED_PARSE_ERRORS
        .iter()
        .map(|&code| code.to_string())
        .collect();

    let manifest_only: Vec<_> = from_manifest.difference(&from_registry).collect();
    assert!(
        manifest_only.is_empty(),
        "Codes in MANIFEST.md under ## Parse errors missing from SUPPORTED_PARSE_ERRORS: {manifest_only:?}"
    );

    let registry_only: Vec<_> = from_registry.difference(&from_manifest).collect();
    assert!(
        registry_only.is_empty(),
        "Codes in SUPPORTED_PARSE_ERRORS missing from MANIFEST.md under ## Parse errors: {registry_only:?}"
    );
}

/// `(source, [(code, line, column)])` — every code the parser can report, with an input that
/// produces exactly that list and still builds a tree.
type Probe = (&'static str, &'static [(ParseErrorCode, usize, usize)]);

const PARSE_ERROR_PROBES: &[Probe] = &[
    (
        "<div a\"b=1>x</div>",
        &[
            (ParseErrorCode::UnexpectedCharacterInAttributeName, 1, 7),
            (ParseErrorCode::UnsupportedAttributeName, 1, 6),
        ],
    ),
    (
        "<div =a>x</div>",
        &[
            (
                ParseErrorCode::UnexpectedEqualsSignBeforeAttributeName,
                1,
                6,
            ),
            (ParseErrorCode::UnsupportedAttributeName, 1, 6),
        ],
    ),
    (
        "<div id=a id=b></div>",
        &[(ParseErrorCode::DuplicateAttribute, 1, 11)],
    ),
    (
        "<div id=></div>",
        &[(ParseErrorCode::MissingAttributeValue, 1, 9)],
    ),
    (
        "<div a=\"1\"b=\"2\"></div>",
        &[(ParseErrorCode::MissingWhitespaceBetweenAttributes, 1, 11)],
    ),
    (
        "<div / a></div>",
        &[(ParseErrorCode::UnexpectedSolidusInTag, 1, 8)],
    ),
    (
        "<div></div x=\"1\">",
        &[(ParseErrorCode::EndTagWithAttributes, 1, 6)],
    ),
    (
        "<p>a</p><div class=\"x",
        &[(ParseErrorCode::EofInTag, 1, 22)],
    ),
    ("<p>a</></p>", &[(ParseErrorCode::MissingEndTagName, 1, 7)]),
    ("<p>a<", &[(ParseErrorCode::EofBeforeTagName, 1, 6)]),
    (
        "<p>1 < 2</p>",
        &[(ParseErrorCode::InvalidFirstCharacterOfTagName, 1, 7)],
    ),
    (
        "<?xml version=\"1.0\"?><p>x</p>",
        &[(ParseErrorCode::UnexpectedQuestionMarkInsteadOfTagName, 1, 2)],
    ),
    (
        "<p>&bogus;</p>",
        &[(ParseErrorCode::UnknownNamedCharacterReference, 1, 4)],
    ),
    (
        "<p>&#;</p>",
        &[(
            ParseErrorCode::AbsenceOfDigitsInNumericCharacterReference,
            1,
            4,
        )],
    ),
    (
        "<p>&#0;</p>",
        &[(ParseErrorCode::NullCharacterReference, 1, 4)],
    ),
    (
        "<p>&#x110000;</p>",
        &[(ParseErrorCode::CharacterReferenceOutsideUnicodeRange, 1, 4)],
    ),
    (
        "<p>&#xD800;</p>",
        &[(ParseErrorCode::SurrogateCharacterReference, 1, 4)],
    ),
    (
        "<p>a\0b</p>",
        &[(ParseErrorCode::UnexpectedNullCharacter, 1, 5)],
    ),
    (
        "<p>a\u{1}b</p>",
        &[(ParseErrorCode::ControlCharacterInInputStream, 1, 5)],
    ),
    (
        "<!--><p>x</p>",
        &[(ParseErrorCode::AbruptClosingOfEmptyComment, 1, 1)],
    ),
    (
        "<!x><p>y</p>",
        &[(ParseErrorCode::IncorrectlyOpenedComment, 1, 1)],
    ),
    (
        "<p>a</p><!-- open",
        &[(ParseErrorCode::EofInComment, 1, 18)],
    ),
    ("<!DOCTYPE html", &[(ParseErrorCode::EofInDoctype, 1, 15)]),
    (
        "<!DOCTYPE><p>x</p>",
        &[
            (ParseErrorCode::MissingDoctypeName, 1, 1),
            (ParseErrorCode::QuirksModeDoctype, 1, 1),
        ],
    ),
    (
        "<p>a</p><my_widget>b</my_widget>",
        &[
            (ParseErrorCode::InvalidTagName, 1, 9),
            (ParseErrorCode::InvalidTagName, 1, 21),
        ],
    ),
    ("<p>a</p></div>", &[(ParseErrorCode::StrayEndTag, 1, 9)]),
    (
        "<div><span>x</div>",
        &[(ParseErrorCode::EndTagDoesNotMatchCurrentNode, 1, 13)],
    ),
    (
        "<p><span>a<div>b</div>",
        &[(ParseErrorCode::ElementClosedImplicitly, 1, 11)],
    ),
    (
        "<html><body><html lang=\"en\"><p>x</p>",
        &[(ParseErrorCode::UnexpectedHtmlStartTag, 1, 13)],
    ),
    (
        "<body class=\"a\"><body id=\"b\"><p>x</p>",
        &[(ParseErrorCode::UnexpectedBodyStartTag, 1, 17)],
    ),
    (
        "<head></head><head>",
        &[(ParseErrorCode::UnexpectedHeadStartTag, 1, 14)],
    ),
    (
        "<!DOCTYPE foo><p>x</p>",
        &[(ParseErrorCode::QuirksModeDoctype, 1, 1)],
    ),
    (
        "<div a\"b=1>x</div>",
        &[
            (ParseErrorCode::UnexpectedCharacterInAttributeName, 1, 7),
            (ParseErrorCode::UnsupportedAttributeName, 1, 6),
        ],
    ),
];

#[test]
fn every_parse_error_code_has_an_exact_probe() {
    for (source, expected) in PARSE_ERROR_PROBES {
        let outcome =
            parse(source).unwrap_or_else(|err| panic!("{source:?} must not abort: {err}"));
        let found: Vec<_> = outcome
            .diagnostics()
            .iter()
            .map(|d| (d.code(), d.location().line(), d.location().column()))
            .collect();
        assert_eq!(&found, expected, "diagnostics for {source:?}");
    }

    let probed: BTreeSet<&str> = PARSE_ERROR_PROBES
        .iter()
        .flat_map(|(_, expected)| expected.iter().map(|(code, _, _)| code.as_str()))
        .collect();
    let missing: Vec<_> = SUPPORTED_PARSE_ERRORS
        .iter()
        .filter(|code| !probed.contains(*code))
        .collect();
    assert!(missing.is_empty(), "codes without a probe: {missing:?}");
}
