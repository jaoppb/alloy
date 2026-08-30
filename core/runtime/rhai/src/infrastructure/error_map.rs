//! `rhai` failures → the one [`EngineError`]. No `rhai` error type crosses the
//! seam (ADR-0011 item 4).

use engine::{EngineError, ExecutionLimit, SourceLocation};
use rhai::{EvalAltResult, ParseError, Position};

/// A `rhai::Position` → an optional [`SourceLocation`] (`None` when the position
/// is not recorded).
#[must_use]
pub fn position_to_location(position: Position) -> Option<SourceLocation> {
    if position.is_none() {
        return None;
    }
    let line = position.line().unwrap_or(0);
    let column = position.position().unwrap_or(0);
    Some(SourceLocation::new(
        u32::try_from(line).unwrap_or(u32::MAX),
        u32::try_from(column).unwrap_or(u32::MAX),
    ))
}

/// Map a compile-time parse failure to [`EngineError::Compilation`]
/// (PRD-002:81).
#[must_use]
pub fn map_parse_error(error: &ParseError) -> EngineError {
    EngineError::compilation(error.to_string(), position_to_location(error.1))
}

/// Map a run-time evaluation failure to the matching [`EngineError`] variant.
#[must_use]
pub fn map_eval_error(error: &EvalAltResult) -> EngineError {
    match error {
        EvalAltResult::ErrorTooManyOperations(_) => {
            EngineError::execution_limit_exceeded(ExecutionLimit::Operations)
        }
        EvalAltResult::ErrorStackOverflow(_) => {
            EngineError::execution_limit_exceeded(ExecutionLimit::CallDepth)
        }
        EvalAltResult::ErrorTerminated(_, _) => {
            // The only source of termination in this adapter is the wall-clock
            // `on_progress` guard installed by `create_context`.
            EngineError::execution_limit_exceeded(ExecutionLimit::Duration)
        }
        EvalAltResult::ErrorParsing(_, position) => {
            EngineError::compilation(error.to_string(), position_to_location(*position))
        }
        EvalAltResult::ErrorVariableNotFound(name, position) => EngineError::script_runtime(
            format!("variable not found: {name}"),
            position_to_location(*position),
        ),
        EvalAltResult::ErrorFunctionNotFound(signature, position) => EngineError::script_runtime(
            format!("function not found: {signature}"),
            position_to_location(*position),
        ),
        EvalAltResult::ErrorMismatchOutputType(expected, actual, position) => {
            EngineError::script_runtime(
                format!("type mismatch: expected {expected}, found {actual}"),
                position_to_location(*position),
            )
        }
        EvalAltResult::ErrorSystem(_, inner) => inner
            .downcast_ref::<EngineError>()
            .cloned()
            .unwrap_or_else(|| EngineError::script_runtime(inner.to_string(), None)),
        other => {
            EngineError::script_runtime(other.to_string(), position_to_location(other.position()))
        }
    }
}
