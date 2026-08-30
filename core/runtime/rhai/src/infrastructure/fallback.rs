//! **C-09**: trap + fallback. `catch_unwind` in
//! [`RhaiEngine`][crate::RhaiEngine] already turns a panic into
//! [`EngineError::ScriptPanic`] with the host alive; this module adds the
//! fallback of `PRD-003:66-69`:
//!
//! 1. Run the primary script into a fresh [`DomTree`].
//! 2. On **any** `Err` (compile, limit, permission, panic, DOM): write a
//!    diagnostic to `stderr` (the DevTools event bus of `PRD-003:67` is a stub
//!    in v0.2).
//! 3. Run the embedded `default_dom.rhai` in a **new** guarded context over a
//!    **clean** tree — never the partial one.
//! 4. If that also fails, build `<html><body></body></html>` in Rust with
//!    [`minimal_document`].
//!
//! A [`PanicHookGuard`] is installed only around each evaluation so the default
//! panic backtrace never reaches `stderr` (the panic is trapped regardless).

use std::panic::{self, PanicHookInfo};
use std::path::Path;
use std::sync::{Arc, Mutex};

use dom::{DomTree, TagName};
use engine::{CapabilitySet, EngineError, EngineValue, RuntimeEngine, SourceLocation};

use crate::RhaiEngine;

/// Run `primary_source` with a bound DOM; on failure log it and fall back to
/// `default_dom_source`, then to [`minimal_document`]. Always returns a
/// well-formed tree; never panics. The second element is the primary script's
/// return value — `Some` only when the primary ran to completion, `None` on any
/// fallback path (a fallback script's return value carries no meaning).
#[must_use]
pub fn run_with_fallback(
    engine: &RhaiEngine,
    capabilities: CapabilitySet,
    primary_source: &str,
    primary_path: Option<&Path>,
    default_dom_source: &str,
) -> (DomTree, Option<EngineValue>) {
    match evaluate_into_tree(engine, capabilities, primary_source) {
        Ok((tree, value)) => (tree, Some(value)),
        Err(error) => {
            report_failure(primary_path, &error);
            (recover(engine, capabilities, default_dom_source), None)
        }
    }
}

fn recover(engine: &RhaiEngine, capabilities: CapabilitySet, default_dom_source: &str) -> DomTree {
    match evaluate_into_tree(engine, capabilities, default_dom_source) {
        Ok((tree, _value)) => tree,
        Err(error) => {
            eprintln!(
                "alloy: the embedded default DOM script also failed ({error}); \
                 using the built-in minimal document"
            );
            minimal_document()
        }
    }
}

fn evaluate_into_tree(
    engine: &RhaiEngine,
    capabilities: CapabilitySet,
    source: &str,
) -> Result<(DomTree, EngineValue), EngineError> {
    let tree = Arc::new(Mutex::new(DomTree::new()));
    let mut context = engine.create_context(capabilities)?;
    context.bind_dom(Arc::clone(&tree))?;

    let value = {
        let _quiet = PanicHookGuard::install();
        engine.eval_value(&mut context, source)?
    };

    drop(context);
    Ok((unwrap_tree(tree), value))
}

fn unwrap_tree(tree: Arc<Mutex<DomTree>>) -> DomTree {
    match Arc::try_unwrap(tree) {
        Ok(mutex) => mutex
            .into_inner()
            .unwrap_or_else(|poison| poison.into_inner()),
        Err(shared) => shared
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone(),
    }
}

fn report_failure(path: Option<&Path>, error: &EngineError) {
    let origin = path.map_or_else(|| "<script>".to_owned(), |path| path.display().to_string());
    eprintln!("alloy: muscle script `{origin}` failed: {error}");
    if let Some(location) = source_location(error) {
        eprintln!("       at {location}");
    }
}

fn source_location(error: &EngineError) -> Option<SourceLocation> {
    match error {
        EngineError::Compilation { location, .. } | EngineError::ScriptRuntime { location, .. } => {
            *location
        }
        _ => None,
    }
}

/// The last-resort document: `<html><body></body></html>`, built without a
/// script. This routine is not optional — a fault in the embedded fallback
/// script must not reopen the hole C-09 closes.
#[must_use]
pub fn minimal_document() -> DomTree {
    let mut tree = DomTree::new();
    let html = tree.create_element(TagName::new("html").expect("`html` is a valid tag"));
    let body = tree.create_element(TagName::new("body").expect("`body` is a valid tag"));
    tree.append_child(tree.document(), html)
        .expect("a fresh document accepts a child");
    tree.append_child(html, body)
        .expect("a fresh element accepts a child");
    tree
}

/// The shape `std::panic::take_hook` returns / `set_hook` accepts.
type BoxedPanicHook = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

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
