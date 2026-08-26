use crate::domain::command::RenderCommand;
use crate::domain::display_list::DisplayList;
use crate::domain::geometry::Rect;
use css::Color;
use engine::{
    Capability, EngineError, EngineValue, ExecutionContext, Identifier, guarded_native_fn,
};
use std::sync::{Arc, Mutex};

/// Shared state container for display lists operated by user scripts.
#[derive(Clone, Default)]
pub struct ScriptDisplayListContainer {
    list: Arc<Mutex<DisplayList>>,
}

impl ScriptDisplayListContainer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            list: Arc::new(Mutex::new(DisplayList::new())),
        }
    }

    #[must_use]
    pub fn get_display_list(&self) -> DisplayList {
        self.list.lock().unwrap().clone()
    }
}

/// Registers graphics DisplayList bindings into an `ExecutionContext` (PRD-005:76-91, C-18).
///
/// Every registered function is guarded by `Capability::GRAPHICS_DRAW`.
///
/// # Errors
/// Returns `EngineError` if function registration fails.
pub fn register_graphics_bindings(
    ctx: &mut dyn ExecutionContext,
    container: ScriptDisplayListContainer,
) -> Result<(), EngineError> {
    // 1. graphics_push_rect(x, y, w, h, hex_or_name)
    let c1 = container.clone();
    ctx.register_fn(
        Identifier::new("graphics_push_rect")?,
        guarded_native_fn(Capability::GRAPHICS_DRAW, move |_ctx, args| {
            let x = args.first().and_then(|v| v.as_f64().ok()).unwrap_or(0.0) as f32;
            let y = args.get(1).and_then(|v| v.as_f64().ok()).unwrap_or(0.0) as f32;
            let w = args.get(2).and_then(|v| v.as_f64().ok()).unwrap_or(100.0) as f32;
            let h = args.get(3).and_then(|v| v.as_f64().ok()).unwrap_or(100.0) as f32;
            let color_str = args.get(4).and_then(|v| v.as_str().ok()).unwrap_or("black");

            let color = Color::parse(color_str).unwrap_or(Color::BLACK);
            let mut list = c1.list.lock().unwrap();
            list.push(RenderCommand::DrawRect {
                rect: Rect::new(x, y, w, h),
                color,
            });

            Ok(EngineValue::Int(list.len() as i64))
        }),
    )?;

    // 2. graphics_get_len()
    let c2 = container.clone();
    ctx.register_fn(
        Identifier::new("graphics_get_len")?,
        guarded_native_fn(Capability::GRAPHICS_DRAW, move |_ctx, _args| {
            let list = c2.list.lock().unwrap();
            Ok(EngineValue::Int(list.len() as i64))
        }),
    )?;

    // 3. graphics_serialize_json()
    let c3 = container;
    ctx.register_fn(
        Identifier::new("graphics_serialize_json")?,
        guarded_native_fn(Capability::GRAPHICS_DRAW, move |_ctx, _args| {
            let list = c3.list.lock().unwrap();
            let json = list.serialize_to_json();
            Ok(EngineValue::String(json))
        }),
    )?;

    Ok(())
}
