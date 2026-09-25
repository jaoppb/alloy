//! Comprehensive test suite for CSS Variables and Custom Properties (`--*` and `var()`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "../src/domain/computed/variables.rs"]
pub mod variables;

pub mod domain {
    pub use css::domain::declaration;
    pub use css::domain::identifier;
    pub mod computed {
        pub use crate::variables;
    }
}

#[path = "../src/infrastructure/cascade/variable_values.rs"]
pub mod variable_values;

use domain::computed::variables::{
    CustomPropertiesMap, VariableError, VariableName, VariableValue,
};
use domain::declaration::{Declaration, DeclarationBlock, DeclarationValue, Importance};
use domain::identifier::Identifier;
use variable_values::{
    detect_cycle, extract_custom_properties, inherit_custom_properties,
    parse_custom_properties_block, parse_custom_property, resolve_declaration_value,
    resolve_variable, substitute_variables,
};

// ============================================================================
// 1. Storage & Domain Primitives (VariableName, VariableValue, CustomPropertiesMap)
// ============================================================================

#[test]
fn variable_name_accepts_valid_identifiers() {
    assert!(VariableName::new("--primary").is_some());
    assert!(VariableName::new("--primary-color").is_some());
    assert!(VariableName::new("--header_size_1").is_some());
    assert!(VariableName::new("--123").is_some());
    assert!(VariableName::new("--a").is_some());
    assert!(VariableName::new("--_").is_some());
}

#[test]
fn variable_name_rejects_invalid_identifiers() {
    assert_eq!(VariableName::new("-single-dash"), None);
    assert_eq!(VariableName::new("no-dash"), None);
    assert_eq!(VariableName::new("--"), None);
    assert_eq!(VariableName::new(""), None);
    assert_eq!(VariableName::new("--has space"), None);
    assert_eq!(VariableName::new("--has(paren)"), None);
    assert_eq!(VariableName::new("--has:colon"), None);
    assert_eq!(VariableName::new("--has;semicolon"), None);
    assert_eq!(VariableName::new("--has,comma"), None);
}

#[test]
fn variable_name_is_case_sensitive() {
    let lower = VariableName::new("--my-var").unwrap();
    let upper = VariableName::new("--My-Var").unwrap();
    assert_ne!(lower, upper);
    assert_eq!(lower.as_str(), "--my-var");
    assert_eq!(upper.as_str(), "--My-Var");
    assert_eq!(format!("{lower}"), "--my-var");
}

#[test]
fn variable_name_parse_returns_typed_error() {
    assert_eq!(
        VariableName::parse(""),
        Err(VariableError::EmptyVariableReference)
    );
    assert_eq!(
        VariableName::parse("invalid"),
        Err(VariableError::InvalidName("invalid".to_owned()))
    );
    assert!(VariableName::parse("--valid").is_ok());
}

#[test]
fn variable_value_trims_whitespace_and_checks_state() {
    let value = VariableValue::new("  16px solid red  ");
    assert_eq!(value.as_str(), "16px solid red");
    assert!(!value.is_empty());
    assert!(!value.contains_var());

    let empty = VariableValue::new("   ");
    assert!(empty.is_empty());

    let with_var = VariableValue::new("var(--base-size)");
    assert!(with_var.contains_var());
    assert_eq!(format!("{with_var}"), "var(--base-size)");
}

#[test]
fn custom_properties_map_basic_operations() {
    let mut map = CustomPropertiesMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);

    let color_name = VariableName::new("--main-color").unwrap();
    let color_val = VariableValue::new("#3498db");

    map.set(color_name.clone(), color_val.clone());
    assert!(!map.is_empty());
    assert_eq!(map.len(), 1);
    assert!(map.contains(&color_name));
    assert_eq!(map.get(&color_name), Some(&color_val));

    let removed = map.remove(&color_name);
    assert_eq!(removed, Some(color_val));
    assert!(map.is_empty());
    assert!(!map.contains(&color_name));
}

#[test]
fn custom_properties_map_has_deterministic_traversal_order() {
    let mut map = CustomPropertiesMap::new();
    map.set(
        VariableName::new("--zeta").unwrap(),
        VariableValue::new("3"),
    );
    map.set(
        VariableName::new("--alpha").unwrap(),
        VariableValue::new("1"),
    );
    map.set(
        VariableName::new("--beta").unwrap(),
        VariableValue::new("2"),
    );

    let keys: Vec<String> = map.iter().map(|(k, _)| k.as_str().to_owned()).collect();
    assert_eq!(keys, vec!["--alpha", "--beta", "--zeta"]);
}

// ============================================================================
// 2. Parser for Custom Property Declarations (`--*`)
// ============================================================================

