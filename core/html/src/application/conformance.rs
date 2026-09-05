//! An adapter-agnostic conformance suite for [`TreeSink`] implementations (ADR-0011 item 6).
//!
//! Ordinary library code, not `#[cfg(test)]`, so an adapter can call it from its own tests.

#![allow(clippy::panic, clippy::expect_used)]

use crate::application::ports::TreeSink;
use crate::domain::token::{AttributeEntry, AttributeList};

/// Runs the standard conformance suite against any [`TreeSink`] implementation.
///
/// Panics on the first invariant violation with a diagnostic message.
pub fn run_html_conformance(sink: &mut dyn TreeSink) {
    check_root_is_available(sink);
    check_element_creation_and_append(sink);
    check_text_creation_and_append(sink);
    check_comment_creation_and_append(sink);
}

fn check_root_is_available(sink: &dyn TreeSink) {
    let root = sink.root_node();
    assert_eq!(root, dom::NodeId::root(), "Root node must be node #0");
}

fn check_element_creation_and_append(sink: &mut dyn TreeSink) {
    let root = sink.root_node();
    let mut attrs = AttributeList::new();
    let entry = match AttributeEntry::new("lang", "en") {
        Ok(entry) => entry,
        Err(err) => panic!("valid attribute: {err}"),
    };
    attrs.push(entry);

    let html_node = match sink.create_element("html", &attrs) {
        Ok(node) => node,
        Err(err) => panic!("element creation must succeed: {err}"),
    };

    if let Err(err) = sink.append_child(root, html_node) {
        panic!("appending child to root must succeed: {err}");
    }
}

fn check_text_creation_and_append(sink: &mut dyn TreeSink) {
    let root = sink.root_node();
    let text_node = match sink.create_text("Hello, Alloy!") {
        Ok(node) => node,
        Err(err) => panic!("text creation must succeed: {err}"),
    };

    if let Err(err) = sink.append_child(root, text_node) {
        panic!("appending text node must succeed: {err}");
    }
}

fn check_comment_creation_and_append(sink: &mut dyn TreeSink) {
    let root = sink.root_node();
    let comment_node = match sink.create_comment("This is a comment") {
        Ok(node) => node,
        Err(err) => panic!("comment creation must succeed: {err}"),
    };

    if let Err(err) = sink.append_child(root, comment_node) {
        panic!("appending comment node must succeed: {err}");
    }
}
