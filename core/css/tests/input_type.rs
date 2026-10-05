//! [`css::InputType`] — the closed vocabulary of an `<input>`'s `type`
//! attribute (WHATWG HTML §4.10.5) — and the label the styled tree synthesizes
//! from it for button-like controls (issues #2 / #3).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use css::{CascadeResolver, InputType, StyleSheetSet, UaCascade, snapshot};

// ---- parsing ------------------------------------------------------------------

#[test]
fn the_type_keyword_matches_ascii_case_insensitively() {
    assert_eq!(
        InputType::from_attribute(Some("SUBMIT")),
        InputType::SubmitButton
    );
    assert_eq!(
        InputType::from_attribute(Some("Reset")),
        InputType::ResetButton
    );
    assert_eq!(
        InputType::from_attribute(Some("datetime-local")),
        InputType::LocalDateAndTime
    );
    assert_eq!(InputType::from_attribute(Some("tel")), InputType::Telephone);
}

#[test]
fn a_missing_or_invalid_type_is_the_text_state() {
    assert_eq!(InputType::from_attribute(None), InputType::Text);
    assert_eq!(InputType::from_attribute(Some("bogus")), InputType::Text);
    assert_eq!(InputType::from_attribute(Some("")), InputType::Text);
    assert_eq!(
        InputType::from_attribute(Some(" submit")),
        InputType::Text,
        "the keyword is matched exactly, not trimmed"
    );
    assert_eq!(InputType::default(), InputType::Text);
}

#[test]
fn keywords_are_the_canonical_lowercase_spelling() {
    assert_eq!(InputType::SubmitButton.keyword(), "submit");
    assert_eq!(InputType::RadioButton.keyword(), "radio");
    assert_eq!(InputType::FileUpload.keyword(), "file");
}

// ---- labels -------------------------------------------------------------------

#[test]
fn button_states_have_default_labels_and_others_have_none() {
    assert_eq!(
        InputType::SubmitButton.default_label(),
        Some("Submit Query")
    );
    assert_eq!(InputType::ResetButton.default_label(), Some("Reset"));
    assert_eq!(InputType::Button.default_label(), Some(""));
    assert_eq!(InputType::Text.default_label(), None);
    assert_eq!(InputType::Checkbox.default_label(), None);
}

#[test]
fn a_value_attribute_overrides_the_default_label() {
    assert_eq!(InputType::SubmitButton.label(Some("Go")), Some("Go"));
    assert_eq!(InputType::SubmitButton.label(None), Some("Submit Query"));
    assert_eq!(
        InputType::Text.label(Some("typed")),
        None,
        "a text field's value is not a painted label"
    );
}

// ---- synthesis in the styled tree --------------------------------------------

/// The synthesized label of one `<{tag}>` carrying `attributes`.
fn synthesized_label(tag: &str, attributes: &[(&str, &str)]) -> Option<String> {
    let mut tree = dom::DomTree::new();
    let root = tree.document();
    let node = tree.create_element(html::TagName::new(tag).unwrap());
    tree.append_child(root, node).unwrap();
    for (name, value) in attributes {
        tree.set_attribute(
            node,
            html::AttributeName::new(name).unwrap(),
            html::AttributeValue::new(*value),
        )
        .unwrap();
    }
    let dom = snapshot(&tree, root);
    let styled = UaCascade::new()
        .resolve(&dom, &StyleSheetSet::new())
        .expect("the cascade resolves");
    let element = dom.nodes_in_document_order().nth(1).expect("the element");
    let styled_node = styled.node(element).expect("styled");
    styled_node.text().map(|run| run.as_str().to_owned())
}

#[test]
fn a_submit_input_synthesizes_its_label_whatever_the_keyword_case() {
    assert_eq!(
        synthesized_label("input", &[("type", "SUBMIT")]),
        Some("Submit Query".to_owned())
    );
    assert_eq!(
        synthesized_label("input", &[("type", "submit"), ("value", "Send")]),
        Some("Send".to_owned())
    );
    assert_eq!(
        synthesized_label("input", &[("type", "reset")]),
        Some("Reset".to_owned())
    );
}

#[test]
fn inputs_without_a_visible_label_synthesize_nothing() {
    assert_eq!(synthesized_label("input", &[("type", "button")]), None);
    assert_eq!(synthesized_label("input", &[("type", "text")]), None);
    assert_eq!(synthesized_label("input", &[]), None);
    assert_eq!(synthesized_label("input", &[("type", "bogus")]), None);
    assert_eq!(
        synthesized_label("button", &[("type", "submit")]),
        None,
        "only an `<input>` element synthesizes a label"
    );
}
