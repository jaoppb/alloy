//! The five `DomTree` invariants of the v0.2 report §2.2, plus value-object
//! validation and the type-checked accessors.

use dom::{AttributeName, AttributeValue, DomError, DomTree, NodeId, TagName, TextContent};

fn element(tree: &mut DomTree, tag: &str) -> NodeId {
    tree.create_element(TagName::new(tag).expect("valid tag"))
}

#[test]
fn append_child_creating_a_cycle_is_refused_and_leaves_the_tree_untouched() {
    let mut tree = DomTree::new();
    let outer = element(&mut tree, "section");
    let inner = element(&mut tree, "p");
    tree.append_child(tree.document(), outer).unwrap();
    tree.append_child(outer, inner).unwrap();

    let before = tree.clone();
    let error = tree.append_child(inner, outer).unwrap_err();

    assert_eq!(error, DomError::WouldCycle);
    assert_eq!(tree, before, "a refused append must not mutate the tree");
}

#[test]
fn attaching_a_node_that_already_has_a_parent_moves_it() {
    let mut tree = DomTree::new();
    let first_parent = element(&mut tree, "header");
    let second_parent = element(&mut tree, "footer");
    let child = element(&mut tree, "span");
    tree.append_child(tree.document(), first_parent).unwrap();
    tree.append_child(tree.document(), second_parent).unwrap();
    tree.append_child(first_parent, child).unwrap();

    tree.append_child(second_parent, child).unwrap();

    assert!(!tree.child_ids(first_parent).unwrap().contains(child));
    assert!(tree.child_ids(second_parent).unwrap().contains(child));
    assert_eq!(tree.parent(child).unwrap(), Some(second_parent));
}

#[test]
fn the_document_root_cannot_be_detached_or_removed() {
    let mut tree = DomTree::new();
    assert_eq!(
        tree.detach(tree.document()).unwrap_err(),
        DomError::CannotDetachDocument
    );
    assert_eq!(
        tree.remove(tree.document()).unwrap_err(),
        DomError::CannotDetachDocument
    );
}

#[test]
fn a_node_cannot_be_its_own_parent() {
    let mut tree = DomTree::new();
    let node = element(&mut tree, "div");
    assert_eq!(
        tree.append_child(node, node).unwrap_err(),
        DomError::SelfParent
    );
}

#[test]
fn remove_tombstones_the_whole_subtree() {
    let mut tree = DomTree::new();
    let list = element(&mut tree, "ul");
    let item = element(&mut tree, "li");
    let label = tree.create_text(TextContent::new("item"));
    tree.append_child(tree.document(), list).unwrap();
    tree.append_child(list, item).unwrap();
    tree.append_child(item, label).unwrap();

    tree.remove(list).unwrap();

    assert_eq!(
        tree.node_kind(list).unwrap_err(),
        DomError::NodeNotFound(list)
    );
    assert_eq!(
        tree.node_kind(item).unwrap_err(),
        DomError::NodeNotFound(item)
    );
    assert_eq!(
        tree.node_kind(label).unwrap_err(),
        DomError::NodeNotFound(label)
    );
    assert!(!tree.child_ids(tree.document()).unwrap().contains(list));
}

#[test]
fn a_text_or_comment_node_cannot_hold_children() {
    let mut tree = DomTree::new();
    let text = tree.create_text(TextContent::new("leaf"));
    let orphan = element(&mut tree, "b");

    assert_eq!(
        tree.append_child(text, orphan).unwrap_err(),
        DomError::CannotHaveChildren(text)
    );
}

#[test]
fn character_data_and_element_operations_are_type_checked() {
    let mut tree = DomTree::new();
    let div = element(&mut tree, "div");
    let text = tree.create_text(TextContent::new("hi"));

    assert_eq!(
        tree.set_text(div, TextContent::new("x")).unwrap_err(),
        DomError::NotCharacterData(div)
    );
    let name = AttributeName::new("class").unwrap();
    assert_eq!(
        tree.set_attribute(text, name, AttributeValue::new("x"))
            .unwrap_err(),
        DomError::NotAnElement(text)
    );
}

#[test]
fn invalid_tag_and_attribute_names_are_rejected() {
    assert!(matches!(TagName::new(""), Err(DomError::InvalidTagName(_))));
    assert!(matches!(
        TagName::new("1bad"),
        Err(DomError::InvalidTagName(_))
    ));
    assert!(matches!(
        TagName::new("has space"),
        Err(DomError::InvalidTagName(_))
    ));
    assert_eq!(TagName::new("DIV").unwrap().as_str(), "div");
    assert!(matches!(
        AttributeName::new("bad=name"),
        Err(DomError::InvalidAttributeName(_))
    ));
}
