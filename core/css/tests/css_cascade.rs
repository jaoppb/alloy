use css::{
    Color, ColorResolver, CssColorResolver, CssError, DeclarationList, DisplayType, PropertyName,
    Px, Rule, Selector, Specificity, StyleCascade, parse_css,
};
use dom::{AttributeMap, AttributeName, AttributeValue, DomTree, TagName};
use html::parse_html;

#[test]
fn test_parse_stylesheet_rules_and_declarations() {
    let css = r#"
        /* Base styles */
        body {
            background-color: #ffffff;
            color: black;
            margin: 0px;
        }
        h1, h2 {
            color: red;
            font-size: 24px;
        }
    "#;

    let stylesheet = parse_css(css).expect("CSS should parse cleanly");
    assert_eq!(stylesheet.rules().len(), 2);

    let mut rules_iter = stylesheet.rules().iter();
    let body_rule = rules_iter.next().unwrap();
    assert_eq!(body_rule.selectors().len(), 1);
    assert_eq!(
        body_rule.declarations().get(&PropertyName::new("color")),
        Some(&css::PropertyValue::Color(Color::BLACK))
    );

    let heading_rule = rules_iter.next().unwrap();
    assert_eq!(heading_rule.selectors().len(), 2);
    assert_eq!(
        heading_rule
            .declarations()
            .get(&PropertyName::new("font-size")),
        Some(&css::PropertyValue::Length(Px::new(24.0)))
    );
}

#[test]
fn test_specificity_ordering() {
    let universal = Selector::Universal.specificity();
    let tag = Selector::Tag(TagName::new("div").unwrap()).specificity();
    let class = Selector::Class("container".to_string()).specificity();
    let id = Selector::Id("header".to_string()).specificity();

    let div_p = Selector::Descendant(
        Box::new(Selector::Tag(TagName::new("div").unwrap())),
        Box::new(Selector::Tag(TagName::new("p").unwrap())),
    )
    .specificity();

    let div_class = Selector::Descendant(
        Box::new(Selector::Tag(TagName::new("div").unwrap())),
        Box::new(Selector::Class("lead".to_string())),
    )
    .specificity();

    assert_eq!(universal, Specificity::new(0, 0, 0));
    assert_eq!(tag, Specificity::new(0, 0, 1));
    assert_eq!(class, Specificity::new(0, 1, 0));
    assert_eq!(id, Specificity::new(1, 0, 0));
    assert_eq!(div_p, Specificity::new(0, 0, 2));
    assert_eq!(div_class, Specificity::new(0, 1, 1));

    // Assert strict ordering: ID > Class > Tag > Universal
    assert!(id > div_class);
    assert!(div_class > class);
    assert!(class > div_p);
    assert!(div_p > tag);
    assert!(tag > universal);
}

#[test]
fn test_selector_matching_against_dom() {
    let mut tree = DomTree::new();
    let doc_id = tree.create_document();

    let mut div_attrs = AttributeMap::new();
    div_attrs.insert(AttributeName::new("id"), AttributeValue::new("main-box"));
    div_attrs.insert(
        AttributeName::new("class"),
        AttributeValue::new("wrapper card active"),
    );

    let div_id = tree.create_element(TagName::new("div").unwrap(), div_attrs);
    tree.append_child(doc_id, div_id).unwrap();

    let p_id = tree.create_element(TagName::new("p").unwrap(), AttributeMap::new());
    tree.append_child(div_id, p_id).unwrap();

    // 1. Universal
    assert!(Selector::Universal.matches(div_id, &tree));

    // 2. Tag
    assert!(Selector::Tag(TagName::new("div").unwrap()).matches(div_id, &tree));
    assert!(!Selector::Tag(TagName::new("span").unwrap()).matches(div_id, &tree));

    // 3. Class (tests split by whitespace)
    assert!(Selector::Class("card".to_string()).matches(div_id, &tree));
    assert!(Selector::Class("active".to_string()).matches(div_id, &tree));
    assert!(!Selector::Class("inactive".to_string()).matches(div_id, &tree));

    // 4. ID
    assert!(Selector::Id("main-box".to_string()).matches(div_id, &tree));
    assert!(!Selector::Id("other-box".to_string()).matches(div_id, &tree));

    // 5. Descendant: div p
    let descendant_sel = Selector::Descendant(
        Box::new(Selector::Tag(TagName::new("div").unwrap())),
        Box::new(Selector::Tag(TagName::new("p").unwrap())),
    );
    assert!(descendant_sel.matches(p_id, &tree));
    assert!(!descendant_sel.matches(div_id, &tree));
}

