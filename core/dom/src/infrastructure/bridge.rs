use crate::domain::attribute::AttributeMap;
use crate::domain::node_data::NodeData;
use crate::domain::node_id::NodeId;
use crate::domain::tag_name::TagName;
use crate::domain::tree::DomTree;
use engine::{Capability, EngineError, EngineValue, ExecutionContext, Identifier};
use std::sync::{Arc, Mutex};

/// Registers DOM domain operations into an `ExecutionContext`, enforcing capability sandboxing (C-03).
///
/// # Errors
/// Returns `EngineError` if any function registration fails.
pub fn register_dom_bindings(
    ctx: &mut dyn ExecutionContext,
    tree: Arc<Mutex<DomTree>>,
) -> Result<(), EngineError> {
    // 1. dom_create_element(tag_name: String) -> NodeId
    {
        let tree = Arc::clone(&tree);
        ctx.register_fn(
            Identifier::new("dom_create_element")?,
            Arc::new(move |ctx, args| {
                if !ctx.capabilities().contains(Capability::DOM_MUTATE) {
                    return Err(EngineError::PermissionDenied(format!(
                        "{:?}",
                        Capability::DOM_MUTATE
                    )));
                }

                let tag_str = args.first().and_then(|v| v.as_str().ok()).ok_or(
                    EngineError::TypeMismatch {
                        expected: "string",
                        found: "unknown",
                    },
                )?;

                let tag =
                    TagName::new(tag_str).map_err(|e| EngineError::RuntimeError(e.to_string()))?;
                let mut guard = tree
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                let id = guard.create_element(tag, AttributeMap::new());
                Ok(EngineValue::Int(i64::from(id.index())))
            }),
        )?;
    }

    // 2. dom_create_text(text: String) -> NodeId
    {
        let tree = Arc::clone(&tree);
        ctx.register_fn(
            Identifier::new("dom_create_text")?,
            Arc::new(move |ctx, args| {
                if !ctx.capabilities().contains(Capability::DOM_MUTATE) {
                    return Err(EngineError::PermissionDenied(format!(
                        "{:?}",
                        Capability::DOM_MUTATE
                    )));
                }

                let text = args.first().and_then(|v| v.as_str().ok()).ok_or(
                    EngineError::TypeMismatch {
                        expected: "string",
                        found: "unknown",
                    },
                )?;

                let mut guard = tree
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                let id = guard.create_text(text);
                Ok(EngineValue::Int(i64::from(id.index())))
            }),
        )?;
    }

    // 3. dom_append_child(parent_id: Int, child_id: Int) -> Null
    {
        let tree = Arc::clone(&tree);
        ctx.register_fn(
            Identifier::new("dom_append_child")?,
            Arc::new(move |ctx, args| {
                if !ctx.capabilities().contains(Capability::DOM_MUTATE) {
                    return Err(EngineError::PermissionDenied(format!(
                        "{:?}",
                        Capability::DOM_MUTATE
                    )));
                }

                let parent_idx = args.first().and_then(|v| v.as_i64().ok()).ok_or(
                    EngineError::TypeMismatch {
                        expected: "int",
                        found: "unknown",
                    },
                )?;

                let child_idx =
                    args.get(1)
                        .and_then(|v| v.as_i64().ok())
                        .ok_or(EngineError::TypeMismatch {
                            expected: "int",
                            found: "unknown",
                        })?;

                let parent = NodeId::new(parent_idx as u32);
                let child = NodeId::new(child_idx as u32);

                let mut guard = tree
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                guard
                    .append_child(parent, child)
                    .map_err(|e| EngineError::RuntimeError(e.to_string()))?;

                Ok(EngineValue::Null)
            }),
        )?;
    }

    // 4. dom_get_text(node_id: Int) -> String
    {
        let tree = Arc::clone(&tree);
        ctx.register_fn(
            Identifier::new("dom_get_text")?,
            Arc::new(move |ctx, args| {
                if !ctx.capabilities().contains(Capability::DOM_READ) {
                    return Err(EngineError::PermissionDenied(format!(
                        "{:?}",
                        Capability::DOM_READ
                    )));
                }

                let node_idx = args.first().and_then(|v| v.as_i64().ok()).ok_or(
                    EngineError::TypeMismatch {
                        expected: "int",
                        found: "unknown",
                    },
                )?;

                let id = NodeId::new(node_idx as u32);
                let guard = tree
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                let node = guard
                    .get(id)
                    .ok_or_else(|| EngineError::RuntimeError(format!("Node not found: {id}")))?;

                let text = node.data().as_text().unwrap_or("");
                Ok(EngineValue::String(text.to_string()))
            }),
        )?;
    }

    // 5. dom_set_text(node_id: Int, new_text: String) -> Null
    {
        let tree = Arc::clone(&tree);
        ctx.register_fn(
            Identifier::new("dom_set_text")?,
            Arc::new(move |ctx, args| {
                if !ctx.capabilities().contains(Capability::DOM_MUTATE) {
                    return Err(EngineError::PermissionDenied(format!(
                        "{:?}",
                        Capability::DOM_MUTATE
                    )));
                }

                let node_idx = args.first().and_then(|v| v.as_i64().ok()).ok_or(
                    EngineError::TypeMismatch {
                        expected: "int",
                        found: "unknown",
                    },
                )?;

                let new_text =
                    args.get(1)
                        .and_then(|v| v.as_str().ok())
                        .ok_or(EngineError::TypeMismatch {
                            expected: "string",
                            found: "unknown",
                        })?;

                let id = NodeId::new(node_idx as u32);
                let mut guard = tree
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                let node = guard
                    .get_mut(id)
                    .ok_or_else(|| EngineError::RuntimeError(format!("Node not found: {id}")))?;

                if let Some(text_mut) = node.data_mut().as_text_mut() {
                    *text_mut = new_text.to_string();
                    return Ok(EngineValue::Null);
                }

                // If not already a text node, replace with text node data
                *node.data_mut() = NodeData::Text(new_text.to_string());
                Ok(EngineValue::Null)
            }),
        )?;
    }

    Ok(())
}
