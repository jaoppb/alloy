use crate::domain::command::RenderCommand;
use crate::domain::display_list::DisplayList;
use crate::domain::geometry::Rect;
use css::Color;
use engine::{
    Capability, EngineError, EngineValue, ExecutionContext, HostModule, HostObject, Identifier,
    guarded_native_fn,
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
        self.list
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// Host module registering 2D graphics DisplayList bindings (ADR-0012, N-01, C-53, C-55, C-56).
pub struct GraphicsHostModule {
    container: ScriptDisplayListContainer,
}

impl GraphicsHostModule {
    /// Creates a new `GraphicsHostModule` wrapping a `ScriptDisplayListContainer`.
    #[must_use]
    pub const fn new(container: ScriptDisplayListContainer) -> Self {
        Self { container }
    }

    fn create_renderer_host_object(&self) -> Result<HostObject, EngineError> {
        let mut renderer = HostObject::new(Identifier::new("renderer")?)
            .with_singleton(true)
            .with_capability(Capability::GRAPHICS_DRAW);

        // pushRect(x, y, w, h, color) -> count
        let c1 = self.container.clone();
        renderer.add_method(Identifier::new("pushRect")?, move |_this, args| {
            let x = args.first().and_then(|v| v.as_f64().ok()).unwrap_or(0.0) as f32;
            let y = args.get(1).and_then(|v| v.as_f64().ok()).unwrap_or(0.0) as f32;
            let w = args.get(2).and_then(|v| v.as_f64().ok()).unwrap_or(100.0) as f32;
            let h = args.get(3).and_then(|v| v.as_f64().ok()).unwrap_or(100.0) as f32;
            let color_str = args.get(4).and_then(|v| v.as_str().ok()).unwrap_or("black");

            let color = Color::parse(color_str).unwrap_or(Color::BLACK);
            let mut list = c1.list.lock().unwrap_or_else(|p| p.into_inner());
            list.push(RenderCommand::DrawRect {
                rect: Rect::new(x, y, w, h),
                color,
            });

            Ok(EngineValue::Int(list.len() as i64))
        });

        // pushBorder(x, y, w, h, border_width, color) -> count (C-55)
        let cb = self.container.clone();
        renderer.add_method(Identifier::new("pushBorder")?, move |_this, args| {
            let x = args.first().and_then(|v| v.as_f64().ok()).unwrap_or(0.0) as f32;
            let y = args.get(1).and_then(|v| v.as_f64().ok()).unwrap_or(0.0) as f32;
            let w = args.get(2).and_then(|v| v.as_f64().ok()).unwrap_or(100.0) as f32;
            let h = args.get(3).and_then(|v| v.as_f64().ok()).unwrap_or(100.0) as f32;
            let width = args.get(4).and_then(|v| v.as_f64().ok()).unwrap_or(1.0) as f32;
            let color_str = args.get(5).and_then(|v| v.as_str().ok()).unwrap_or("black");

            let color = Color::parse(color_str).unwrap_or(Color::BLACK);
            let mut list = cb.list.lock().unwrap_or_else(|p| p.into_inner());
            list.push(RenderCommand::DrawBorder {
                rect: Rect::new(x, y, w, h),
                color,
                width,
            });

            Ok(EngineValue::Int(list.len() as i64))
        });

        // pushText(text, x, y, w, h, font_size, color) -> count (C-55)
        let ct = self.container.clone();
        renderer.add_method(Identifier::new("pushText")?, move |_this, args| {
            let text = args.first().and_then(|v| v.as_str().ok()).unwrap_or("");
            let x = args.get(1).and_then(|v| v.as_f64().ok()).unwrap_or(0.0) as f32;
            let y = args.get(2).and_then(|v| v.as_f64().ok()).unwrap_or(0.0) as f32;
            let w = args.get(3).and_then(|v| v.as_f64().ok()).unwrap_or(100.0) as f32;
            let h = args.get(4).and_then(|v| v.as_f64().ok()).unwrap_or(20.0) as f32;
            let font_size = args.get(5).and_then(|v| v.as_f64().ok()).unwrap_or(16.0) as f32;
            let color_str = args.get(6).and_then(|v| v.as_str().ok()).unwrap_or("black");

            let color = Color::parse(color_str).unwrap_or(Color::BLACK);
            let mut list = ct.list.lock().unwrap_or_else(|p| p.into_inner());
            list.push(RenderCommand::DrawText {
                text: text.to_string(),
                rect: Rect::new(x, y, w, h),
                color,
                font_size,
            });

            Ok(EngineValue::Int(list.len() as i64))
        });

        // commandCount() -> count
        let c2 = self.container.clone();
        renderer.add_method(Identifier::new("commandCount")?, move |_this, _args| {
            let list = c2.list.lock().unwrap_or_else(|p| p.into_inner());
            Ok(EngineValue::Int(list.len() as i64))
        });

        // toJSON() -> String
        let c3 = self.container.clone();
        renderer.add_method(Identifier::new("toJSON")?, move |_this, _args| {
            let list = c3.list.lock().unwrap_or_else(|p| p.into_inner());
            let json = list.serialize_to_json();
            Ok(EngineValue::String(json))
        });

        // clear() or clear(color) -> Null (C-55)
        let c4 = self.container.clone();
        renderer.add_method(Identifier::new("clear")?, move |_this, args| {
            let mut list = c4.list.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(color_val) = args.first().and_then(|v| v.as_str().ok()) {
                if let Some(c) = Color::parse(color_val) {
                    list.push(RenderCommand::Clear(c));
                    return Ok(EngineValue::Null);
                }
            }
            list.clear();
            Ok(EngineValue::Null)
        });

        Ok(renderer)
    }

    fn register_legacy_functions(&self, ctx: &mut dyn ExecutionContext) -> Result<(), EngineError> {
        let c1 = self.container.clone();
        ctx.register_fn(
            Identifier::new("graphics_push_rect")?,
            guarded_native_fn(Capability::GRAPHICS_DRAW, move |_ctx, args| {
                let x = args.first().and_then(|v| v.as_f64().ok()).unwrap_or(0.0) as f32;
                let y = args.get(1).and_then(|v| v.as_f64().ok()).unwrap_or(0.0) as f32;
                let w = args.get(2).and_then(|v| v.as_f64().ok()).unwrap_or(100.0) as f32;
                let h = args.get(3).and_then(|v| v.as_f64().ok()).unwrap_or(100.0) as f32;
                let color_str = args.get(4).and_then(|v| v.as_str().ok()).unwrap_or("black");

                let color = Color::parse(color_str).unwrap_or(Color::BLACK);
                let mut list = c1.list.lock().unwrap_or_else(|p| p.into_inner());
                list.push(RenderCommand::DrawRect {
                    rect: Rect::new(x, y, w, h),
                    color,
                });

                Ok(EngineValue::Int(list.len() as i64))
            }),
        )?;

        let c2 = self.container.clone();
        ctx.register_fn(
            Identifier::new("graphics_get_len")?,
            guarded_native_fn(Capability::GRAPHICS_DRAW, move |_ctx, _args| {
                let list = c2.list.lock().unwrap_or_else(|p| p.into_inner());
                Ok(EngineValue::Int(list.len() as i64))
            }),
        )?;

        let c3 = self.container.clone();
        ctx.register_fn(
            Identifier::new("graphics_serialize_json")?,
            guarded_native_fn(Capability::GRAPHICS_DRAW, move |_ctx, _args| {
                let list = c3.list.lock().unwrap_or_else(|p| p.into_inner());
                let json = list.serialize_to_json();
                Ok(EngineValue::String(json))
            }),
        )?;

        Ok(())
    }
}

impl HostModule for GraphicsHostModule {
    fn name(&self) -> &'static str {
        "graphics"
    }

    fn register(&self, ctx: &mut dyn ExecutionContext) -> Result<(), EngineError> {
        ctx.register_host_object(self.create_renderer_host_object()?)?;
        self.register_legacy_functions(ctx)?;
        Ok(())
    }
}

/// Registers graphics DisplayList bindings into an `ExecutionContext` under `renderer` host object (ADR-0012, N-01, C-53).
pub fn register_graphics_bindings(
    ctx: &mut dyn ExecutionContext,
    container: ScriptDisplayListContainer,
) -> Result<(), EngineError> {
    GraphicsHostModule::new(container).register(ctx)
}