#[test]
fn test_cascade_specificity_override() {
    let html = r#"<div id="target" class="lead">Text</div>"#;
    let dom = parse_html(html).expect("DOM parsing");

    // CSS with three rules of differing specificity:
    // div (0, 0, 1) -> color: black
    // .lead (0, 1, 0) -> color: blue
    // #target (1, 0, 0) -> color: red
    let css = r#"
        #target { color: red; }
        .lead { color: blue; }
        div { color: black; }
    "#;
    let stylesheet = parse_css(css).expect("CSS parsing");

    let styled_tree = StyleCascade::build_styled_tree(&dom, &stylesheet);
    let root = styled_tree.root().expect("Root styled node");

    // Root is document; first child is <div>
    let div_styled = root.children().first().expect("Div styled node");
    assert_eq!(
        div_styled.style().color(),
        Color::RED,
        "ID rule with highest specificity must override class and tag rules"
    );
}

#[test]
fn test_style_inheritance_and_styled_tree() {
    let html = r#"<div id="parent"><p>Child paragraph</p></div>"#;
    let dom = parse_html(html).expect("DOM parsing");

    let css = r#"
        #parent {
            color: blue;
            font-size: 28px;
            display: block;
        }
    "#;
    let stylesheet = parse_css(css).expect("CSS parsing");

    let styled_tree = StyleCascade::build_styled_tree(&dom, &stylesheet);
    let root = styled_tree.root().expect("Root node");

    let parent_node = root.children().first().expect("Parent div");
    assert_eq!(parent_node.style().color(), Color::BLUE);
    assert_eq!(parent_node.style().font_size(), Px::new(28.0));
    assert_eq!(parent_node.style().display(), DisplayType::Block);

    let child_node = parent_node.children().first().expect("Child p");
    assert_eq!(
        child_node.style().color(),
        Color::BLUE,
        "Child <p> must inherit color from parent"
    );
    assert_eq!(
        child_node.style().font_size(),
        Px::new(28.0),
        "Child <p> must inherit font-size from parent"
    );
}

#[test]
fn test_cascade_source_order_precedence_on_tie() {
    let html = r#"<div class="box">Text</div>"#;
    let dom = parse_html(html).expect("DOM parsing");

    let css = r#"
        .box {
            color: red;
        }
        .box {
            color: blue;
        }
    "#;
    let stylesheet = parse_css(css).expect("CSS parsing");

    let styled_tree = StyleCascade::build_styled_tree(&dom, &stylesheet);
    let root = styled_tree.root().expect("Root node");
    let div_node = root.children().first().expect("Div node");

    assert_eq!(
        div_node.style().color(),
        Color::BLUE,
        "When specificity is tied, the last rule in source order must win"
    );
}

#[test]
fn test_rule_invariants() {
    let empty_selectors: Vec<Selector> = Vec::new();
    let result = Rule::new(empty_selectors, DeclarationList::new());
    assert!(matches!(result, Err(CssError::InvalidSelector(_))));

    let valid_rule = Rule::new(vec![Selector::Universal], DeclarationList::new());
    assert!(valid_rule.is_ok());
}

