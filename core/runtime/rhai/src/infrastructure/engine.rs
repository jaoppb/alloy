//! [`RhaiEngine`] — the [`engine::RuntimeEngine`] implementation.

use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use engine::{CapabilitySet, EngineError, EngineValue, ExecutionLimits, RuntimeEngine};

use crate::infrastructure::context::{RhaiCompiledScript, RhaiContext};
use crate::infrastructure::error_map::{map_eval_error, map_parse_error};
use crate::infrastructure::marshal;

/// A Rhai script backend. Cheap to clone-construct a context from; holds the
/// execution ceilings and a compiler engine.
pub struct RhaiEngine {
    compiler: rhai::Engine,
    limits: ExecutionLimits,
}

impl RhaiEngine {
    /// A backend with [`ExecutionLimits::strict`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_limits(ExecutionLimits::strict())
    }

    /// A backend with explicit ceilings.
    #[must_use]
    pub fn with_limits(limits: ExecutionLimits) -> Self {
        Self {
            compiler: configured_engine(limits, Arc::new(Mutex::new(None))),
            limits,
        }
    }

    fn evaluate_ast(
        &self,
        context: &mut RhaiContext,
        ast: &rhai::AST,
    ) -> Result<EngineValue, EngineError> {
        arm_deadline(&context.deadline, self.limits.max_duration());
        let RhaiContext { engine, scope, .. } = context;
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
            engine.eval_ast_with_scope::<rhai::Dynamic>(scope, ast)
        }));
        disarm_deadline(&context.deadline);

        match outcome {
            Err(payload) => Err(EngineError::script_panic(panic_message(&*payload))),
            Ok(Err(eval_error)) => Err(map_eval_error(&eval_error)),
            Ok(Ok(dynamic)) => EngineValue::try_from(marshal::RhaiValue(dynamic)),
        }
    }
}

impl Default for RhaiEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeEngine for RhaiEngine {
    type Context = RhaiContext;
    type CompiledScript = RhaiCompiledScript;

    fn create_context(&self, capabilities: CapabilitySet) -> Result<RhaiContext, EngineError> {
        let deadline = Arc::new(Mutex::new(None));
        let engine = configured_engine(self.limits, Arc::clone(&deadline));
        Ok(RhaiContext::new(engine, capabilities, deadline))
    }

    fn compile(&self, script_source: &str) -> Result<RhaiCompiledScript, EngineError> {
        self.compiler
            .compile(script_source)
            .map(RhaiCompiledScript::new)
            .map_err(|error| map_parse_error(&error))
    }

    fn eval_value(
        &self,
        context: &mut RhaiContext,
        script_source: &str,
    ) -> Result<EngineValue, EngineError> {
        let ast = context
            .engine
            .compile(script_source)
            .map_err(|error| map_parse_error(&error))?;
        self.evaluate_ast(context, &ast)
    }

    fn eval_compiled_value(
        &self,
        context: &mut RhaiContext,
        compiled: &RhaiCompiledScript,
    ) -> Result<EngineValue, EngineError> {
        let ast = Arc::clone(&compiled.ast);
        self.evaluate_ast(context, &ast)
    }
}

fn configured_engine(
    limits: ExecutionLimits,
    deadline: Arc<Mutex<Option<Instant>>>,
) -> rhai::Engine {
    let mut engine = rhai::Engine::new();
    engine.set_max_operations(limits.max_operations());
    engine.set_max_call_levels(usize::from(limits.max_call_depth()));
    let expression_depth = usize::from(limits.max_expression_depth());
    engine.set_max_expr_depths(expression_depth, expression_depth);
    engine.on_progress(move |_operations| {
        if deadline_reached(&deadline) {
            return Some(rhai::Dynamic::UNIT);
        }
        None
    });
    engine
}

fn arm_deadline(deadline: &Arc<Mutex<Option<Instant>>>, budget: Duration) {
    let mut slot = deadline.lock().unwrap_or_else(PoisonError::into_inner);
    *slot = Instant::now().checked_add(budget);
}

fn disarm_deadline(deadline: &Arc<Mutex<Option<Instant>>>) {
    let mut slot = deadline.lock().unwrap_or_else(PoisonError::into_inner);
    *slot = None;
}

fn deadline_reached(deadline: &Arc<Mutex<Option<Instant>>>) -> bool {
    let slot = deadline.lock().unwrap_or_else(PoisonError::into_inner);
    slot.is_some_and(|instant| Instant::now() >= instant)
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&'static str>() {
        return (*text).to_owned();
    }
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    format!("native code panicked ({:?})", payload.type_id())
}
