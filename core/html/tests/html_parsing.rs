use dom::{AttributeName, DomService, NodeData, TagName};
use html::parse_html;

#[test]
fn test_parse_full_document() {
    let html = "<!DOCTYPE html><html><head><title>Alloy Test</title></head><body><h1>Main Title</h1><p>Paragraph text.</p></body></html>";
    let tree = parse_html(html).expect("Parse should succeed");

    let root = tree.root().expect("Root document should be present");
    let serialized = DomService::serialize_to_html(&tree, root);

    assert_eq!(
        serialized,
        "<html><head><title>Alloy Test</title></head><body><h1>Main Title</h1><p>Paragraph text.</p></body></html>"
    );
}

#[test]
fn test_parse_attributes_and_quotes() {
    let html = r#"<div id="main" class='container flex' data-active="true"><span class="badge">Badge</span></div>"#;
    let tree = parse_html(html).expect("Parse should succeed");

    let root = tree.root().unwrap();
    let divs = DomService::find_by_tag_name(&tree, root, &TagName::new("div").unwrap());
    assert_eq!(divs.len(), 1);

    let div_node = tree.get(divs[0]).unwrap();
    match div_node.data() {
        NodeData::Element { attributes, .. } => {
            assert_eq!(
                attributes.get(&AttributeName::new("id")).unwrap().as_str(),
                "main"
            );
            assert_eq!(
                attributes
                    .get(&AttributeName::new("class"))
                    .unwrap()
                    .as_str(),
                "container flex"
            );
            assert_eq!(
                attributes
                    .get(&AttributeName::new("data-active"))
                    .unwrap()
                    .as_str(),
                "true"
            );
        }
        _ => panic!("Expected Element node"),
    }
}

#[test]
fn test_parse_void_elements() {
    let html = r#"<div><p>Before<br><img src="test.png"/>After</p><hr></div>"#;
    let tree = parse_html(html).expect("Parse should succeed");

    let root = tree.root().unwrap();
    let serialized = DomService::serialize_to_html(&tree, root);

    assert_eq!(
        serialized,
        "<div><p>Before<br></br><img src=\"test.png\"></img>After</p><hr></hr></div>"
    );
}

#[test]
fn test_parse_entities() {
    let html = "<p>Salt &amp; Pepper &lt;3 &quot;Quote&quot;</p>";
    let tree = parse_html(html).expect("Parse should succeed");

    let root = tree.root().unwrap();
    let text = DomService::get_text_content(&tree, root);

    assert_eq!(text, "Salt & Pepper <3 \"Quote\"");
}

#[test]
fn test_parse_comments() {
    let html = "<div><!-- comment here --><span>visible</span></div>";
    let tree = parse_html(html).expect("Parse should succeed");

    let root = tree.root().unwrap();
    let serialized = DomService::serialize_to_html(&tree, root);

    assert_eq!(
        serialized,
        "<div><!-- comment here --><span>visible</span></div>"
    );
}

#[test]
fn test_parse_unclosed_tags_resilience() {
    let html = "<div><p>First paragraph<div>Nested div</div>";
    let tree = parse_html(html).expect("Parse should succeed");

    assert!(tree.node_count() >= 4);
    let root = tree.root().unwrap();
    let text = DomService::get_text_content(&tree, root);
    assert_eq!(text, "First paragraphNested div");
}

#[test]
fn test_parse_unquoted_and_boolean_attributes() {
    let html = r#"<input type=checkbox checked disabled class=active>"#;
    let tree = parse_html(html).expect("Parse should succeed");

    let root = tree.root().unwrap();
    let inputs = DomService::find_by_tag_name(&tree, root, &TagName::new("input").unwrap());
    assert_eq!(inputs.len(), 1);

    let input_node = tree.get(inputs[0]).unwrap();
    if let NodeData::Element { attributes, .. } = input_node.data() {
        assert_eq!(
            attributes
                .get(&AttributeName::new("type"))
                .unwrap()
                .as_str(),
            "checkbox"
        );
        assert_eq!(
            attributes
                .get(&AttributeName::new("checked"))
                .unwrap()
                .as_str(),
            ""
        );
        assert_eq!(
            attributes
                .get(&AttributeName::new("disabled"))
                .unwrap()
                .as_str(),
            ""
        );
        assert_eq!(
            attributes
                .get(&AttributeName::new("class"))
                .unwrap()
                .as_str(),
            "active"
        );
    } else {
        panic!("Expected Element node");
    }
}

#[test]
fn test_parse_case_insensitivity() {
    let html = "<DIV CLASS='container'><P>Hello</P></DIV>";
    let tree = parse_html(html).expect("Parse should succeed");

    let root = tree.root().unwrap();
    let divs = DomService::find_by_tag_name(&tree, root, &TagName::new("div").unwrap());
    assert_eq!(divs.len(), 1, "Tag DIV must normalize to div");

    let ps = DomService::find_by_tag_name(&tree, root, &TagName::new("p").unwrap());
    assert_eq!(ps.len(), 1, "Tag P must normalize to p");
}
