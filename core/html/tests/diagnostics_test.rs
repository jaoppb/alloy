//! Recoverable parse diagnostics (issue #36, ADR-0023): reported with a location, never fatal,
//! delivered in order, and the rest of the document still parses.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use html::{
    MockEvent, MockTreeSink, ParseErrorCode, Token, TokenSink, TokenSinkResult, Tokenizer,
    TokenizerRunResult, TreeBuilder, parse_with_sink,
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

fn mock_for(source: &str) -> MockTreeSink {
    let mut sink = MockTreeSink::new();
    parse_with_sink(source, &mut sink).unwrap();
    sink
}

fn created(sink: &MockTreeSink, name: &str) -> Vec<html::AttributeList> {
    sink.events()
        .iter()
        .filter_map(|event| match event {
            MockEvent::CreateElement {
                tag, attributes, ..
            } if tag == name => Some(attributes.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn an_invalid_attribute_name_is_dropped_and_reported_instead_of_dropped_silently() {
    let sink = mock_for("<div a\"b=1 id=kept>x</div>");
    let codes: Vec<_> = parse_error_events(&sink)
        .into_iter()
        .map(|(code, _, _)| code)
        .collect();
    assert_eq!(
        codes,
        [
            ParseErrorCode::UnexpectedCharacterInAttributeName,
            ParseErrorCode::InvalidAttributeName
        ]
    );
    let divs = created(&sink, "div");
    assert_eq!(divs[0].len(), 1);
    assert_eq!(divs[0].get_value_str("id"), Some("kept"));
}

#[test]
fn an_invalid_tag_name_no_longer_aborts_the_document() {
    let sink = mock_for("<p>a</p><my_widget>kept</my_widget><p>b</p>");
    let text: String = sink
        .events()
        .iter()
        .filter_map(|event| match event {
            MockEvent::CreateText { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "akeptb");
    assert_eq!(parse_error_events(&sink).len(), 2);
}

#[test]
fn a_duplicate_attribute_keeps_the_first() {
    let sink = mock_for("<div id=first id=second></div>");
    assert_eq!(created(&sink, "div")[0].get_value_str("id"), Some("first"));
}

#[test]
fn a_repeated_body_start_tag_hands_its_attributes_to_the_sink_to_merge() {
    let sink = mock_for("<body class=a><body id=b><p>x</p>");
    assert!(sink.events().iter().any(|event| matches!(
        event,
        MockEvent::AddAttributes {
            attribute_count: 1,
            ..
        }
    )));
}

#[test]
fn spec_implied_closures_stay_silent() {
    for source in [
        "<p>one<p>two",
        "<ul><li>one<li>two</ul>",
        "<ul><li><p>one</li></ul>",
        "<!DOCTYPE html><html><head></head><body><p>x</p></body></html>",
    ] {
        let sink = mock_for(source);
        assert!(
            parse_error_events(&sink).is_empty(),
            "{source:?}: {:?}",
            parse_error_events(&sink)
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
        mock_for(source);
    }
}
