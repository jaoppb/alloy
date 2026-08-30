//! Unit coverage for `IntoEngineValue` / `FromEngineValue` and `EngineFunction`.

// Test-only idioms in free helpers that clippy's `-in-tests` heuristic misses.
#![allow(clippy::expect_used, clippy::float_cmp, clippy::needless_pass_by_value)]

use std::collections::BTreeMap;

use engine::{Arity, EngineError, EngineFunction, EngineValue, FromEngineValue, IntoEngineValue};

fn roundtrip<T>(value: T) -> T
where
    T: IntoEngineValue + FromEngineValue + Clone,
{
    T::from_engine_value(value.into_engine_value()).expect("value roundtrips through EngineValue")
}

#[test]
fn scalars_roundtrip() {
    let () = roundtrip(());
    assert!(roundtrip(true));
    assert_eq!(roundtrip(-9_i64), -9_i64);
    assert_eq!(roundtrip(12_i32), 12_i32);
    assert_eq!(roundtrip(3.5_f64), 3.5_f64);
    assert_eq!(roundtrip(String::from("hello")), "hello");
}

#[test]
fn str_slice_projects_into_text() {
    assert_eq!(
        "borrowed".into_engine_value(),
        EngineValue::Text("borrowed".to_owned())
    );
}

#[test]
fn engine_value_is_its_own_identity_conversion() {
    let value = EngineValue::Array(vec![EngineValue::Int(1)]);
    assert_eq!(roundtrip(value.clone()), value);
}

#[test]
fn collections_roundtrip_elementwise() {
    let list = vec![1_i64, 2, 3];
    assert_eq!(roundtrip(list.clone()), list);

    let mut map = BTreeMap::new();
    map.insert("one".to_owned(), 1_i64);
    map.insert("two".to_owned(), 2_i64);
    assert_eq!(roundtrip(map.clone()), map);
}

#[test]
fn option_maps_none_to_unit_and_back() {
    assert_eq!(None::<i64>.into_engine_value(), EngineValue::Unit);
    assert_eq!(Some(5_i64).into_engine_value(), EngineValue::Int(5));
    assert_eq!(
        Option::<i64>::from_engine_value(EngineValue::Unit),
        Ok(None)
    );
    assert_eq!(
        Option::<i64>::from_engine_value(EngineValue::Int(5)),
        Ok(Some(5))
    );
}

#[test]
fn narrowing_conversions_fail_loudly() {
    let too_big = EngineValue::Int(i64::from(i32::MAX) + 1);
    let outcome = i32::from_engine_value(too_big);
    assert!(matches!(outcome, Err(EngineError::Conversion { .. })));

    assert_eq!(
        String::from_engine_value(EngineValue::Int(1)),
        Err(EngineError::type_mismatch("text", "int"))
    );
    assert_eq!(
        Vec::<i64>::from_engine_value(EngineValue::Unit),
        Err(EngineError::type_mismatch("array", "unit"))
    );
    assert_eq!(
        BTreeMap::<String, i64>::from_engine_value(EngineValue::Unit),
        Err(EngineError::type_mismatch("map", "unit"))
    );
    assert_eq!(
        <()>::from_engine_value(EngineValue::Int(0)),
        Err(EngineError::type_mismatch("unit", "int"))
    );
}

#[test]
fn nested_collection_conversion_failure_propagates() {
    let mixed = EngineValue::Array(vec![EngineValue::Int(1), EngineValue::Bool(true)]);
    let outcome = Vec::<i64>::from_engine_value(mixed);
    assert!(matches!(
        outcome,
        Err(EngineError::TypeMismatch {
            expected: "int",
            found: "bool"
        })
    ));
}

// ---- EngineFunction ---------------------------------------------------

fn invoke<Function, Args, Ret>(function: Function, arguments: &[EngineValue]) -> EngineValue
where
    Function: EngineFunction<Args, Ret>,
{
    function.invoke(arguments).expect("native call succeeds")
}

fn arity_of<Function, Args, Ret>(function: &Function) -> Arity
where
    Function: EngineFunction<Args, Ret>,
{
    function.arity()
}

#[test]
fn closures_of_each_supported_arity_are_engine_functions() {
    assert_eq!(invoke(|| 1_i64, &[]), EngineValue::Int(1));
    assert_eq!(
        invoke(|a: i64| a + 1, &[EngineValue::Int(41)]),
        EngineValue::Int(42)
    );
    assert_eq!(
        invoke(
            |a: i64, b: i64| a + b,
            &[EngineValue::Int(20), EngineValue::Int(22)]
        ),
        EngineValue::Int(42)
    );
    assert_eq!(
        invoke(
            |a: i64, b: i64, c: i64| a + b + c,
            &[
                EngineValue::Int(1),
                EngineValue::Int(2),
                EngineValue::Int(3)
            ]
        ),
        EngineValue::Int(6)
    );
    assert_eq!(
        invoke(
            |a: i64, b: i64, c: i64, d: i64| a + b + c + d,
            &[
                EngineValue::Int(1),
                EngineValue::Int(2),
                EngineValue::Int(3),
                EngineValue::Int(4),
            ]
        ),
        EngineValue::Int(10)
    );
}

#[test]
fn arity_is_reported_for_every_supported_shape() {
    assert_eq!(arity_of(&|| 0_i64), Arity::exact(0));
    assert_eq!(arity_of(&|_a: i64| 0_i64), Arity::exact(1));
    assert_eq!(arity_of(&|_a: i64, _b: i64| 0_i64), Arity::exact(2));
    assert_eq!(
        arity_of(&|_a: i64, _b: i64, _c: i64| 0_i64),
        Arity::exact(3)
    );
    assert_eq!(
        arity_of(&|_a: i64, _b: i64, _c: i64, _d: i64| 0_i64),
        Arity::exact(4)
    );
    assert_eq!(Arity::exact(2).count(), 2);
}

#[test]
fn a_call_with_the_wrong_argument_count_is_a_binding_error() {
    let function = |a: i64, b: i64| a + b;
    let wrong_count = function.invoke(&[EngineValue::Int(1)]);
    assert!(matches!(wrong_count, Err(EngineError::Binding { .. })));

    let too_many = (|| 0_i64).invoke(&[EngineValue::Int(1)]);
    assert!(matches!(too_many, Err(EngineError::Binding { .. })));
}

#[test]
fn a_wrongly_typed_argument_is_a_binding_error() {
    let function = |a: i64| a;
    let outcome = function.invoke(&[EngineValue::Text("not a number".to_owned())]);
    assert!(matches!(outcome, Err(EngineError::TypeMismatch { .. })));
}
