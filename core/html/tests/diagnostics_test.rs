//! Recoverable parse diagnostics (issue #36, ADR-0023): reported with a location, never fatal,
//! delivered in order, and the rest of the document still parses.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "dom")]

use html::{
    MockEvent, MockTreeSink, ParseErrorCode, Token, TokenSink, TokenSinkResult, Tokenizer,
    TokenizerRunResult, TreeBuilder, parse, parse_with_sink,
};

fn parse_error_events(sink: &MockTreeSink) -> Vec<(ParseErrorCode, usize, usize)> {
    sink.events()
        .iter()
        .filter_map(|event| match event {
            MockEvent::ParseError { code, location } => {
                Some((*code, location.line(), location.column()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_malformed_attribute_name_is_reported_twice_and_the_document_parses() {
    let source = "<p>before</p>\n<div a\"b=1>after</div>\n";
    let mut sink = MockTreeSink::new();
    parse_with_sink(source, &mut sink).expect("a malformed attribute must not abort");

    assert_eq!(
        parse_error_events(&sink),
        [
            (ParseErrorCode::UnexpectedCharacterInAttributeName, 2, 7),
            (ParseErrorCode::InvalidAttributeName, 2, 6),
        ]
    );
    let texts: Vec<_> = sink
        .events()
        .iter()
        .filter_map(|event| match event {
            MockEvent::CreateText { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        texts.contains(&"before"),
        "content before survives: {texts:?}"
    );
    assert!(
        texts.contains(&"after"),
        "content after survives: {texts:?}"
    );
}

#[test]
fn an_invalid_attribute_name_is_dropped_and_reported_instead_of_dropped_silently() {
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
    let tree = outcome.into_tree();
    let div = tree
        .descendants(tree.document())
        .find(|&node| matches!(tree.node_kind(node), Ok(dom::NodeKind::Element(el)) if el.tag().as_str() == "div"))
        .unwrap();
    let Ok(dom::NodeKind::Element(element)) = tree.node_kind(div) else {
        panic!("div is an element");
    };
    let id = dom::AttributeName::new("id").unwrap();
    assert_eq!(
        element
            .attributes()
            .get(&id)
            .map(dom::AttributeValue::as_str),
        Some("kept")
    );
}

#[test]
fn an_invalid_tag_name_no_longer_aborts_the_document() {
    let outcome = parse("<p>a</p><my_widget>kept</my_widget><p>b</p>").unwrap();
    let tree = outcome.tree();
    let text: String = tree
        .descendants(tree.document())
        .filter_map(|node| match tree.node_kind(node) {
            Ok(dom::NodeKind::Text(text)) => Some(text.as_str().to_string()),
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
    let div = tree
        .descendants(tree.document())
        .find(|&node| matches!(tree.node_kind(node), Ok(dom::NodeKind::Element(el)) if el.tag().as_str() == "div"))
        .unwrap();
    let Ok(dom::NodeKind::Element(element)) = tree.node_kind(div) else {
        panic!("div is an element");
    };
    let id = dom::AttributeName::new("id").unwrap();
    assert_eq!(
        element
            .attributes()
            .get(&id)
            .map(dom::AttributeValue::as_str),
        Some("first")
    );
}

#[test]
fn a_repeated_body_start_tag_merges_attributes_instead_of_dropping_them() {
    let outcome = parse("<body class=a><body id=b><p>x</p>").unwrap();
    let tree = outcome.tree();
    let body = tree
        .descendants(tree.document())
        .find(|&node| matches!(tree.node_kind(node), Ok(dom::NodeKind::Element(el)) if el.tag().as_str() == "body"))
        .unwrap();
    let Ok(dom::NodeKind::Element(element)) = tree.node_kind(body) else {
        panic!("body is an element");
    };
    let id = dom::AttributeName::new("id").unwrap();
    assert_eq!(
        element
            .attributes()
            .get(&id)
            .map(dom::AttributeValue::as_str),
        Some("b")
    );
}

#[test]
fn spec_implied_closures_stay_silent() {
    for source in [
        "<p>one<p>two",
        "<ul><li>one<li>two</ul>",
        "<ul><li><p>one</li></ul>",
        "<!DOCTYPE html><html><head></head><body><p>x</p></body></html>",
    ] {
        let outcome = parse(source).unwrap();
        assert!(
            outcome.diagnostics().is_empty(),
            "{source:?}: {:?}",
            outcome.diagnostics()
        );
    }
}

#[test]
fn a_diagnostic_precedes_the_token_that_triggered_it() {
    let mut sink = MockTreeSink::new();
    parse_with_sink("<div id=a id=b></div>", &mut sink).unwrap();
    let events = sink.events();
    let diagnostic = events
        .iter()
        .position(|event| matches!(event, MockEvent::ParseError { .. }))
        .unwrap();
    let element = events
        .iter()
        .position(|event| matches!(event, MockEvent::CreateElement { tag, .. } if tag == "div"))
        .unwrap();
    assert!(diagnostic < element);
}

#[test]
fn diagnostics_before_and_after_a_suspension_are_all_delivered() {
    struct SuspendOnScript<'a>(TreeBuilder<'a, MockTreeSink>);
    impl TokenSink for SuspendOnScript<'_> {
        fn process_token(&mut self, token: Token) -> Result<TokenSinkResult, html::HtmlError> {
            let is_script = matches!(&token, Token::StartTag(tag) if tag.name() == "script");
            let result = self.0.process_token(token)?;
            if is_script {
                return Ok(TokenSinkResult::Suspend);
            }
            Ok(result)
        }
        fn finish(&mut self) -> Result<(), html::HtmlError> {
            self.0.finish()
        }
    }

    let source = "<p>&bogus;</p><script>x</script><p>&#0;</p>";
    let mut mock = MockTreeSink::new();
    let mut sink = SuspendOnScript(TreeBuilder::new(&mut mock));
    let mut tokenizer = Tokenizer::new(source);
    let first = tokenizer.run_resumable(&mut sink).unwrap();
    assert!(matches!(first, TokenizerRunResult::Suspended { .. }));
    let second = tokenizer.resume("<i>&bogus;</i>", &mut sink).unwrap();
    assert!(matches!(
        second,
        TokenizerRunResult::Suspended { .. } | TokenizerRunResult::Completed
    ));
    while !matches!(
        tokenizer.resume("", &mut sink).unwrap(),
        TokenizerRunResult::Completed
    ) {}
    drop(sink);

    let codes: Vec<_> = parse_error_events(&mock)
        .into_iter()
        .map(|(code, _, _)| code)
        .collect();
    assert_eq!(
        codes,
        [
            ParseErrorCode::UnknownNamedCharacterReference,
            ParseErrorCode::UnknownNamedCharacterReference,
            ParseErrorCode::NullCharacterReference,
        ]
    );
}

#[test]
fn hostile_input_never_aborts_or_panics() {
    for source in [
        "<",
        "</",
        "<!",
        "<!-",
        "<!--",
        "<!DOCTYPE",
        "<a",
        "<a b",
        "<a b=",
        "<a b=\"",
        "<a b='c",
        "</a b",
        "&",
        "&#",
        "&#x",
        "<\0>",
        "<a\"\"\">",
        "</>",
        "<//>",
        "<a/ / />",
        "<=>",
    ] {
        parse(source).unwrap_or_else(|err| panic!("{source:?} aborted: {err}"));
    }
}
