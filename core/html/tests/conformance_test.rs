//! Conformance tests for [`TreeSink`] adapters (ADR-0011 item 6).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use html::{DomTreeSink, MockTreeSink, run_html_conformance};

#[test]
fn dom_tree_sink_passes_conformance() {
    let mut sink = DomTreeSink::new();
    run_html_conformance(&mut sink);
}

#[test]
fn mock_tree_sink_passes_conformance() {
    let mut sink = MockTreeSink::new();
    run_html_conformance(&mut sink);
    assert!(
        !sink.events().is_empty(),
        "Mock sink must have recorded events"
    );
}
