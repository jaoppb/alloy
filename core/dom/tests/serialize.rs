//! `serialize_html` is deterministic, escapes text and attribute values, sorts
//! attributes alphabetically, and emits void elements without a close tag
//! (v0.2 report §3 F3 step 5, §5).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dom::{
    AttributeName, AttributeValue, CommentContent, DomError, DomTree, HtmlEntity, NodeId, TagName,
    TextContent, serialize_html,
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
    let image = element(&mut tree, "img");
    tree.append_child(tree.document(), body).unwrap();
    tree.append_child(body, line_break).unwrap();
    tree.append_child(body, image).unwrap();
    attribute(&mut tree, image, "src", "logo.png");

    assert_eq!(
        serialize_html(&tree, tree.document()).unwrap(),
        r#"<body><br><img src="logo.png"></body>"#
    );
}

#[test]
fn attributes_are_sorted_deterministically_via_btree_map() {
    let mut tree = DomTree::new();
    let node = element(&mut tree, "div");
    tree.append_child(tree.document(), node).unwrap();
    attribute(&mut tree, node, "role", "main");
    attribute(&mut tree, node, "id", "one");
    attribute(&mut tree, node, "class", "box");
    attribute(&mut tree, node, "id", "two");

    assert_eq!(
        serialize_html(&tree, tree.document()).unwrap(),
        r#"<div class="box" id="two" role="main"></div>"#
    );
}

#[test]
fn custom_elements_serialize_with_open_and_close_tags() {
    let mut tree = DomTree::new();
    let custom = element(&mut tree, "custom-card");
    let text = tree.create_text(TextContent::new("Hello Custom"));
    tree.append_child(tree.document(), custom).unwrap();
    tree.append_child(custom, text).unwrap();

    assert_eq!(
        serialize_html(&tree, tree.document()).unwrap(),
        "<custom-card>Hello Custom</custom-card>"
    );
}

#[test]
fn full_w3c_named_entities_are_escaped_in_text_and_attributes() {
    let mut tree = DomTree::new();
    let p = element(&mut tree, "p");
    let text = tree.create_text(TextContent::new(
        "Alloy © 2026 • 100€ & 50£ \u{00A0} trade™",
    ));
    tree.append_child(tree.document(), p).unwrap();
    tree.append_child(p, text).unwrap();
    attribute(&mut tree, p, "data-symbol", "© & \"quoted\"");

    assert_eq!(
        serialize_html(&tree, tree.document()).unwrap(),
        r#"<p data-symbol="&copy; &amp; &quot;quoted&quot;">Alloy &copy; 2026 &bull; 100&euro; &amp; 50&pound; &nbsp; trade&trade;</p>"#
    );
}

#[test]
fn html_entity_bidirectional_lookups() {
    assert_eq!(HtmlEntity::from_char('&'), Some(HtmlEntity::Amp));
    assert_eq!(HtmlEntity::from_char('©'), Some(HtmlEntity::Copy));
    assert_eq!(HtmlEntity::from_char('€'), Some(HtmlEntity::Euro));
    assert_eq!(HtmlEntity::from_char('z'), None);

    assert_eq!(HtmlEntity::from_name("copy"), Some(HtmlEntity::Copy));
    assert_eq!(HtmlEntity::from_name("euro"), Some(HtmlEntity::Euro));
    assert_eq!(HtmlEntity::from_name("unknown"), None);

    assert_eq!(HtmlEntity::Copy.as_char(), '©');
    assert_eq!(HtmlEntity::Copy.as_entity(), "&copy;");
    assert_eq!(HtmlEntity::Copy.entity_name(), "copy");
    assert_eq!(HtmlEntity::Copy.to_string(), "&copy;");
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
