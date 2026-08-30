//! Unit coverage for the `domain/` value objects — no engine involved.

use std::collections::BTreeMap;

use engine::{
    Capability, CapabilitySet, EngineError, EngineValue, ExecutionLimit, ExecutionLimits,
    SourceLocation, TypeRegistration, ValueKind, profiles,
};

// ---- EngineValue --------------------------------------------------------

#[test]
fn kind_matches_the_variant() {
    assert_eq!(EngineValue::Unit.kind(), ValueKind::Unit);
    assert_eq!(EngineValue::Bool(true).kind(), ValueKind::Bool);
    assert_eq!(EngineValue::Int(1).kind(), ValueKind::Int);
    assert_eq!(EngineValue::Float(1.0).kind(), ValueKind::Float);
    assert_eq!(EngineValue::Text(String::new()).kind(), ValueKind::Text);
    assert_eq!(EngineValue::Array(vec![]).kind(), ValueKind::Array);
    assert_eq!(EngineValue::Map(BTreeMap::new()).kind(), ValueKind::Map);
    assert!(EngineValue::Unit.is_unit());
    assert!(!EngineValue::Int(0).is_unit());
}

#[test]
fn accessors_return_the_inner_value_or_a_type_mismatch() {
    assert_eq!(EngineValue::Bool(true).as_bool(), Ok(true));
    assert_eq!(EngineValue::Int(7).as_int(), Ok(7));
    assert_eq!(EngineValue::Float(2.5).as_float(), Ok(2.5));
    // an integer widens into a float request
    assert_eq!(EngineValue::Int(3).as_float(), Ok(3.0));
    assert_eq!(EngineValue::Text("hi".to_owned()).as_text(), Ok("hi"));
    assert_eq!(
        EngineValue::Array(vec![EngineValue::Int(1)]).as_array(),
        Ok(&[EngineValue::Int(1)][..])
    );

    let mut map = BTreeMap::new();
    map.insert("k".to_owned(), EngineValue::Int(1));
    assert_eq!(EngineValue::Map(map.clone()).as_map(), Ok(&map));

    assert_eq!(
        EngineValue::Int(1).as_bool(),
        Err(EngineError::type_mismatch("bool", "int"))
    );
    assert_eq!(
        EngineValue::Unit.as_text(),
        Err(EngineError::type_mismatch("text", "unit"))
    );
    assert_eq!(
        EngineValue::Unit.as_array(),
        Err(EngineError::type_mismatch("array", "unit"))
    );
    assert_eq!(
        EngineValue::Unit.as_map(),
        Err(EngineError::type_mismatch("map", "unit"))
    );
    assert_eq!(
        EngineValue::Bool(true).as_float(),
        Err(EngineError::type_mismatch("float", "bool"))
    );
}

#[test]
fn display_renders_every_shape() {
    assert_eq!(EngineValue::Unit.to_string(), "()");
    assert_eq!(EngineValue::Bool(false).to_string(), "false");
    assert_eq!(EngineValue::Int(-4).to_string(), "-4");
    assert_eq!(EngineValue::Text("x".to_owned()).to_string(), "x");
    assert_eq!(
        EngineValue::Array(vec![EngineValue::Int(1), EngineValue::Int(2)]).to_string(),
        "[1, 2]"
    );

    let mut map = BTreeMap::new();
    map.insert("a".to_owned(), EngineValue::Int(1));
    map.insert("b".to_owned(), EngineValue::Bool(true));
    assert_eq!(EngineValue::Map(map).to_string(), "{a: 1, b: true}");
}

#[test]
fn value_kind_names_are_stable() {
    for (kind, name) in [
        (ValueKind::Unit, "unit"),
        (ValueKind::Bool, "bool"),
        (ValueKind::Int, "int"),
        (ValueKind::Float, "float"),
        (ValueKind::Text, "text"),
        (ValueKind::Array, "array"),
        (ValueKind::Map, "map"),
    ] {
        assert_eq!(kind.name(), name);
        assert_eq!(kind.to_string(), name);
    }
}

// ---- EngineError -------------------------------------------------------

#[test]
fn error_display_is_human_readable() {
    let with_location = EngineError::compilation("bad token", Some(SourceLocation::new(3, 5)));
    assert_eq!(
        with_location.to_string(),
        "compilation failed at line 3, column 5: bad token"
    );

    let without_location = EngineError::script_runtime("boom", None);
    assert_eq!(without_location.to_string(), "script error: boom");

    assert_eq!(
        EngineError::execution_limit_exceeded(ExecutionLimit::Operations).to_string(),
        "execution limit exceeded: operation count"
    );
    assert_eq!(
        EngineError::permission_denied(Capability::DOM_MUTATE).to_string(),
        "permission denied: missing capability Capability(DOM_MUTATE)"
    );
    assert_eq!(
        EngineError::type_mismatch("int", "text").to_string(),
        "type mismatch: expected int, found text"
    );
    assert_eq!(
        EngineError::conversion("nope").to_string(),
        "conversion failed: nope"
    );
    assert_eq!(
        EngineError::binding("nope").to_string(),
        "native binding error: nope"
    );
    assert_eq!(
        EngineError::script_panic("kaboom").to_string(),
        "script panic (trapped): kaboom"
    );
}

