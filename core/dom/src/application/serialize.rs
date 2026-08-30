//! [`serialize_html`] — fold a subtree to an HTML string. Pure and deterministic
//! (v0.2 report §3, F3 step 5), and non-recursive: an explicit work stack of
//! `Step`s, never a self-call.

use crate::domain::error::DomError;
use crate::domain::node::{ElementData, NodeId, NodeKind};
use crate::domain::tree::DomTree;

/// Serialize the subtree rooted at `root`. A `Document` root contributes only
/// its children's markup; an element gets a start tag and, unless it is a void
/// element, an end tag; text and comment nodes are written escaped / delimited.
/// Attribute order is insertion order. `Err(DomError::NodeNotFound)` when `root`
/// does not resolve.
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
    if is_void(element.tag().as_str()) {
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
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attribute(raw: &str) -> String {
    escape_text(raw).replace('"', "&quot;")
}

fn is_void(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}
