//! The five `DomTree` invariants of the v0.2 report §2.2, plus value-object
//! validation and the type-checked accessors.

#![allow(clippy::unwrap_used, clippy::expect_used)]

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

    assert!(!tree.children(first_parent).any(|c| c == child));
    assert!(tree.children(second_parent).any(|c| c == child));
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
    assert!(!tree.children(tree.document()).any(|c| c == list));
}

#[test]
fn insert_before_places_the_new_child_at_the_anchor_position() {
    let mut tree = DomTree::new();
    let parent = element(&mut tree, "ul");
    let first = element(&mut tree, "li");
    let third = element(&mut tree, "li");
    tree.append_child(tree.document(), parent).unwrap();
    tree.append_child(parent, first).unwrap();
    tree.append_child(parent, third).unwrap();

    let second = element(&mut tree, "li");
    tree.insert_before(parent, second, third).unwrap();

    let order: Vec<NodeId> = tree.children(parent).collect();
    assert_eq!(order, vec![first, second, third]);
    assert_eq!(tree.parent(second).unwrap(), Some(parent));
}

#[test]
fn insert_before_moves_a_node_that_already_has_a_parent() {
    let mut tree = DomTree::new();
    let old_parent = element(&mut tree, "aside");
    let new_parent = element(&mut tree, "main");
    let anchor = element(&mut tree, "p");
    let moved = element(&mut tree, "span");
    tree.append_child(tree.document(), old_parent).unwrap();
    tree.append_child(tree.document(), new_parent).unwrap();
    tree.append_child(new_parent, anchor).unwrap();
    tree.append_child(old_parent, moved).unwrap();

    tree.insert_before(new_parent, moved, anchor).unwrap();

    assert!(!tree.children(old_parent).any(|c| c == moved));
    let order: Vec<NodeId> = tree.children(new_parent).collect();
    assert_eq!(order, vec![moved, anchor]);
}

#[test]
fn insert_before_rejects_an_anchor_that_is_not_a_child_of_the_parent() {
    let mut tree = DomTree::new();
    let parent = element(&mut tree, "ul");
    let stranger = element(&mut tree, "li");
    let newcomer = element(&mut tree, "li");
    tree.append_child(tree.document(), parent).unwrap();
    tree.append_child(tree.document(), stranger).unwrap();

    let before = tree.clone();
    assert_eq!(
        tree.insert_before(parent, newcomer, stranger).unwrap_err(),
        DomError::NodeNotFound(stranger)
    );
    assert_eq!(tree, before, "a rejected insert must not mutate the tree");
}

#[test]
fn detach_unlinks_a_node_without_tombstoning_it() {
    let mut tree = DomTree::new();
    let parent = element(&mut tree, "div");
    let child = element(&mut tree, "span");
    tree.append_child(tree.document(), parent).unwrap();
    tree.append_child(parent, child).unwrap();

    tree.detach(child).unwrap();

    assert!(!tree.children(parent).any(|c| c == child));
    assert_eq!(tree.parent(child).unwrap(), None);
    // still in the arena — detach is not remove
    assert!(tree.node_kind(child).is_ok());
}

#[test]
fn doubly_linked_sibling_pointers_are_coherent() {
    let mut tree = DomTree::new();
    let parent = element(&mut tree, "ul");
    let first = element(&mut tree, "li");
    let second = element(&mut tree, "li");
    let third = element(&mut tree, "li");
    tree.append_child(tree.document(), parent).unwrap();
    tree.append_child(parent, first).unwrap();
    tree.append_child(parent, second).unwrap();
    tree.append_child(parent, third).unwrap();

    assert_eq!(tree.first_child(parent).unwrap(), Some(first));
    assert_eq!(tree.last_child(parent).unwrap(), Some(third));

    assert_eq!(tree.previous_sibling(first).unwrap(), None);
    assert_eq!(tree.next_sibling(first).unwrap(), Some(second));

    assert_eq!(tree.previous_sibling(second).unwrap(), Some(first));
    assert_eq!(tree.next_sibling(second).unwrap(), Some(third));

    assert_eq!(tree.previous_sibling(third).unwrap(), Some(second));
    assert_eq!(tree.next_sibling(third).unwrap(), None);

    // detach middle node
    tree.detach(second).unwrap();
    assert_eq!(tree.first_child(parent).unwrap(), Some(first));
    assert_eq!(tree.last_child(parent).unwrap(), Some(third));
    assert_eq!(tree.next_sibling(first).unwrap(), Some(third));
    assert_eq!(tree.previous_sibling(third).unwrap(), Some(first));
    assert_eq!(tree.previous_sibling(second).unwrap(), None);
    assert_eq!(tree.next_sibling(second).unwrap(), None);
    assert_eq!(tree.parent(second).unwrap(), None);
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
    assert_eq!(TagName::new("div").unwrap(), TagName::Div);
    assert_eq!(
        TagName::new("custom-card").unwrap(),
        TagName::Custom("custom-card".into())
    );
    assert!(matches!(
        AttributeName::new("bad=name"),
        Err(DomError::InvalidAttributeName(_))
    ));
}

#[test]
fn node_id_root_identifies_document() {
    let tree = DomTree::new();
    assert_eq!(NodeId::root(), tree.document());
    assert_eq!(NodeId::root().index(), 0);
}

#[test]
fn thiserror_display_messages_are_formatted_correctly() {
    let id = NodeId::root();
    assert_eq!(
        DomError::NodeNotFound(id).to_string(),
        "node #0 does not exist"
    );
    assert_eq!(
        DomError::WouldCycle.to_string(),
        "operation would make the tree cyclic"
    );
    assert_eq!(
        DomError::SelfParent.to_string(),
        "a node cannot be its own parent"
    );
    assert_eq!(
        DomError::CannotDetachDocument.to_string(),
        "the document root cannot be detached or removed"
    );
    assert_eq!(
        DomError::CannotHaveChildren(id).to_string(),
        "node #0 cannot hold children"
    );
    assert_eq!(
        DomError::InvalidTagName("123".into()).to_string(),
        "not a valid tag name: \"123\""
    );
    assert_eq!(
        DomError::InvalidAttributeName("a b".into()).to_string(),
        "not a valid attribute name: \"a b\""
    );
    assert_eq!(
        DomError::NotAnElement(id).to_string(),
        "node #0 is not an element"
    );
    assert_eq!(
        DomError::NotCharacterData(id).to_string(),
        "node #0 is not character data"
    );
}
