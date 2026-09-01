//! [`serialize_html`] — fold a subtree to an HTML string. Pure and deterministic
//! (v0.2 report §3, F3 step 5), and non-recursive: an explicit work stack of
//! `Step`s, never a self-call.

use crate::domain::error::DomError;
use crate::domain::node::{ElementData, NodeId, NodeKind};
use crate::domain::tree::DomTree;

/// Serialize the subtree rooted at `root`.
///
/// A `Document` root contributes only its children's markup; an element gets a
/// start tag and, unless it is a void element, an end tag; text and comment
/// nodes are written escaped / delimited. Attribute order is deterministic
/// sorted order. `Err(DomError::NodeNotFound)` when `root` does not resolve.
pub fn serialize_html(tree: &DomTree, root: NodeId) -> Result<String, DomError> {
    tree.node_kind(root)?;
    let mut output = String::new();
    let mut steps = vec![Step::Enter(root)];
    while let Some(step) = steps.pop() {
        run_step(tree, step, &mut output, &mut steps)?;
    }
    Ok(output)
}

/// One unit of pending serialization work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Enter(NodeId),
    CloseElement(NodeId),
}

fn run_step(
    tree: &DomTree,
    step: Step,
    output: &mut String,
    steps: &mut Vec<Step>,
) -> Result<(), DomError> {
    match step {
        Step::CloseElement(node) => close_element(tree, node, output),
        Step::Enter(node) => enter(tree, node, output, steps),
    }
}

fn enter(
    tree: &DomTree,
    node: NodeId,
    output: &mut String,
    steps: &mut Vec<Step>,
) -> Result<(), DomError> {
    match tree.node_kind(node)? {
        NodeKind::Document => {
            push_children(tree, node, steps);
            Ok(())
        }
        NodeKind::Text(content) => {
            output.push_str(&escape_text(content.as_str()));
            Ok(())
        }
        NodeKind::Comment(content) => {
            write_comment(output, content.as_str());
            Ok(())
        }
        NodeKind::Element(element) => {
            write_element(tree, node, element, output, steps);
            Ok(())
        }
    }
}

fn write_element(
    tree: &DomTree,
    node: NodeId,
    element: &ElementData,
    output: &mut String,
    steps: &mut Vec<Step>,
) {
    write_open_tag(output, element);
    if element.tag().is_void() {
        return;
    }
    steps.push(Step::CloseElement(node));
    push_children(tree, node, steps);
}

fn close_element(tree: &DomTree, node: NodeId, output: &mut String) -> Result<(), DomError> {
    let tag = tree.tag(node)?;
    output.push_str("</");
    output.push_str(tag.as_str());
    output.push('>');
    Ok(())
}

fn write_open_tag(output: &mut String, element: &ElementData) {
    output.push('<');
    output.push_str(element.tag().as_str());
    for (name, value) in element.attributes().iter() {
        write_attribute(output, name.as_str(), value.as_str());
    }
    output.push('>');
}

fn write_attribute(output: &mut String, name: &str, value: &str) {
    output.push(' ');
    output.push_str(name);
    output.push_str("=\"");
    output.push_str(&escape_attribute(value));
    output.push('"');
}

fn write_comment(output: &mut String, content: &str) {
    output.push_str("<!--");
    output.push_str(content);
    output.push_str("-->");
}

fn push_children(tree: &DomTree, parent: NodeId, steps: &mut Vec<Step>) {
    for child in tree.child_id_vec(parent).into_iter().rev() {
        steps.push(Step::Enter(child));
    }
}

fn escape_text(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match escape_named_entity(ch) {
            Some(entity) => escaped.push_str(entity),
            None => escaped.push(ch),
        }
    }
    escaped
}

fn escape_attribute(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch == '"' {
            escaped.push_str("&quot;");
        } else {
            match escape_named_entity(ch) {
                Some(entity) => escaped.push_str(entity),
                None => escaped.push(ch),
            }
        }
    }
    escaped
}

const fn escape_named_entity(ch: char) -> Option<&'static str> {
    match ch {
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '\u{00A0}' => Some("&nbsp;"),
        '\u{00A2}' => Some("&cent;"),
        '\u{00A3}' => Some("&pound;"),
        '\u{00A5}' => Some("&yen;"),
        '\u{00A7}' => Some("&sect;"),
        '\u{00A9}' => Some("&copy;"),
        '\u{00AB}' => Some("&laquo;"),
        '\u{00AE}' => Some("&reg;"),
        '\u{00B0}' => Some("&deg;"),
        '\u{00B1}' => Some("&plusmn;"),
        '\u{00B5}' => Some("&micro;"),
        '\u{00B7}' => Some("&middot;"),
        '\u{00BB}' => Some("&raquo;"),
        '\u{00BC}' => Some("&frac14;"),
        '\u{00BD}' => Some("&frac12;"),
        '\u{00BE}' => Some("&frac34;"),
        '\u{00D7}' => Some("&times;"),
        '\u{00F7}' => Some("&divide;"),
        '\u{2013}' => Some("&ndash;"),
        '\u{2014}' => Some("&mdash;"),
        '\u{2018}' => Some("&lsquo;"),
        '\u{2019}' => Some("&rsquo;"),
        '\u{201C}' => Some("&ldquo;"),
        '\u{201D}' => Some("&rdquo;"),
        '\u{2022}' => Some("&bull;"),
        '\u{2026}' => Some("&hellip;"),
        '\u{20AC}' => Some("&euro;"),
        '\u{2122}' => Some("&trade;"),
        '\u{2190}' => Some("&larr;"),
        '\u{2191}' => Some("&uarr;"),
        '\u{2192}' => Some("&rarr;"),
        '\u{2193}' => Some("&darr;"),
        '\u{2194}' => Some("&harr;"),
        '\u{2248}' => Some("&asymp;"),
        '\u{2260}' => Some("&ne;"),
        '\u{2264}' => Some("&le;"),
        '\u{2265}' => Some("&ge;"),
        _ => None,
    }
}
