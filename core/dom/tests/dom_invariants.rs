use dom::{AttributeMap, AttributeName, AttributeValue, DomError, DomService, DomTree, TagName};

#[test]
fn test_dom_tree_construction_and_hierarchy() {
    let mut tree = DomTree::new();

    let doc = tree.create_document();
    let html = tree.create_element(TagName::new("html").unwrap(), AttributeMap::new());
    let body = tree.create_element(TagName::new("body").unwrap(), AttributeMap::new());

    let mut h1_attrs = AttributeMap::new();
    h1_attrs.insert(
        AttributeName::new("class"),
        AttributeValue::new("main-title"),
    );
    let h1 = tree.create_element(TagName::new("h1").unwrap(), h1_attrs);
    let text = tree.create_text("Hello Alloy");

    tree.append_child(doc, html).unwrap();
    tree.append_child(html, body).unwrap();
    tree.append_child(body, h1).unwrap();
    tree.append_child(h1, text).unwrap();

    assert_eq!(tree.root(), Some(doc));
    assert_eq!(tree.get(html).unwrap().parent(), Some(doc));
    assert_eq!(tree.get(body).unwrap().parent(), Some(html));
    assert_eq!(tree.get(h1).unwrap().parent(), Some(body));
    assert_eq!(tree.get(text).unwrap().parent(), Some(h1));

    assert_eq!(tree.get(h1).unwrap().children().len(), 1);
    assert_eq!(tree.get(h1).unwrap().children().as_slice(), &[text]);

    let html_serialized = DomService::serialize_to_html(&tree, doc);
    assert_eq!(
        html_serialized,
        "<html><body><h1 class=\"main-title\">Hello Alloy</h1></body></html>"
    );
}

#[test]
fn test_acyclicity_prevents_cycles() {
    let mut tree = DomTree::new();

    let parent = tree.create_element(TagName::new("div").unwrap(), AttributeMap::new());
    let child = tree.create_element(TagName::new("p").unwrap(), AttributeMap::new());
    let grandchild = tree.create_element(TagName::new("span").unwrap(), AttributeMap::new());

    tree.append_child(parent, child).unwrap();
    tree.append_child(child, grandchild).unwrap();

    // 1. Self cycle: append node to itself
    let self_cycle = tree.append_child(parent, parent);
    assert!(matches!(self_cycle, Err(DomError::CycleDetected { .. })));

    // 2. Direct cycle: append parent to child
    let direct_cycle = tree.append_child(child, parent);
    assert!(matches!(direct_cycle, Err(DomError::CycleDetected { .. })));

    // 3. Indirect cycle: append parent to grandchild
    let indirect_cycle = tree.append_child(grandchild, parent);
    assert!(matches!(
        indirect_cycle,
        Err(DomError::CycleDetected { .. })
    ));
}

#[test]
fn test_reparenting_removes_from_previous_parent() {
    let mut tree = DomTree::new();

    let p1 = tree.create_element(TagName::new("div").unwrap(), AttributeMap::new());
    let p2 = tree.create_element(TagName::new("section").unwrap(), AttributeMap::new());
    let child = tree.create_element(TagName::new("p").unwrap(), AttributeMap::new());

    tree.append_child(p1, child).unwrap();
    assert_eq!(tree.get(p1).unwrap().children().len(), 1);
    assert_eq!(tree.get(child).unwrap().parent(), Some(p1));

    // Move child to p2
    tree.append_child(p2, child).unwrap();

    // Invariant: child was detached from p1 and now belongs exclusively to p2
    assert_eq!(tree.get(p1).unwrap().children().len(), 0);
    assert_eq!(tree.get(p2).unwrap().children().len(), 1);
    assert_eq!(tree.get(child).unwrap().parent(), Some(p2));
}