#[test]
fn test_color_resolver_adapter_and_rebeccapurple() {
    let resolver = CssColorResolver::new();
    assert_eq!(
        resolver.resolve("rebeccapurple"),
        Some(Color::rgba(102, 51, 153, 255))
    );
    assert_eq!(
        Color::parse("rebeccapurple"),
        Some(Color::rgba(102, 51, 153, 255))
    );
    assert_eq!(
        Color::parse("#663399"),
        Some(Color::rgba(102, 51, 153, 255))
    );
    assert_eq!(Color::parse("#fff"), Some(Color::rgba(255, 255, 255, 255)));
    assert_eq!(Color::parse("#00ff0080"), Some(Color::rgba(0, 255, 0, 128)));
    assert_eq!(
        resolver.resolve("aliceblue"),
        Some(Color::rgba(240, 248, 255, 255))
    );
}

#[test]
fn test_w3c_property_name_enum() {
    let color = PropertyName::new("color");
    assert_eq!(color, PropertyName::Color);
    assert_eq!(color.as_str(), "color");

    let bg = PropertyName::new("BACKGROUND-COLOR");
    assert_eq!(bg, PropertyName::BackgroundColor);
    assert_eq!(bg.as_str(), "background-color");

    let custom = PropertyName::new("--primary-brand-color");
    assert_eq!(
        custom,
        PropertyName::Custom("--primary-brand-color".to_string())
    );
    assert_eq!(custom.as_str(), "--primary-brand-color");
}

#[test]
fn test_selectors_level_3_combinators_attributes_and_pseudoclasses() {
    let css = r#"
        /* Child combinator */
        ul > li { color: red; }
        /* Adjacent sibling combinator */
        h1 + p { color: green; }
        /* General sibling combinator */
        h1 ~ span { color: blue; }
        /* Attribute selectors */
        input[type="text"] { color: yellow; }
        a[href^="https"] { color: purple; }
        img[src$=".png"] { width: 100px; }
        div[class*="box"] { display: block; }
        /* Pseudo-classes */
        li:first-child { font-weight: bold; }
        div:empty { display: none; }
        /* Compound selector */
        p.intro#lead[data-highlight=true] { color: cyan; }
    "#;

    let sheet = parse_css(css).expect("CSS Level 3 parsing must succeed");
    assert_eq!(sheet.rules().len(), 10);

    // Verify Specificities
    // ul > li: 2 tags -> (0, 0, 2)
    let r1 = sheet.rules().iter().next().unwrap();
    assert_eq!(r1.selectors()[0].specificity().as_tuple(), (0, 0, 2));

    // p.intro#lead[data-highlight=true]: 1 id, 2 classes/attributes, 1 tag -> (1, 2, 1)
    let last_rule = sheet.rules().iter().last().unwrap();
    assert_eq!(last_rule.selectors()[0].specificity().as_tuple(), (1, 2, 1));

    // Verify DOM Matching
    let mut tree = DomTree::new();
    let doc = tree.create_document();
    let ul = tree.create_element(TagName::new("ul").unwrap(), AttributeMap::new());
    let li1 = tree.create_element(TagName::new("li").unwrap(), AttributeMap::new());
    let li2 = tree.create_element(TagName::new("li").unwrap(), AttributeMap::new());
    tree.append_child(doc, ul).unwrap();
    tree.append_child(ul, li1).unwrap();
    tree.append_child(ul, li2).unwrap();

    let child_sel = &r1.selectors()[0]; // ul > li
    assert!(child_sel.matches(li1, &tree));
    assert!(child_sel.matches(li2, &tree));
    assert!(!child_sel.matches(ul, &tree));

    // Child combinator (>) must NOT match non-direct descendant
    let inner_div = tree.create_element(TagName::new("div").unwrap(), AttributeMap::new());
    let nested_li = tree.create_element(TagName::new("li").unwrap(), AttributeMap::new());
    tree.append_child(ul, inner_div).unwrap();
    tree.append_child(inner_div, nested_li).unwrap();
    assert!(!child_sel.matches(nested_li, &tree));

    let first_child_sel = &sheet.rules().iter().nth(7).unwrap().selectors()[0]; // li:first-child
    assert!(first_child_sel.matches(li1, &tree));
    assert!(!first_child_sel.matches(li2, &tree));
}
