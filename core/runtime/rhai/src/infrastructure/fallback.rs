//! **C-09**: trap + fallback — the *shape*, not the payload.
//!
//! `catch_unwind` in [`RhaiEngine`][crate::RhaiEngine] already turns a panic into
//! [`EngineError::ScriptPanic`] with the host alive; this module adds the generic
//! fallback skeleton of `PRD-003:66-69`:
//!
//! 1. Run the primary closure.
//! 2. On **any** `Err` (compile, limit, permission, panic, …): write a
//!    diagnostic (with a [`SourceLocation`] when the error carries one).
//! 3. Run the caller's `default` closure.
//! 4. If that also fails, use the caller's built-in `last_resort`.
//!
//! The domain-specific instances (a bound `DomTree`, and later the muscle-policy
//! cycle) live in `rhai-bindings`; this crate owns only the skeleton, so
//! `rhai-runtime` names no domain type (v0.5 report §2.12).
//!
//! A [`PanicHookGuard`] is installed by the caller around each evaluation so the
//! default panic backtrace never reaches `stderr` (the panic is trapped
//! regardless).

use std::panic::{self, PanicHookInfo};
use std::path::Path;

use engine::{EngineError, EngineValue, SourceLocation};

/// Run `primary` and fall back safely on error.
///
/// On a primary failure, log it and run `default`; if that also fails, call
/// `last_resort`. Always returns a `T`; never panics. The second element is the
/// primary closure's return value — `Some` only when the primary ran to
/// completion, `None` on any fallback path (a fallback's return value carries no
/// meaning).
#[must_use]
pub fn run_with_fallback<T>(
    primary_path: Option<&Path>,
    primary: impl FnOnce() -> Result<(T, EngineValue), EngineError>,
    default: impl FnOnce() -> Result<T, EngineError>,
    last_resort: impl FnOnce() -> T,
) -> (T, Option<EngineValue>) {
    match primary() {
        Ok((value, returned)) => (value, Some(returned)),
        Err(error) => {
            report_failure(primary_path, &error);
            match default() {
                Ok(value) => (value, None),
                Err(error) => {
                    tracing::error!(
                        %error,
                        "the embedded default script also failed; using the built-in last resort"
                    );
                    (last_resort(), None)
                }
            }
        }
    }
}

fn report_failure(path: Option<&Path>, error: &EngineError) {
    let origin = path.map_or_else(|| "<script>".to_owned(), |path| path.display().to_string());
    if let Some(location) = source_location(error) {
        tracing::warn!(origin, %error, %location, "muscle script failed; running fallback");
    } else {
        tracing::warn!(origin, %error, "muscle script failed; running fallback");
    }
}

const fn source_location(error: &EngineError) -> Option<SourceLocation> {
    match error {
        EngineError::Compilation { location, .. } | EngineError::ScriptRuntime { location, .. } => {
            *location
        }
        _ => None,
    }
}

/// The shape `std::panic::take_hook` returns / `set_hook` accepts.
type BoxedPanicHook = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

/// Scoped guard for the process-wide panic hook.
///
/// Silences the default panic backtrace for the lifetime of the guard, then
/// restores the previous hook on drop. The panic itself is still trapped by
/// `catch_unwind` and surfaces as [`EngineError::ScriptPanic`].
pub struct PanicHookGuard {
    previous: Option<BoxedPanicHook>,
}

impl PanicHookGuard {
    #[must_use]
    pub fn install() -> Self {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(|_info| {}));
        Self {
            previous: Some(previous),
        }
    }
}

impl Drop for PanicHookGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            panic::set_hook(previous);
        }
    }
}
