//! Conformance of the real [`dom::DomTreeSink`] to the `html` `TreeSink` port (ADR-0011 item 6).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dom::DomTreeSink;
use html::run_html_conformance;

#[test]
fn dom_tree_sink_passes_conformance() {
    let mut sink = DomTreeSink::new();
    run_html_conformance(&mut sink);
}
