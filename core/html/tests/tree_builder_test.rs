//! Tests for HTML5 tree construction rules and tag omission scoping (issue #78, WHATWG §13.2.6.4.7).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use html::{MockEvent, MockTreeSink, NodeHandle, parse_with_sink};

fn events_of(source: &str) -> Vec<MockEvent> {
    let mut sink = MockTreeSink::new();
    parse_with_sink(source, &mut sink)
        .unwrap_or_else(|err| panic!("{source:?} must not abort: {err}"));
    sink.events().to_vec()
}

fn elements_named(events: &[MockEvent], name: &str) -> Vec<NodeHandle> {
    events
        .iter()
        .filter_map(|event| match event {
            MockEvent::CreateElement { id, tag, .. } if tag == name => Some(*id),
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
fn nested_li_remains_child_of_inner_ul() {
    let events = events_of("<ul><li>a<ul><li>b</ul></ul>");
    let uls = elements_named(&events, "ul");
    let lis = elements_named(&events, "li");
    assert_eq!(uls.len(), 2, "must create two <ul> elements");
    assert_eq!(lis.len(), 2, "must create two <li> elements");

    let outer_ul = uls[0];
    let outer_li = lis[0];
    let inner_ul = uls[1];
    let inner_li = lis[1];

    assert_eq!(parent_of(&events, outer_li), Some(outer_ul));
    assert_eq!(parent_of(&events, inner_ul), Some(outer_li));
    assert_eq!(
        parent_of(&events, inner_li),
        Some(inner_ul),
        "inner <li> must remain a child of the inner <ul>"
    );
}

#[test]
fn definition_list_omission_yields_sibling_children() {
    let events = events_of("<dl><dt>a<dd>b<dt>c</dl>");
    let dls = elements_named(&events, "dl");
    let dts = elements_named(&events, "dt");
    let dds = elements_named(&events, "dd");
    assert_eq!(dls.len(), 1, "must create one <dl> element");
    assert_eq!(dts.len(), 2, "must create two <dt> elements");
    assert_eq!(dds.len(), 1, "must create one <dd> element");

    let dl = dls[0];
    let dt1 = dts[0];
    let dd = dds[0];
    let dt2 = dts[1];

    assert_eq!(parent_of(&events, dt1), Some(dl));
    assert_eq!(parent_of(&events, dd), Some(dl));
    assert_eq!(parent_of(&events, dt2), Some(dl));
}

#[test]
fn flat_list_omission_yields_siblings() {
    let events = events_of("<ul><li>One<li>Two</ul>");
    let uls = elements_named(&events, "ul");
    let items = elements_named(&events, "li");
    assert_eq!(uls.len(), 1);
    assert_eq!(items.len(), 2);

    let ul = uls[0];
    assert_eq!(parent_of(&events, items[0]), Some(ul));
    assert_eq!(parent_of(&events, items[1]), Some(ul));
    assert_ne!(parent_of(&events, items[1]), Some(items[0]));
}

#[test]
fn div_inside_li_does_not_prevent_omission() {
    let events = events_of("<ul><li><div>One<li>Two</ul>");
    let uls = elements_named(&events, "ul");
    let items = elements_named(&events, "li");
    assert_eq!(uls.len(), 1);
    assert_eq!(items.len(), 2);

    let ul = uls[0];
    assert_eq!(parent_of(&events, items[0]), Some(ul));
    assert_eq!(parent_of(&events, items[1]), Some(ul));
}
