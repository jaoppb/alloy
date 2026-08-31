//! `MockEngine` — the in-repo **reference adapter** for the [`RuntimeEngine`]
//! port (ADR-0011 item 6), and the proof of **C-05**: a consumer written
//! generically over `RuntimeEngine` runs unchanged on an engine that has nothing
//! to do with `rhai`.
//!
//! It interprets a deliberately tiny language — just enough to exercise every
//! method of the port:
//!
//! | source            | result                                             |
//! | ----------------- | -------------------------------------------------- |
//! | `()` or empty     | `Unit`                                              |
//! | `true` / `false`  | `Bool`                                              |
//! | `-?[0-9]+`        | `Int`                                               |
//! | `"…"`             | `Text` (no escapes; a lone `"` is a compile error) |
//! | `name`            | scope variable lookup                               |
//! | `name()`          | registered native function call (no args)           |
//! | `expr + expr`     | integer addition                                    |
//! | anything else     | `EngineError::Compilation`                          |

use std::collections::HashMap;

use engine::{
    Arity, Capability, CapabilitySet, EngineError, EngineType, EngineValue, ExecutionContext,
    FunctionName, NativeFn, RuntimeEngine, SourceLocation, TypeRegistration, VariableName,
};

// ---------------------------------------------------------------------------
// The adapter
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct MockEngine;

impl MockEngine {
    const fn new() -> Self {
        Self
    }
}

struct MockContext {
    capabilities: CapabilitySet,
    variables: HashMap<String, EngineValue>,
    functions: HashMap<String, NativeFn>,
    registered_types: Vec<TypeRegistration>,
}

impl MockContext {
    fn new(capabilities: CapabilitySet) -> Self {
        Self {
            capabilities,
            variables: HashMap::new(),
            functions: HashMap::new(),
            registered_types: Vec::new(),
        }
    }

    const fn registered_type_count(&self) -> usize {
        self.registered_types.len()
    }
}

#[derive(Debug)]
struct MockScript {
    expression: Expression,
}

impl RuntimeEngine for MockEngine {
    type Context = MockContext;
    type CompiledScript = MockScript;

    fn create_context(&self, capabilities: CapabilitySet) -> Result<MockContext, EngineError> {
        Ok(MockContext::new(capabilities))
    }

    fn compile(&self, script_source: &str) -> Result<MockScript, EngineError> {
        let expression = parse(script_source)?;
        Ok(MockScript { expression })
    }

    fn eval_value(
        &self,
        context: &mut MockContext,
        script_source: &str,
    ) -> Result<EngineValue, EngineError> {
        let expression = parse(script_source)?;
        evaluate(&expression, context)
    }

    fn eval_compiled_value(
        &self,
        context: &mut MockContext,
        compiled: &MockScript,
    ) -> Result<EngineValue, EngineError> {
        evaluate(&compiled.expression, context)
    }
}

impl ExecutionContext for MockContext {
    fn capabilities(&self) -> CapabilitySet {
        self.capabilities
    }

    fn register_type_erased(&mut self, registration: TypeRegistration) -> Result<(), EngineError> {
        self.registered_types.push(registration);
        Ok(())
    }

    fn register_native_fn(
        &mut self,
        name: &FunctionName,
        _arity: Arity,
        handler: NativeFn,
    ) -> Result<(), EngineError> {
        self.functions.insert(name.as_str().to_owned(), handler);
        Ok(())
    }

    fn set_value(&mut self, name: &VariableName, value: EngineValue) -> Result<(), EngineError> {
        self.variables.insert(name.as_str().to_owned(), value);
        Ok(())
    }

    fn get_value(&self, name: &VariableName) -> Option<EngineValue> {
        self.variables.get(name.as_str()).cloned()
    }

    fn call_function_value(
        &mut self,
        name: &FunctionName,
        arguments: &[EngineValue],
    ) -> Result<EngineValue, EngineError> {
        let handler = self
            .functions
            .get(name.as_str())
            .cloned()
            .ok_or_else(|| EngineError::binding(format!("unknown function `{name}`")))?;
        handler(arguments)
    }

