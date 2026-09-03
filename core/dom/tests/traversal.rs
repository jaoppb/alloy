//! `Descendants` visits in document order; `Ancestors` ends at the `Document`
//! root; both are inert on a stale id (v0.2 report §2.3, §5).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dom::{DomTree, NodeId, TagName};

fn element(tree: &mut DomTree, tag: &str) -> NodeId {
    tree.create_element(TagName::new(tag).expect("valid tag"))
}

#[test]
fn descendants_are_visited_in_document_order() {
    let mut tree = DomTree::new();
    let html = element(&mut tree, "html");
    let head = element(&mut tree, "head");
    let body = element(&mut tree, "body");
    let title = element(&mut tree, "title");
    let paragraph = element(&mut tree, "p");
    tree.append_child(tree.document(), html).unwrap();
    tree.append_child(html, head).unwrap();
    tree.append_child(html, body).unwrap();
    tree.append_child(head, title).unwrap();
    tree.append_child(body, paragraph).unwrap();

    let order: Vec<NodeId> = tree.descendants(tree.document()).collect();

    assert_eq!(order, vec![html, head, title, body, paragraph]);
}

#[test]
fn ancestors_walk_up_to_and_including_the_document() {
    let mut tree = DomTree::new();
    let html = element(&mut tree, "html");
    let body = element(&mut tree, "body");
    let paragraph = element(&mut tree, "p");
    tree.append_child(tree.document(), html).unwrap();
    tree.append_child(html, body).unwrap();
    tree.append_child(body, paragraph).unwrap();

    let chain: Vec<NodeId> = tree.ancestors(paragraph).collect();

    assert_eq!(chain, vec![body, html, tree.document()]);
}

#[test]
fn traversal_from_a_stale_id_yields_nothing() {
    let mut tree = DomTree::new();
    let doomed = element(&mut tree, "div");
    tree.append_child(tree.document(), doomed).unwrap();
    tree.remove(doomed).unwrap();

    assert_eq!(tree.children(doomed).count(), 0);
    assert_eq!(tree.descendants(doomed).count(), 0);
    assert_eq!(tree.ancestors(doomed).count(), 0);
}

#[test]
fn children_iterator_yields_direct_children_in_order() {
    let mut tree = DomTree::new();
    let parent = element(&mut tree, "ul");
    let c1 = element(&mut tree, "li");
    let c2 = element(&mut tree, "li");
    let c3 = element(&mut tree, "li");
    tree.append_child(tree.document(), parent).unwrap();
    tree.append_child(parent, c1).unwrap();
    tree.append_child(parent, c2).unwrap();
    tree.append_child(parent, c3).unwrap();

    let children: Vec<NodeId> = tree.children(parent).collect();
    assert_eq!(children, vec![c1, c2, c3]);
}
