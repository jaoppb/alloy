//! DOM-specific instances of the `rhai-runtime` fallback skeleton (**C-09**) and
//! the [`bind_dom`] bridge that stamps a host-owned [`DomTree`] into a context.
//!
//! `rhai-runtime` owns only the *shape* of "run primary → default → last resort"
//! ([`rhai_runtime::run_with_fallback`]); the tree it produces, and the built-in
//! [`minimal_document`] last resort, live here so the backend names no domain
//! type (v0.5 report §2.12).

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use dom::{DomTree, TagName};
use engine::{
    CapabilitySet, EngineError, EngineValue, ExecutionContext, RuntimeEngine, VariableName,
};
use rhai_runtime::{PanicHookGuard, RhaiContext, RhaiEngine, run_with_fallback};

use crate::dom_bindings::NodeHandle;

/// Bind a host-owned [`DomTree`] into `context` as the global `document` handle
/// (**C-03**, roadmap I1).
///
/// Registers [`NodeHandle`] as a script type and stamps the handle with the
/// context's capability set — `ADR-0004` fixes capabilities at context creation,
/// so baking the set into the handle is sound. Every DOM binding then self-guards
/// (**C-06**); a missing capability is [`EngineError::PermissionDenied`]
/// (**C-07**). The caller keeps the `Arc` it passes in and reads the mutated tree
/// back through it after evaluation returns (`ADR-0003`).
pub fn bind_dom(context: &mut RhaiContext, tree: Arc<Mutex<DomTree>>) -> Result<(), EngineError> {
    context.register_custom_type::<NodeHandle>()?;
    let root = tree
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .document();
    let capabilities = context.capabilities();
    let document = NodeHandle::new(tree, root, capabilities);
    let name = VariableName::parse("document")?;
    context.set_custom_value(&name, document);
    Ok(())
}

/// Run `primary_source` with a bound DOM and fall back safely on error
/// (`PRD-003:66-69`).
///
/// On **any** `Err` (compile, limit, permission, panic, DOM), log a diagnostic
/// and run the embedded `default_dom_source` over a **clean** tree; if that also
/// fails, use [`minimal_document`]. Always returns a well-formed tree; never
/// panics. The second element is the primary script's return value — `Some` only
/// when the primary ran to completion.
#[must_use]
pub fn run_dom_with_fallback(
    engine: &RhaiEngine,
    capabilities: CapabilitySet,
    primary_source: &str,
    primary_path: Option<&Path>,
    default_dom_source: &str,
) -> (DomTree, Option<EngineValue>) {
    run_with_fallback(
        primary_path,
        || evaluate_into_tree(engine, capabilities, primary_source),
        || evaluate_into_tree(engine, capabilities, default_dom_source).map(|(tree, _value)| tree),
        minimal_document,
    )
}

fn evaluate_into_tree(
    engine: &RhaiEngine,
    capabilities: CapabilitySet,
    source: &str,
) -> Result<(DomTree, EngineValue), EngineError> {
    let tree = Arc::new(Mutex::new(DomTree::new()));
    let mut context = engine.create_context(capabilities)?;
    bind_dom(&mut context, Arc::clone(&tree))?;

    let value = {
        let _quiet = PanicHookGuard::install();
        engine.eval_value(&mut context, source)?
    };

    drop(context);
    Ok((unwrap_tree(tree), value))
}

fn unwrap_tree(tree: Arc<Mutex<DomTree>>) -> DomTree {
    match Arc::try_unwrap(tree) {
        Ok(mutex) => mutex.into_inner().unwrap_or_else(PoisonError::into_inner),
        Err(shared) => shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone(),
    }
}

/// The last-resort document: `<html><body></body></html>`, built without a
/// script. This routine is not optional — a fault in the embedded fallback
/// script must not reopen the hole **C-09** closes.
#[must_use]
pub fn minimal_document() -> DomTree {
    let mut tree = DomTree::new();
    if let (Ok(html_tag), Ok(body_tag)) = (TagName::new("html"), TagName::new("body")) {
        let html = tree.create_element(html_tag);
        let body = tree.create_element(body_tag);
        let _ = tree.append_child(tree.document(), html);
        let _ = tree.append_child(html, body);
    }
    tree
}
