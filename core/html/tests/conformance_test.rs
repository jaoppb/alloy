//! Conformance tests for the reference [`TreeSink`] adapter (ADR-0011 item 6). The real
//! `DomTreeSink` is exercised in `core/dom/tests/html_sink_conformance.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use html::{MockTreeSink, run_html_conformance};

#[test]
fn mock_tree_sink_passes_conformance() {
    let mut sink = MockTreeSink::new();
    run_html_conformance(&mut sink);
    assert!(
        !sink.events().is_empty(),
        "Mock sink must have recorded events"
    );
}
