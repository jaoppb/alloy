use crate::domain::attribute::AttributeMap;
use crate::domain::node_data::NodeData;
use crate::domain::node_id::NodeId;
use crate::domain::tag_name::TagName;
use crate::domain::tree::DomTree;
use engine::{Capability, EngineError, EngineValue, ExecutionContext, HostObject, Identifier};
use std::sync::{Arc, Mutex};

fn extract_node_id(val: Option<&EngineValue>) -> Result<NodeId, EngineError> {
    val.and_then(|v| {
        v.downcast_handle::<NodeId>()
            .copied()
            .or_else(|| v.as_i64().ok().map(|i| NodeId::new(i as u32)))
    })
    .ok_or(EngineError::TypeMismatch {
        expected: "Node handle",
        found: "unknown",
    })
}

/// Registers DOM domain operations into an `ExecutionContext` under `document` and `Node` host objects (ADR-0012, N-01).
///
/// # Errors
/// Returns `EngineError` if host object registration fails.
pub fn register_dom_bindings(
    ctx: &mut dyn ExecutionContext,
    tree: Arc<Mutex<DomTree>>,
) -> Result<(), EngineError> {
    // 1. HostObject: document (singleton)
    {
        let mut document = HostObject::new(Identifier::new("document")?)
            .with_singleton(true)
            .with_capability(Capability::DOM_MUTATE);

        // createElement(tag_name: String) -> Handle<NodeId>
        let t1 = Arc::clone(&tree);
        document.add_method(Identifier::new("createElement")?, move |_this, args| {
            let tag_str =
                args.first()
                    .and_then(|v| v.as_str().ok())
                    .ok_or(EngineError::TypeMismatch {
                        expected: "string",
                        found: "unknown",
                    })?;

            let tag =
                TagName::new(tag_str).map_err(|e| EngineError::RuntimeError(e.to_string()))?;
            let mut guard = t1
                .lock()
                .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
            let id = guard.create_element(tag, AttributeMap::new());
            Ok(EngineValue::handle(id))
        });

        // createTextNode(text: String) -> Handle<NodeId>
        let t2 = Arc::clone(&tree);
        document.add_method(Identifier::new("createTextNode")?, move |_this, args| {
            let text =
                args.first()
                    .and_then(|v| v.as_str().ok())
                    .ok_or(EngineError::TypeMismatch {
                        expected: "string",
                        found: "unknown",
                    })?;

            let mut guard = t2
                .lock()
                .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
            let id = guard.create_text(text);
            Ok(EngineValue::handle(id))
        });

        ctx.register_host_object(document)?;
    }

    // 2. HostObject: Node (instance entity)
    {
        let mut node = HostObject::new(Identifier::new("Node")?)
            .with_singleton(false)
            .with_capability(Capability::DOM_MUTATE);

        // appendChild(child: Handle<NodeId>) -> Null
        let t3 = Arc::clone(&tree);
        node.add_method(Identifier::new("appendChild")?, move |this, args| {
            let parent = extract_node_id(this)?;
            let child = extract_node_id(args.first())?;

            let mut guard = t3
                .lock()
                .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
            guard
                .append_child(parent, child)
                .map_err(|e| EngineError::RuntimeError(e.to_string()))?;
            Ok(EngineValue::Null)
        });

        // getText() -> String
        let t4 = Arc::clone(&tree);
        node.add_method(Identifier::new("getText")?, move |this, _args| {
            let id = extract_node_id(this)?;
            let guard = t4
                .lock()
                .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
            let node = guard
                .get(id)
                .ok_or_else(|| EngineError::RuntimeError(format!("Node not found: {id}")))?;
            let text = node.data().as_text().unwrap_or("");
            Ok(EngineValue::String(text.to_string()))
        });

        // setText(text: String) -> Null
        let t5 = Arc::clone(&tree);
        node.add_method(Identifier::new("setText")?, move |this, args| {
            let id = extract_node_id(this)?;
            let new_text =
                args.first()
                    .and_then(|v| v.as_str().ok())
                    .ok_or(EngineError::TypeMismatch {
                        expected: "string",
                        found: "unknown",
                    })?;

            let mut guard = t5
                .lock()
                .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
            let node = guard
                .get_mut(id)
                .ok_or_else(|| EngineError::RuntimeError(format!("Node not found: {id}")))?;

            if let Some(text_mut) = node.data_mut().as_text_mut() {
                *text_mut = new_text.to_string();
                return Ok(EngineValue::Null);
            }

            *node.data_mut() = NodeData::Text(new_text.to_string());
            Ok(EngineValue::Null)
        });

        // Property textContent getter & setter
        let tg = Arc::clone(&tree);
        let ts = Arc::clone(&tree);
        node.add_property(
            Identifier::new("textContent")?,
            move |this| {
                let id = extract_node_id(this)?;
                let guard = tg
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                let node = guard
                    .get(id)
                    .ok_or_else(|| EngineError::RuntimeError(format!("Node not found: {id}")))?;
                let text = node.data().as_text().unwrap_or("");
                Ok(EngineValue::String(text.to_string()))
            },
            Some(Arc::new(move |this, val| {
                let id = extract_node_id(this)?;
                let new_text = val.as_str().unwrap_or("");
                let mut guard = ts
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                let node = guard
                    .get_mut(id)
                    .ok_or_else(|| EngineError::RuntimeError(format!("Node not found: {id}")))?;
                if let Some(text_mut) = node.data_mut().as_text_mut() {
                    *text_mut = new_text.to_string();
                } else {
                    *node.data_mut() = NodeData::Text(new_text.to_string());
                }
                Ok(())
            })),
        );

        ctx.register_host_object(node)?;
    }

    // Backwards compatibility registrations for flat function calls
    {
        let t = Arc::clone(&tree);
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
                let mut guard = t
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                let id = guard.create_element(tag, AttributeMap::new());
                Ok(EngineValue::handle(id))
            }),
        )?;

        let t = Arc::clone(&tree);
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
                let mut guard = t
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                let id = guard.create_text(text);
                Ok(EngineValue::handle(id))
            }),
        )?;

        let t = Arc::clone(&tree);
        ctx.register_fn(
            Identifier::new("dom_append_child")?,
            Arc::new(move |ctx, args| {
                if !ctx.capabilities().contains(Capability::DOM_MUTATE) {
                    return Err(EngineError::PermissionDenied(format!(
                        "{:?}",
                        Capability::DOM_MUTATE
                    )));
                }
                let parent = extract_node_id(args.first())?;
                let child = extract_node_id(args.get(1))?;
                let mut guard = t
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                guard
                    .append_child(parent, child)
                    .map_err(|e| EngineError::RuntimeError(e.to_string()))?;
                Ok(EngineValue::Null)
            }),
        )?;

        let t = Arc::clone(&tree);
        ctx.register_fn(
            Identifier::new("dom_get_text")?,
            Arc::new(move |ctx, args| {
                if !ctx.capabilities().contains(Capability::DOM_READ) {
                    return Err(EngineError::PermissionDenied(format!(
                        "{:?}",
                        Capability::DOM_READ
                    )));
                }
                let id = extract_node_id(args.first())?;
                let guard = t
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                let node = guard
                    .get(id)
                    .ok_or_else(|| EngineError::RuntimeError(format!("Node not found: {id}")))?;
                let text = node.data().as_text().unwrap_or("");
                Ok(EngineValue::String(text.to_string()))
            }),
        )?;

        let t = Arc::clone(&tree);
        ctx.register_fn(
            Identifier::new("dom_set_text")?,
            Arc::new(move |ctx, args| {
                if !ctx.capabilities().contains(Capability::DOM_MUTATE) {
                    return Err(EngineError::PermissionDenied(format!(
                        "{:?}",
                        Capability::DOM_MUTATE
                    )));
                }
                let id = extract_node_id(args.first())?;
                let new_text =
                    args.get(1)
                        .and_then(|v| v.as_str().ok())
                        .ok_or(EngineError::TypeMismatch {
                            expected: "string",
                            found: "unknown",
                        })?;
                let mut guard = t
                    .lock()
                    .map_err(|_| EngineError::RuntimeError("Lock poisoned".to_string()))?;
                let node = guard
                    .get_mut(id)
                    .ok_or_else(|| EngineError::RuntimeError(format!("Node not found: {id}")))?;
                if let Some(text_mut) = node.data_mut().as_text_mut() {
                    *text_mut = new_text.to_string();
                } else {
                    *node.data_mut() = NodeData::Text(new_text.to_string());
                }
                Ok(EngineValue::Null)
            }),
        )?;
    }

    Ok(())
}
