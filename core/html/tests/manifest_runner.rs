//! Conformance gate for the declared HTML5 v0.5 cut.
//!
//! Mirrors `core/css/tests/manifest_runner.rs`:
//! Checks three-way consistency between `MANIFEST.md`, the registries in `lib.rs`,
//! and actual parser execution.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use html::{
    AttributeList, MockEvent, MockTreeSink, NodeHandle, ParseErrorCode, SUPPORTED_PARSE_ERRORS,
    SUPPORTED_SYNTAX, SUPPORTED_TAGS, parse_with_sink,
};

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
    if html::TagName::new(tag).unwrap().is_void() {
        return format!("<{tag}>");
    }
    format!("<{tag}>test</{tag}>")
}

fn events_of(source: &str) -> Vec<MockEvent> {
    let mut sink = MockTreeSink::new();
    parse_with_sink(source, &mut sink)
        .unwrap_or_else(|err| panic!("{source:?} must not abort: {err}"));
    sink.events().to_vec()
}

fn elements_named<'events>(
    events: &'events [MockEvent],
    name: &str,
) -> Vec<(NodeHandle, &'events AttributeList)> {
    events
        .iter()
        .filter_map(|event| match event {
            MockEvent::CreateElement {
                id,
                tag,
                attributes,
            } if tag == name => Some((*id, attributes)),
            _ => None,
        })
        .collect()
}

fn texts(events: &[MockEvent]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| match event {
            MockEvent::CreateText { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect()
}

fn parent_of(events: &[MockEvent], node: NodeHandle) -> Option<NodeHandle> {
    events.iter().find_map(|event| match event {
        MockEvent::AppendChild { parent, child } if *child == node => Some(*parent),
        _ => None,
    })
}

#[test]
fn every_supported_tag_has_a_passing_probe() {
    for &tag in SUPPORTED_TAGS {
        let events = events_of(&make_tag_probe(tag));
        assert!(
            !elements_named(&events, tag).is_empty(),
            "Tag <{tag}> was parsed but never created"
        );
    }
}

#[test]
fn test_syntax_attributes() {
    let value_of = |source: &str, element: &str, attribute: &str| {
        let events = events_of(source);
        let (_, attributes) = elements_named(&events, element)[0];
        attributes.get_value_str(attribute).map(str::to_owned)
    };

    // double-quoted, single-quoted, unquoted and boolean attributes
    assert_eq!(
        value_of("<div class=\"main\"></div>", "div", "class").as_deref(),
        Some("main")
    );
    assert_eq!(
        value_of("<div class='sidebar'></div>", "div", "class").as_deref(),
        Some("sidebar")
    );
    assert_eq!(
        value_of("<div id=header></div>", "div", "id").as_deref(),
        Some("header")
    );
    assert!(value_of("<input disabled>", "input", "disabled").is_some());
}

#[test]
fn test_syntax_doctype_and_tags() {
    // 1. DOCTYPE
    let events = events_of("<!DOCTYPE html><html><body><p>Hi</p></body></html>");
    for tag in ["html", "body", "p"] {
        assert_eq!(elements_named(&events, tag).len(), 1, "<{tag}>");
    }

    // 2. self-closing tag
    let events = events_of("<br />");
    assert_eq!(elements_named(&events, "br").len(), 1);

    // 3. comments
    let events = events_of("<!-- note --><div></div>");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, MockEvent::CreateComment { .. }))
    );
}

#[test]
fn test_syntax_entities() {
    let events = events_of("<p>&copy; &amp; &#60; &#x3e;</p>");
    let text: String = texts(&events).concat();
    assert!(text.contains('©'));
    assert!(text.contains('&'));
    assert!(text.contains('<'));
    assert!(text.contains('>'));
}

#[test]
fn test_syntax_full_named_reference_table() {
    let events = events_of("<p>&hellip; &euro; &larr; &NotEqualTilde; &copy &#128;</p>");
    let text: String = texts(&events).concat();
    assert_eq!(text, "… € ← \u{2242}\u{338} © €");
}

#[test]
fn test_syntax_attribute_reference_rule() {
    let events = events_of("<a href=\"?q=1&copy=2&amp;x\">y</a>");
    let (_, attributes) = elements_named(&events, "a")[0];
    assert_eq!(attributes.get_value_str("href"), Some("?q=1&copy=2&x"));
}

#[test]
fn test_syntax_rawtext_and_omissions() {
    // 1. script rawtext
    let events = events_of("<script>const markup = '<div>inside</div>';</script>");
    assert!(
        elements_named(&events, "div").is_empty(),
        "<script> rawtext must not parse inner markup as elements"
    );

    // 2. p tag omission
    let events = events_of("<p>First<p>Second");
    let paragraphs = elements_named(&events, "p");
    assert_eq!(paragraphs.len(), 2);
    assert_ne!(parent_of(&events, paragraphs[1].0), Some(paragraphs[0].0));

    // 3. li tag omission
    let events = events_of("<ul><li>One<li>Two</ul>");
    let items = elements_named(&events, "li");
    assert_eq!(items.len(), 2);
    assert_ne!(parent_of(&events, items[1].0), Some(items[0].0));
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
            (ParseErrorCode::InvalidAttributeName, 1, 6),
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
            (ParseErrorCode::InvalidAttributeName, 1, 6),
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
        "<p>&copy</p>",
        &[(
            ParseErrorCode::MissingSemicolonAfterCharacterReference,
            1,
            4,
        )],
    ),
    (
        "<p>&#128;</p>",
        &[(ParseErrorCode::ControlCharacterReference, 1, 4)],
    ),
    (
        "<p>&#xFDD0;</p>",
        &[(ParseErrorCode::NoncharacterCharacterReference, 1, 4)],
    ),
    (
        "<div a=x=y></div>",
        &[(
            ParseErrorCode::UnexpectedCharacterInUnquotedAttributeValue,
            1,
            9,
        )],
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
];

#[test]
fn every_parse_error_code_has_an_exact_probe() {
    for (source, expected) in PARSE_ERROR_PROBES {
        let found: Vec<_> = events_of(source)
            .iter()
            .filter_map(|event| match event {
                MockEvent::ParseError { code, location } => {
                    Some((*code, location.line(), location.column()))
                }
                _ => None,
            })
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
