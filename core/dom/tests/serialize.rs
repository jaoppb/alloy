//! `serialize_html` is deterministic, escapes text and attribute values, keeps
//! attribute insertion order, and emits void elements without a close tag
//! (v0.2 report §3 F3 step 5, §5).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dom::{
    AttributeName, AttributeValue, CommentContent, DomError, DomTree, NodeId, TagName, TextContent,
    serialize_html,
};

fn element(tree: &mut DomTree, tag: &str) -> NodeId {
    tree.create_element(TagName::new(tag).expect("valid tag"))
}

fn attribute(tree: &mut DomTree, node: NodeId, name: &str, value: &str) {
    tree.set_attribute(
        node,
        AttributeName::new(name).expect("valid attribute name"),
        AttributeValue::new(value),
    )
    .unwrap();
}

#[test]
fn a_small_tree_serializes_deterministically_with_escaping() {
    let mut tree = DomTree::new();
    let html = element(&mut tree, "html");
    let body = element(&mut tree, "body");
    let paragraph = element(&mut tree, "p");
    let text = tree.create_text(TextContent::new("Hi & <ok>"));
    tree.append_child(tree.document(), html).unwrap();
    tree.append_child(html, body).unwrap();
    tree.append_child(body, paragraph).unwrap();
    tree.append_child(paragraph, text).unwrap();
    attribute(&mut tree, paragraph, "class", "a");
    attribute(&mut tree, paragraph, "data-x", "\"q\"");

    let rendered = serialize_html(&tree, tree.document()).unwrap();

    assert_eq!(
        rendered,
        r#"<html><body><p class="a" data-x="&quot;q&quot;">Hi &amp; &lt;ok&gt;</p></body></html>"#
    );
    assert_eq!(
        rendered,
        serialize_html(&tree, tree.document()).unwrap(),
        "serialization is deterministic across calls"
    );
}

#[test]
fn void_elements_have_no_closing_tag() {
    let mut tree = DomTree::new();
    let body = element(&mut tree, "body");
    let line_break = element(&mut tree, "br");
    tree.append_child(tree.document(), body).unwrap();
    tree.append_child(body, line_break).unwrap();

    assert_eq!(
        serialize_html(&tree, tree.document()).unwrap(),
        "<body><br></body>"
    );
}

#[test]
fn attributes_keep_insertion_order_across_an_update() {
    let mut tree = DomTree::new();
    let node = element(&mut tree, "div");
    tree.append_child(tree.document(), node).unwrap();
    attribute(&mut tree, node, "id", "one");
    attribute(&mut tree, node, "role", "main");
    attribute(&mut tree, node, "id", "two");

    assert_eq!(
        serialize_html(&tree, tree.document()).unwrap(),
        r#"<div id="two" role="main"></div>"#
    );
}

#[test]
fn a_comment_node_serializes_between_delimiters_without_escaping() {
    let mut tree = DomTree::new();
    let body = element(&mut tree, "body");
    // v0.2 writes comment content raw (documented limitation) — `<`, `&` pass through.
    let comment = tree.create_comment(CommentContent::new("keep <me> & raw"));
    tree.append_child(tree.document(), body).unwrap();
    tree.append_child(body, comment).unwrap();

    assert_eq!(
        serialize_html(&tree, tree.document()).unwrap(),
        "<body><!--keep <me> & raw--></body>"
    );
}

#[test]
fn serializing_a_stale_root_is_a_node_not_found_error() {
    let mut tree = DomTree::new();
    let doomed = element(&mut tree, "div");
    tree.append_child(tree.document(), doomed).unwrap();
    tree.remove(doomed).unwrap();

    assert_eq!(
        serialize_html(&tree, doomed).unwrap_err(),
        DomError::NodeNotFound(doomed)
    );
}