#[test]
fn test_insert_before_ordering() {
    let mut tree = DomTree::new();

    let list = tree.create_element(TagName::new("ul").unwrap(), AttributeMap::new());
    let item1 = tree.create_element(TagName::new("li").unwrap(), AttributeMap::new());
    let item3 = tree.create_element(TagName::new("li").unwrap(), AttributeMap::new());
    let item2 = tree.create_element(TagName::new("li").unwrap(), AttributeMap::new());

    tree.append_child(list, item1).unwrap();
    tree.append_child(list, item3).unwrap();

    // Insert item2 before item3
    tree.insert_before(list, item2, item3).unwrap();

    let children = tree.get(list).unwrap().children();
    assert_eq!(children.as_slice(), &[item1, item2, item3]);
}

#[test]
fn test_remove_child() {
    let mut tree = DomTree::new();

    let parent = tree.create_element(TagName::new("div").unwrap(), AttributeMap::new());
    let child1 = tree.create_element(TagName::new("span").unwrap(), AttributeMap::new());
    let child2 = tree.create_element(TagName::new("p").unwrap(), AttributeMap::new());

    tree.append_child(parent, child1).unwrap();
    tree.append_child(parent, child2).unwrap();

    tree.remove_child(parent, child1).unwrap();

    assert_eq!(tree.get(parent).unwrap().children().as_slice(), &[child2]);
    assert_eq!(tree.get(child1).unwrap().parent(), None);
}

#[test]
fn test_depth_first_pre_order_traversal() {
    let mut tree = DomTree::new();

    let root = tree.create_element(TagName::new("html").unwrap(), AttributeMap::new());
    let head = tree.create_element(TagName::new("head").unwrap(), AttributeMap::new());
    let body = tree.create_element(TagName::new("body").unwrap(), AttributeMap::new());
    let p = tree.create_element(TagName::new("p").unwrap(), AttributeMap::new());

    tree.append_child(root, head).unwrap();
    tree.append_child(root, body).unwrap();
    tree.append_child(body, p).unwrap();

    let order = tree.traverse_pre_order(root);
    assert_eq!(order, vec![root, head, body, p]);
}

#[test]
fn test_deep_acyclicity_prevents_indirect_ancestor_cycles() {
    let mut tree = DomTree::new();

    let a = tree.create_element(TagName::new("div").unwrap(), AttributeMap::new());
    let b = tree.create_element(TagName::new("div").unwrap(), AttributeMap::new());
    let c = tree.create_element(TagName::new("div").unwrap(), AttributeMap::new());
    let d = tree.create_element(TagName::new("div").unwrap(), AttributeMap::new());

    tree.append_child(a, b).unwrap();
    tree.append_child(b, c).unwrap();
    tree.append_child(c, d).unwrap();

    // Attempting to append root ancestor 'a' under deep descendant 'd' must fail
    let err = tree.append_child(d, a);
    assert_eq!(
        err,
        Err(dom::DomError::CycleDetected { node: a, parent: d })
    );
}

#[test]
fn test_insert_before_with_unrelated_reference_node_fails() {
    let mut tree = DomTree::new();

    let parent1 = tree.create_element(TagName::new("div").unwrap(), AttributeMap::new());
    let child1 = tree.create_element(TagName::new("span").unwrap(), AttributeMap::new());
    tree.append_child(parent1, child1).unwrap();

    let parent2 = tree.create_element(TagName::new("div").unwrap(), AttributeMap::new());
    let child2 = tree.create_element(TagName::new("span").unwrap(), AttributeMap::new());
    tree.append_child(parent2, child2).unwrap();

    let new_node = tree.create_element(TagName::new("p").unwrap(), AttributeMap::new());

    // Attempting insert_before on parent1 with child2 (which belongs to parent2) must fail
    let err = tree.insert_before(parent1, new_node, child2);
    assert!(
        matches!(err, Err(dom::DomError::InvalidHierarchy(ref msg)) if msg.contains("not a child of parent")),
        "Expected InvalidHierarchy, got {err:?}"
    );
}