#[test]
fn error_implements_std_error() {
    fn assert_error<T: std::error::Error>() {}
    assert_error::<EngineError>();
}

// ---- Capability / CapabilitySet -------------------------------------

#[test]
fn capability_set_carries_exactly_what_it_was_given() {
    let set = CapabilitySet::new(Capability::DOM_READ | Capability::GRAPHICS_DRAW);
    assert!(set.contains(Capability::DOM_READ));
    assert!(set.contains(Capability::GRAPHICS_DRAW));
    assert!(set.contains(Capability::DOM_READ | Capability::GRAPHICS_DRAW));
    assert!(!set.contains(Capability::DOM_MUTATE));
    assert_eq!(
        set.granted(),
        Capability::DOM_READ | Capability::GRAPHICS_DRAW
    );

    assert_eq!(CapabilitySet::empty(), CapabilitySet::default());
    assert!(!CapabilitySet::empty().contains(Capability::DOM_READ));
}

#[test]
fn require_reports_the_missing_flags_only() {
    let set = CapabilitySet::new(Capability::DOM_READ);
    assert_eq!(set.require(Capability::DOM_READ), Ok(()));
    assert_eq!(
        set.require(Capability::DOM_READ | Capability::DOM_MUTATE),
        Err(EngineError::permission_denied(Capability::DOM_MUTATE))
    );
}

#[test]
fn subsystem_profiles_match_prd_003() {
    assert_eq!(
        profiles::dom_parser().granted(),
        Capability::DOM_READ | Capability::DOM_MUTATE
    );
    assert_eq!(
        profiles::css_style().granted(),
        Capability::DOM_READ | Capability::GRAPHICS_DRAW
    );
    assert_eq!(
        profiles::network_interceptor().granted(),
        Capability::NETWORK_FETCH | Capability::FS_WRITE_CACHE
    );
    assert_eq!(
        profiles::ui_window().granted(),
        Capability::WINDOW_MANAGE | Capability::GRAPHICS_DRAW | Capability::DOM_READ
    );
}

// ---- ExecutionLimits ------------------------------------------------

#[test]
fn execution_limits_default_is_strict_and_is_a_builder() {
    let strict = ExecutionLimits::strict();
    assert_eq!(ExecutionLimits::default(), strict);
    assert_eq!(strict.max_operations(), 10_000_000);
    assert_eq!(strict.max_call_depth(), 64);
    assert_eq!(strict.max_expression_depth(), 128);
    assert_eq!(strict.max_duration(), std::time::Duration::from_secs(1));

    let tuned = ExecutionLimits::strict()
        .with_max_operations(5)
        .with_max_call_depth(2)
        .with_max_expression_depth(3)
        .with_max_duration(std::time::Duration::from_millis(50));
    assert_eq!(tuned.max_operations(), 5);
    assert_eq!(tuned.max_call_depth(), 2);
    assert_eq!(tuned.max_expression_depth(), 3);
    assert_eq!(tuned.max_duration(), std::time::Duration::from_millis(50));
}

#[test]
fn execution_limit_display_names_each_ceiling() {
    assert_eq!(ExecutionLimit::Operations.to_string(), "operation count");
    assert_eq!(ExecutionLimit::CallDepth.to_string(), "call depth");
    assert_eq!(
        ExecutionLimit::ExpressionDepth.to_string(),
        "expression depth"
    );
    assert_eq!(ExecutionLimit::Duration.to_string(), "time budget");
}

// ---- SourceLocation -----------------------------------------------

#[test]
fn source_location_reports_and_prints_positions() {
    let full = SourceLocation::new(10, 4);
    assert_eq!(full.line(), 10);
    assert_eq!(full.column(), 4);
    assert_eq!(full.to_string(), "line 10, column 4");

    let line_only = SourceLocation::line_only(7);
    assert_eq!(line_only.column(), 0);
    assert_eq!(line_only.to_string(), "line 7");
}

// ---- TypeRegistration -------------------------------------------------

#[test]
fn type_registration_keeps_its_script_name() {
    let registration = TypeRegistration::new("DomNode");
    assert_eq!(registration.script_name(), "DomNode");
    assert_eq!(registration, TypeRegistration::new("DomNode"));
}