    fn reset_scope(&mut self) -> Result<(), EngineError> {
        self.variables.clear();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The tiny interpreter
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Expression {
    Unit,
    Bool(bool),
    Int(i64),
    Text(String),
    Variable(String),
    Call(String),
    Add(Box<Self>, Box<Self>),
}

const ORIGIN: SourceLocation = SourceLocation::new(1, 1);

fn parse(source: &str) -> Result<Expression, EngineError> {
    let trimmed = source.trim();

    if trimmed.is_empty() || trimmed == "()" {
        return Ok(Expression::Unit);
    }
    if trimmed == "true" {
        return Ok(Expression::Bool(true));
    }
    if trimmed == "false" {
        return Ok(Expression::Bool(false));
    }
    if let Ok(number) = trimmed.parse::<i64>() {
        return Ok(Expression::Int(number));
    }
    if let Some(rest) = trimmed.strip_prefix('"') {
        let inner = rest
            .strip_suffix('"')
            .ok_or_else(|| EngineError::compilation("unterminated string literal", Some(ORIGIN)))?;
        return Ok(Expression::Text(inner.to_owned()));
    }
    if let Some((left, right)) = split_addition(trimmed) {
        let left_expression = parse(left)?;
        let right_expression = parse(right)?;
        return Ok(Expression::Add(
            Box::new(left_expression),
            Box::new(right_expression),
        ));
    }
    if let Some(name) = trimmed
        .strip_suffix("()")
        .filter(|candidate| is_identifier(candidate))
    {
        return Ok(Expression::Call(name.to_owned()));
    }
    if is_identifier(trimmed) {
        return Ok(Expression::Variable(trimmed.to_owned()));
    }
    Err(EngineError::compilation(
        format!("unrecognised syntax: `{trimmed}`"),
        Some(ORIGIN),
    ))
}

fn split_addition(source: &str) -> Option<(&str, &str)> {
    let (left, right) = source.split_once('+')?;
    if left.trim().is_empty() || right.trim().is_empty() {
        return None;
    }
    Some((left.trim(), right.trim()))
}

fn is_identifier(candidate: &str) -> bool {
    let mut characters = candidate.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {
            characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
        }
        _ => false,
    }
}

fn evaluate(expression: &Expression, context: &MockContext) -> Result<EngineValue, EngineError> {
    match expression {
        Expression::Unit => Ok(EngineValue::Unit),
        Expression::Bool(value) => Ok(EngineValue::Bool(*value)),
        Expression::Int(value) => Ok(EngineValue::Int(*value)),
        Expression::Text(value) => Ok(EngineValue::Text(value.clone())),
        Expression::Variable(name) => context.variables.get(name).cloned().ok_or_else(|| {
            EngineError::script_runtime(format!("unbound variable `{name}`"), Some(ORIGIN))
        }),
        Expression::Call(name) => call_zero_arg(name, context),
        Expression::Add(left, right) => add(left, right, context),
    }
}

fn call_zero_arg(name: &str, context: &MockContext) -> Result<EngineValue, EngineError> {
    let handler = context.functions.get(name).ok_or_else(|| {
        EngineError::script_runtime(format!("unknown function `{name}`"), Some(ORIGIN))
    })?;
    handler(&[])
}

fn add(
    left: &Expression,
    right: &Expression,
    context: &MockContext,
) -> Result<EngineValue, EngineError> {
    let left_number = evaluate(left, context)?.as_int()?;
    let right_number = evaluate(right, context)?.as_int()?;
    let sum = left_number
        .checked_add(right_number)
        .ok_or_else(|| EngineError::script_runtime("integer overflow in `+`", Some(ORIGIN)))?;
    Ok(EngineValue::Int(sum))
}

// ---------------------------------------------------------------------------
// C-05 and the conformance suite
// ---------------------------------------------------------------------------

/// A domain-side consumer. Generic over the engine — it never names a concrete
/// backend. In F2 the very same function is handed a `RhaiEngine`.
fn evaluate_subject<Engine: RuntimeEngine>(engine: &Engine) -> Result<String, EngineError> {
    let mut context = engine.create_context(CapabilitySet::empty())?;
    context.set_variable(&VariableName::parse("subject")?, "world")?;
    engine.eval::<String>(&mut context, "subject")
}

#[test]
fn engine_is_replaceable_without_touching_the_consumer() {
    let subject = evaluate_subject(&MockEngine::new()).expect("mock engine evaluates the consumer");
    assert_eq!(subject, "world");
}

#[test]
fn mock_engine_passes_core_conformance() {
    engine::conformance::run_core_suite(MockEngine::new);
}

// ---------------------------------------------------------------------------
// Adapter-local checks
// ---------------------------------------------------------------------------

#[test]
fn unterminated_string_is_a_compilation_error_with_a_location() {
    let engine = MockEngine::new();
    let error = engine
        .compile("\"oops")
        .expect_err("a lone quote must not compile");
    match error {
        EngineError::Compilation { location, .. } => {
            assert_eq!(location, Some(SourceLocation::new(1, 1)));
        }
        other => panic!("expected Compilation, got {other:?}"),
    }
}

#[test]
fn addition_actually_evaluates() {
    let engine = MockEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("context");
    context
        .set_variable(&VariableName::parse("base").expect("valid name"), 40_i64)
        .expect("set base");
    let total: i64 = engine.eval(&mut context, "base + 2").expect("evaluate sum");
    assert_eq!(total, 42);
}

#[test]
fn unbound_variable_is_a_runtime_error_not_a_panic() {
    let engine = MockEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("context");
    let outcome = engine.eval::<EngineValue>(&mut context, "ghost");
    assert!(matches!(outcome, Err(EngineError::ScriptRuntime { .. })));
}

struct Widget;

impl EngineType for Widget {
    fn registration() -> TypeRegistration {
        TypeRegistration::new("Widget")
    }
}

#[test]
fn register_type_is_recorded() {
    let engine = MockEngine::new();
    let mut context = engine
        .create_context(CapabilitySet::empty())
        .expect("context");
    context.register_type::<Widget>().expect("register Widget");
    assert_eq!(context.registered_type_count(), 1);
}

#[test]
fn a_denied_capability_is_reported_by_require() {
    let granted = CapabilitySet::new(Capability::DOM_READ);
    let denied = granted.require(Capability::DOM_MUTATE);
    assert_eq!(
        denied,
        Err(EngineError::permission_denied(Capability::DOM_MUTATE)),
    );
}
