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