#[test]
fn parse_single_custom_property_declaration() {
    let (name, value) = parse_custom_property("--theme-color", "#ff5722").unwrap();
    assert_eq!(name.as_str(), "--theme-color");
    assert_eq!(value.as_str(), "#ff5722");

    let err = parse_custom_property("theme-color", "#ff5722").unwrap_err();
    assert_eq!(err, VariableError::InvalidName("theme-color".to_owned()));
}

#[test]
fn parse_custom_properties_block_extracts_only_custom_properties() {
    let css = r"
        display: flex;
        --primary: #007bff;
        margin: 10px;
        --secondary: #6c757d;
        color: red;
        --spacing: 16px;
    ";

    let map = parse_custom_properties_block(css).unwrap();
    assert_eq!(map.len(), 3);

    let primary_name = VariableName::new("--primary").unwrap();
    let secondary_name = VariableName::new("--secondary").unwrap();
    let spacing_name = VariableName::new("--spacing").unwrap();

    assert_eq!(map.get(&primary_name), Some(&VariableValue::new("#007bff")));
    assert_eq!(
        map.get(&secondary_name),
        Some(&VariableValue::new("#6c757d"))
    );
    assert_eq!(map.get(&spacing_name), Some(&VariableValue::new("16px")));
}

#[test]
fn parse_custom_properties_block_handles_complex_values_and_parens() {
    let css = r#"
        --complex-calc: calc(100% - 20px);
        --url-with-semi: url("data:image/svg+xml;utf8,<svg></svg>");
        --shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    "#;

    let map = parse_custom_properties_block(css).unwrap();
    assert_eq!(map.len(), 3);

    let calc_name = VariableName::new("--complex-calc").unwrap();
    assert_eq!(
        map.get(&calc_name),
        Some(&VariableValue::new("calc(100% - 20px)"))
    );

    let url_name = VariableName::new("--url-with-semi").unwrap();
    assert_eq!(
        map.get(&url_name),
        Some(&VariableValue::new(
            "url(\"data:image/svg+xml;utf8,<svg></svg>\")"
        ))
    );
}

#[test]
fn extract_custom_properties_from_declaration_block() {
    let mut block = DeclarationBlock::new();
    block.push(Declaration::new(
        Identifier::new("--brand").unwrap(),
        DeclarationValue::new("coral"),
        Importance::Normal,
    ));
    block.push(Declaration::new(
        Identifier::new("margin").unwrap(),
        DeclarationValue::new("8px"),
        Importance::Normal,
    ));
    block.push(Declaration::new(
        Identifier::new("--radius").unwrap(),
        DeclarationValue::new("4px"),
        Importance::Important,
    ));

    let map = extract_custom_properties(&block);
    assert_eq!(map.len(), 2);

    let brand = VariableName::new("--brand").unwrap();
    let radius = VariableName::new("--radius").unwrap();
    assert_eq!(map.get(&brand), Some(&VariableValue::new("coral")));
    assert_eq!(map.get(&radius), Some(&VariableValue::new("4px")));
}

// ============================================================================
// 3. Resolution of `var()` Function - Basic Substitution
// ============================================================================

#[test]
fn substitute_single_variable() {
    let mut map = CustomPropertiesMap::new();
    map.set(
        VariableName::new("--bg-color").unwrap(),
        VariableValue::new("#ffffff"),
    );

    let result = substitute_variables("var(--bg-color)", &map).unwrap();
    assert_eq!(result, "#ffffff");
}

#[test]
fn substitute_multiple_variables_in_single_declaration() {
    let mut map = CustomPropertiesMap::new();
    map.set(
        VariableName::new("--border-width").unwrap(),
        VariableValue::new("2px"),
    );
    map.set(
        VariableName::new("--border-style").unwrap(),
        VariableValue::new("dashed"),
    );
    map.set(
        VariableName::new("--border-color").unwrap(),
        VariableValue::new("red"),
    );

    let result = substitute_variables(
        "var(--border-width) var(--border-style) var(--border-color)",
        &map,
    )
    .unwrap();
    assert_eq!(result, "2px dashed red");
}

#[test]
fn substitute_tolerates_internal_whitespace() {
    let mut map = CustomPropertiesMap::new();
    map.set(
        VariableName::new("--pad").unwrap(),
        VariableValue::new("12px"),
    );

    let result = substitute_variables("var(   --pad   )", &map).unwrap();
    assert_eq!(result, "12px");
}

