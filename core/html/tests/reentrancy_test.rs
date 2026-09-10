//! Verification of tokenizer suspension, input injection, and resumption (PRD-008 §3.4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use html::{
    MockEvent, MockTreeSink, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerRunResult,
    TreeBuilder,
};

/// A custom sink that intercepts scripts and requests suspension.
struct SuspendingScriptSink<'a> {
    builder: TreeBuilder<'a>,
    suspended_count: usize,
}

impl<'a> SuspendingScriptSink<'a> {
    fn new(sink: &'a mut dyn html::TreeSink) -> Self {
        Self {
            builder: TreeBuilder::new(sink),
            suspended_count: 0,
        }
    }
}

impl TokenSink for SuspendingScriptSink<'_> {
    fn process_token(&mut self, token: Token) -> Result<TokenSinkResult, html::HtmlError> {
        let is_script_start = matches!(&token, Token::StartTag(tag) if tag.name() == "script");
        if is_script_start {
            self.suspended_count = self.suspended_count.saturating_add(1);
            // Dispatch tag to builder first, then suspend
            let _ = self.builder.process_token(token)?;
            return Ok(TokenSinkResult::Suspend);
        }
        self.builder.process_token(token)
    }

    fn finish(&mut self) -> Result<(), html::HtmlError> {
        self.builder.finish()
    }
}

#[test]
fn test_script_suspends_and_resumes_with_injected_input() {
    let html_content = "<div><script>run();</script><span>after</span></div>";
    let mut mock_sink = MockTreeSink::new();

    let mut suspending_sink = SuspendingScriptSink::new(&mut mock_sink);
    let mut tokenizer = Tokenizer::new(html_content);

    // 1. Initial run suspends when <script> is encountered
    let first_run = tokenizer
        .run_resumable(&mut suspending_sink)
        .expect("initial run succeeds");

    assert!(matches!(first_run, TokenizerRunResult::Suspended { .. }));
    assert_eq!(suspending_sink.suspended_count, 1);

    // 2. Simulate document.write("<p>injected</p>") by resuming with extra input
    let second_run = tokenizer
        .resume("<p>injected</p>", &mut suspending_sink)
        .expect("resume succeeds");

    assert_eq!(second_run, TokenizerRunResult::Completed);

    // 3. Verify that the injected paragraph and the subsequent span were both created
    let events = mock_sink.events();
    let has_injected_p = events.iter().any(|event| match event {
        MockEvent::CreateElement { tag, .. } => tag == "p",
        _ => false,
    });
    let has_injected_text = events.iter().any(|event| match event {
        MockEvent::CreateText { content, .. } => content.contains("injected"),
        _ => false,
    });
    let has_span_after = events.iter().any(|event| match event {
        MockEvent::CreateElement { tag, .. } => tag == "span",
        _ => false,
    });

    assert!(has_injected_p, "Injected <p> element must be in the tree");
    assert!(
        has_injected_text,
        "Injected text must be created in the tree"
    );
    assert!(
        has_span_after,
        "Subsequent <span> element must still be processed"
    );
}
