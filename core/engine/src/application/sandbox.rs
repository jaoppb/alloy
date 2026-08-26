use crate::application::ports::{ExecutionContext, NativeFn};
use crate::domain::capability::Capability;
use crate::domain::error::EngineError;
use crate::domain::value::EngineValue;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

/// Wraps a native closure with a mandatory capability permission check (PRD-003:76, C-06, C-07).
///
/// If the calling context does not possess `required_capability`, execution is blocked
/// and `EngineError::PermissionDenied` is returned immediately.
pub fn guarded_native_fn<F>(required_capability: Capability, f: F) -> NativeFn
where
    F: Fn(&mut dyn ExecutionContext, &[EngineValue]) -> Result<EngineValue, EngineError>
        + Send
        + Sync
        + 'static,
{
    Arc::new(move |ctx, args| {
        if !ctx.capabilities().contains(required_capability) {
            return Err(EngineError::PermissionDenied(format!(
                "{required_capability:?}"
            )));
        }
        f(ctx, args)
    })
}

/// Safe execution coordinator that traps panics and recovers using fallback handlers (PRD-003:64-70, C-09).
pub struct TrappedExecutor;

impl TrappedExecutor {
    /// Executes an action with panic trapping.
    ///
    /// If the action panics or returns an error, the panic is caught without crashing the host,
    /// and `fallback` is invoked to provide a graceful default value.
    pub fn execute_with_fallback<T, F, Fallback>(action: F, fallback: Fallback) -> T
    where
        F: FnOnce() -> Result<T, EngineError>,
        Fallback: FnOnce(EngineError) -> T,
    {
        let caught = catch_unwind(AssertUnwindSafe(action));
        match caught {
            Ok(Ok(val)) => val,
            Ok(Err(err)) => fallback(err),
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic payload".to_string()
                };
                fallback(EngineError::PanicTrapped(msg))
            }
        }
    }

    /// Executes an action with panic trapping, returning `Result<T, EngineError>`.
    ///
    /// # Errors
    /// Returns `EngineError::PanicTrapped` if the closure panics, or the underlying `EngineError` if it fails.
    pub fn execute<T, F>(action: F) -> Result<T, EngineError>
    where
        F: FnOnce() -> Result<T, EngineError>,
    {
        let caught = catch_unwind(AssertUnwindSafe(action));
        match caught {
            Ok(res) => res,
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic payload".to_string()
                };
                Err(EngineError::PanicTrapped(msg))
            }
        }
    }
}