#[test]
fn substitute_chained_variable_references() {
    let mut map = CustomPropertiesMap::new();
    map.set(
        VariableName::new("--base").unwrap(),
        VariableValue::new("8px"),
    );
    map.set(
        VariableName::new("--gutter").unwrap(),
        VariableValue::new("var(--base)"),
    );
    map.set(
        VariableName::new("--double-gutter").unwrap(),
        VariableValue::new("calc(var(--gutter) * 2)"),
    );

    let result = resolve_variable(&VariableName::new("--double-gutter").unwrap(), &map).unwrap();
    assert_eq!(result, "calc(8px * 2)");
}

#[test]
fn substitute_preserves_surrounding_text_and_quotes() {
    let mut map = CustomPropertiesMap::new();
    map.set(
        VariableName::new("--font").unwrap(),
        VariableValue::new("Roboto"),
    );

    let result = substitute_variables("14px/1.5 var(--font), sans-serif", &map).unwrap();
    assert_eq!(result, "14px/1.5 Roboto, sans-serif");

    // var() inside literal quotes should NOT be replaced
    let quoted = substitute_variables("content: 'var(--fake)'", &map).unwrap();
    assert_eq!(quoted, "content: 'var(--fake)'");
}

// ============================================================================
// 4. Fallback Support
// ============================================================================

#[test]
fn fallback_used_when_variable_undefined() {
    let map = CustomPropertiesMap::new();
    let result = substitute_variables("var(--missing, 20px)", &map).unwrap();
    assert_eq!(result, "20px");
}

#[test]
fn undefined_variable_without_fallback_errors() {
    let map = CustomPropertiesMap::new();
    let err = substitute_variables("var(--missing)", &map).unwrap_err();
    assert_eq!(
        err,
        VariableError::UndefinedVariable("--missing".to_owned())
    );
}

#[test]
fn nested_fallbacks_resolve_correctly() {
    let mut map = CustomPropertiesMap::new();
    map.set(
        VariableName::new("--theme-accent").unwrap(),
        VariableValue::new("gold"),
    );

    // First is missing, fallback references existing variable
    let result1 = substitute_variables("var(--missing, var(--theme-accent, black))", &map).unwrap();
    assert_eq!(result1, "gold");

    // Both missing, resolves to deepest default
    let result2 = substitute_variables("var(--missing1, var(--missing2, #444444))", &map).unwrap();
    assert_eq!(result2, "#444444");
}

#[test]
fn fallback_containing_commas_is_preserved_intact() {
    let map = CustomPropertiesMap::new();
    let result = substitute_variables(
        "var(--font-stack, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif)",
        &map,
    )
    .unwrap();
    assert_eq!(
        result,
        "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
    );
}

#[test]
fn empty_fallback_resolves_to_empty_string() {
    let map = CustomPropertiesMap::new();
    let result = substitute_variables("var(--missing, )", &map).unwrap();
    assert_eq!(result, "");
}

// ============================================================================
// 5. Reference Cycle Detection
// ============================================================================

#[test]
fn detect_direct_self_cycle() {
    let mut map = CustomPropertiesMap::new();
    let a = VariableName::new("--a").unwrap();
    map.set(a.clone(), VariableValue::new("var(--a)"));

    let cycle = detect_cycle(&a, &map);
    assert_eq!(cycle, Some(vec!["--a".to_owned(), "--a".to_owned()]));

    let err = resolve_variable(&a, &map).unwrap_err();
    assert_eq!(
        err,
        VariableError::CycleDetected(vec!["--a".to_owned(), "--a".to_owned()])
    );
}

#[test]
fn detect_two_step_cycle() {
    let mut map = CustomPropertiesMap::new();
    let a = VariableName::new("--a").unwrap();
    let b = VariableName::new("--b").unwrap();
    map.set(a.clone(), VariableValue::new("var(--b)"));
    map.set(b.clone(), VariableValue::new("var(--a)"));

    let cycle_a = detect_cycle(&a, &map);
    assert_eq!(
        cycle_a,
        Some(vec!["--a".to_owned(), "--b".to_owned(), "--a".to_owned()])
    );

    let cycle_b = detect_cycle(&b, &map);
    assert_eq!(
        cycle_b,
        Some(vec!["--b".to_owned(), "--a".to_owned(), "--b".to_owned()])
    );
}

#[test]
fn detect_three_step_cycle() {
    let mut map = CustomPropertiesMap::new();
    let a = VariableName::new("--a").unwrap();
    let b = VariableName::new("--b").unwrap();
    let c = VariableName::new("--c").unwrap();
    map.set(a.clone(), VariableValue::new("var(--b)"));
    map.set(b, VariableValue::new("var(--c)"));
    map.set(c, VariableValue::new("var(--a)"));

    let cycle = detect_cycle(&a, &map);
    assert_eq!(
        cycle,
        Some(vec![
            "--a".to_owned(),
            "--b".to_owned(),
            "--c".to_owned(),
            "--a".to_owned()
        ])
    );
}

