//! An adapter-agnostic conformance suite for [`TreeSink`] implementations (ADR-0011 item 6).
//!
//! Ordinary library code, not `#[cfg(test)]`, so an adapter can call it from its own tests.

#![allow(clippy::panic, clippy::expect_used)]

use crate::application::ports::TreeSink;
use crate::domain::attribute::{AttributeEntry, AttributeList, AttributeName, AttributeValue};
use crate::domain::diagnostic::{ParseDiagnostic, ParseErrorCode};
use crate::domain::handle::NodeHandle;
use crate::domain::location::SourceLocation;
use crate::domain::tag::TagName;
use crate::domain::text::Text;

/// Runs the standard conformance suite against any [`TreeSink`] implementation.
///
/// Panics on the first invariant violation with a diagnostic message.
pub fn run_html_conformance(sink: &mut dyn TreeSink) {
    check_root_is_available(sink);
    check_element_creation_and_append(sink);
    check_text_creation_and_append(sink);
    check_comment_creation_and_append(sink);
    check_append_before_sibling(sink);
    check_add_attributes_if_missing(sink);
    check_remove_and_reparent(sink);
    check_parse_error_is_accepted(sink);
}

fn check_root_is_available(sink: &dyn TreeSink) {
    let root = sink.root_node();
    assert_eq!(root, NodeHandle::root(), "Root node must be canonical root");
}

fn check_element_creation_and_append(sink: &mut dyn TreeSink) {
    let root = sink.root_node();
    let mut attributes = AttributeList::new();
    let location = SourceLocation::initial();
    let name = AttributeName::new("lang").expect("valid attribute name");
    let value = AttributeValue::new("en");
    attributes
        .insert(AttributeEntry::new(name, value, location))
        .expect("unique attribute");

    let tag = TagName::new("html").expect("valid tag name");
    let html_node = sink
        .create_element(tag, &attributes)
        .expect("element creation must succeed");

    sink.append_child(root, html_node)
        .expect("appending child to root must succeed");
}

fn check_text_creation_and_append(sink: &mut dyn TreeSink) {
    let root = sink.root_node();
    let text = Text::new("Hello, Alloy!");
    let text_node = sink.create_text(&text).expect("text creation must succeed");

    sink.append_child(root, text_node)
        .expect("appending text node must succeed");
}

fn check_comment_creation_and_append(sink: &mut dyn TreeSink) {
    let root = sink.root_node();
    let comment = Text::new("This is a comment");
    let comment_node = sink
        .create_comment(&comment)
        .expect("comment creation must succeed");

    sink.append_child(root, comment_node)
        .expect("appending comment node must succeed");
}

fn check_append_before_sibling(sink: &mut dyn TreeSink) {
    let root = sink.root_node();
    let tag_div = TagName::new("div").expect("valid tag");
    let empty_attrs = AttributeList::new();
    let first = sink
        .create_element(tag_div.clone(), &empty_attrs)
        .expect("first child");
    let second = sink
        .create_element(tag_div, &empty_attrs)
        .expect("second child");

    sink.append_child(root, second).expect("append second");
    sink.append_before_sibling(second, first)
        .expect("insert before");
}

fn check_add_attributes_if_missing(sink: &mut dyn TreeSink) {
    let location = SourceLocation::initial();
    let tag = TagName::new("div").expect("valid tag");
    let empty_attrs = AttributeList::new();
    let node = sink.create_element(tag, &empty_attrs).expect("element");

    let mut new_attrs = AttributeList::new();
    let name = AttributeName::new("id").expect("name");
    new_attrs
        .insert(AttributeEntry::new(
            name,
            AttributeValue::new("main"),
            location,
        ))
        .expect("unique attribute");

    sink.add_attributes_if_missing(node, &new_attrs)
        .expect("add attributes");
}

fn check_remove_and_reparent(sink: &mut dyn TreeSink) {
    let empty_attrs = AttributeList::new();
    let parent_one = sink
        .create_element(TagName::new("div").expect("valid tag name"), &empty_attrs)
        .expect("parent1");
    let parent_two = sink
        .create_element(TagName::new("div").expect("valid tag name"), &empty_attrs)
        .expect("parent2");
    let child = sink
        .create_text(&Text::new("child text"))
        .expect("child node");

    sink.append_child(parent_one, child).expect("append child");
    sink.reparent_children(parent_one, parent_two)
        .expect("reparent");
    sink.remove_from_parent(parent_one).expect("remove");
}

fn check_parse_error_is_accepted(sink: &mut dyn TreeSink) {
    // Infallible and order-preserving: a sink may record the diagnostic but must not fail on it,
    // and reporting one must leave the tree operations around it working (ADR-0023).
    let location = SourceLocation::initial();
    sink.parse_error(ParseDiagnostic::new(ParseErrorCode::StrayEndTag, location));
    let node = sink
        .create_text(&Text::new("after diagnostic"))
        .expect("text creation after a diagnostic");
    sink.append_child(sink.root_node(), node)
        .expect("append after a diagnostic");
}
