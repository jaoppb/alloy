//! Corpus test verifying parsing of real-world pages (class-example.com) into a correct [`dom::DomTree`].

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use html::parse;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixtures/example_com.html")
}

#[test]
fn test_example_com_corpus_parsing() {
    let content =
        std::fs::read_to_string(fixture_path()).expect("Fixture example_com.html must exist");
    let tree = parse(&content).expect("Parsing example_com.html must succeed");

    let doc = tree.document();
    let descendants: Vec<_> = tree.descendants(doc).collect();
    assert!(!descendants.is_empty(), "DOM tree must not be empty");

    // Verify html, head, body structure
    let mut found_html = false;
    let mut found_head = false;
    let mut found_body = false;
    let mut found_title = false;
    let mut found_h1 = false;
    let mut found_script = false;
    let mut p_count = 0;
    let mut li_count = 0;

    for &node in &descendants {
        if let Ok(dom::NodeKind::Element(elem)) = tree.node_kind(node) {
            match elem.tag().as_str() {
                "html" => found_html = true,
                "head" => found_head = true,
                "body" => found_body = true,
                "title" => found_title = true,
                "h1" => found_h1 = true,
                "script" => found_script = true,
                "p" => p_count += 1,
                "li" => li_count += 1,
                _ => {}
            }
        }
    }

    assert!(found_html, "<html> must be present");
    assert!(found_head, "<head> must be present");
    assert!(found_body, "<body> must be present");
    assert!(found_title, "<title> must be present");
    assert!(found_h1, "<h1> must be present");
    assert!(found_script, "<script> must be present");
    assert_eq!(
        p_count, 2,
        "Must have exactly 2 <p> elements via tag omission"
    );
    assert_eq!(
        li_count, 2,
        "Must have exactly 2 <li> elements via tag omission"
    );

    // Verify script rawtext isolation: no element node exists as a descendant of script!
    let script_node = descendants
        .iter()
        .copied()
        .find(|&n| matches!(tree.node_kind(n), Ok(dom::NodeKind::Element(e)) if e.tag().as_str() == "script"))
        .expect("script node");

    let script_children: Vec<_> = tree.children(script_node).collect();
    assert_eq!(
        script_children.len(),
        1,
        "Script node should only contain 1 text node child"
    );
    match tree.node_kind(script_children[0]) {
        Ok(dom::NodeKind::Text(content)) => {
            assert!(
                content
                    .as_str()
                    .contains("<div>This inner string is not an element</div>"),
                "Script text must retain raw string content"
            );
        }
        other => panic!("Expected text node inside script, got {other:?}"),
    }

    // Verify that the tree can be serialized deterministically
    let serialized = dom::serialize_html(&tree, doc).expect("Serialization must succeed");
    assert!(serialized.contains("Example Domain"));
    assert!(serialized.contains("Documentation reference"));
}