#[test]
fn cyclic_reference_with_fallback_uses_fallback() {
    let mut map = CustomPropertiesMap::new();
    let a = VariableName::new("--a").unwrap();
    let b = VariableName::new("--b").unwrap();
    map.set(a, VariableValue::new("var(--b)"));
    map.set(b, VariableValue::new("var(--a)"));

    // Per CSS Custom Properties §3.1, if a variable participates in a cycle,
    // it is invalid at computed-value time. A var() with a fallback will use that fallback.
    let resolved = substitute_variables("color: var(--a, green)", &map).unwrap();
    assert_eq!(resolved, "color: green");
}

#[test]
fn cyclic_reference_inside_fallback_fails() {
    let mut map = CustomPropertiesMap::new();
    let a = VariableName::new("--a").unwrap();
    map.set(a, VariableValue::new("var(--a)"));

    // Fallback itself points to a cyclic variable without fallback
    let err = substitute_variables("var(--missing, var(--a))", &map).unwrap_err();
    assert_eq!(
        err,
        VariableError::CycleDetected(vec!["--a".to_owned(), "--a".to_owned()])
    );
}

// ============================================================================
// 6. Inheritance from Parent to Child Nodes
// ============================================================================

#[test]
fn child_inherits_parent_variables_and_overrides_locally() {
    let mut parent = CustomPropertiesMap::new();
    parent.set(
        VariableName::new("--theme-bg").unwrap(),
        VariableValue::new("#ffffff"),
    );
    parent.set(
        VariableName::new("--theme-text").unwrap(),
        VariableValue::new("#000000"),
    );
    parent.set(
        VariableName::new("--font-size").unwrap(),
        VariableValue::new("16px"),
    );

    let mut child = CustomPropertiesMap::new();
    // Child overrides background color and declares a new padding variable
    child.set(
        VariableName::new("--theme-bg").unwrap(),
        VariableValue::new("#f0f0f0"),
    );
    child.set(
        VariableName::new("--child-pad").unwrap(),
        VariableValue::new("4px"),
    );

    let computed_child_map = inherit_custom_properties(&parent, &child);

    // Child has 4 properties in total
    assert_eq!(computed_child_map.len(), 4);

    let bg = VariableName::new("--theme-bg").unwrap();
    let text = VariableName::new("--theme-text").unwrap();
    let font = VariableName::new("--font-size").unwrap();
    let pad = VariableName::new("--child-pad").unwrap();

    assert_eq!(
        computed_child_map.get(&bg),
        Some(&VariableValue::new("#f0f0f0"))
    );
    assert_eq!(
        computed_child_map.get(&text),
        Some(&VariableValue::new("#000000"))
    );
    assert_eq!(
        computed_child_map.get(&font),
        Some(&VariableValue::new("16px"))
    );
    assert_eq!(
        computed_child_map.get(&pad),
        Some(&VariableValue::new("4px"))
    );

    // Parent map remains unmodified
    assert_eq!(parent.len(), 3);
    assert_eq!(parent.get(&bg), Some(&VariableValue::new("#ffffff")));
    assert!(!parent.contains(&pad));
}

#[test]
fn multi_level_inheritance_tree() {
    let mut root = CustomPropertiesMap::new();
    root.set(
        VariableName::new("--color").unwrap(),
        VariableValue::new("red"),
    );
    root.set(
        VariableName::new("--depth").unwrap(),
        VariableValue::new("0"),
    );

    let mut parent = CustomPropertiesMap::new();
    parent.set(
        VariableName::new("--depth").unwrap(),
        VariableValue::new("1"),
    );
    let parent_map = inherit_custom_properties(&root, &parent);

    let mut child = CustomPropertiesMap::new();
    child.set(
        VariableName::new("--depth").unwrap(),
        VariableValue::new("2"),
    );
    let child_map = inherit_custom_properties(&parent_map, &child);

    let depth = VariableName::new("--depth").unwrap();
    let color = VariableName::new("--color").unwrap();

    assert_eq!(child_map.get(&depth), Some(&VariableValue::new("2")));
    assert_eq!(child_map.get(&color), Some(&VariableValue::new("red")));
}

// ============================================================================
// 7. DeclarationValue Integration
// ============================================================================

#[test]
fn resolve_declaration_value_substitutes_vars() {
    let mut map = CustomPropertiesMap::new();
    map.set(
        VariableName::new("--radius").unwrap(),
        VariableValue::new("8px"),
    );

    let decl_val = DeclarationValue::new("var(--radius)");
    let resolved = resolve_declaration_value(&decl_val, &map).unwrap();

    assert_eq!(resolved.as_str(), "8px");
}
