use css::{Color, StyleCascade, parse_css};
use engine::{
    Capability, CapabilitySet, EngineError, EngineValue, ExecutionContext, RuntimeEngine,
};
use graphics::{
    DisplayList, GraphicsBackendFactory, LayoutEngine, Point, Position, Rect, RenderBackend,
    RenderCommand, ScriptDisplayListContainer, Size, SoftwareCpuBackend,
    register_graphics_bindings,
};
use html::parse_html;
use rhai_runtime::RhaiEngine;

#[test]
fn test_c14_render_backend_trait_implemented() {
    let mut backend = SoftwareCpuBackend::new(100, 100);

    assert_eq!(backend.name(), "SoftwareCpuBackend");
    assert_eq!(backend.dimensions(), (100, 100));

    let mut list = DisplayList::new();
    list.push(RenderCommand::Clear(Color::WHITE));
    list.push(RenderCommand::DrawRect {
        rect: Rect::new(10.0, 10.0, 80.0, 80.0),
        color: Color::RED,
    });

    backend.render(&list).expect("Render should succeed");
    let bytes = backend.to_rgba_bytes().expect("Bytes should be present");
    assert_eq!(bytes.len(), 100 * 100 * 4);
}

#[test]
fn test_c17_headless_fallback_to_software_cpu_backend() {
    let mut backend = GraphicsBackendFactory::create_headless(200, 150);
    assert_eq!(
        backend.name(),
        "SoftwareCpuBackend",
        "Headless factory must fallback to SoftwareCpuBackend"
    );

    let mut list = DisplayList::new();
    list.push(RenderCommand::Clear(Color::BLACK));
    backend.render(&list).expect("Clear surface");

    let bytes = backend.to_rgba_bytes().expect("Bytes");
    // Check first pixel is black (0, 0, 0, 255)
    assert_eq!(&bytes[0..4], &[0, 0, 0, 255]);
}

#[test]
fn test_c18_display_list_serialization_and_script_binding() {
    // 1. Serialization test
    let mut list = DisplayList::new();
    list.push(RenderCommand::Clear(Color::WHITE));
    list.push(RenderCommand::DrawRect {
        rect: Rect::new(0.0, 0.0, 50.0, 50.0),
        color: Color::RED,
    });
    let json = list.serialize_to_json();
    assert!(json.contains(r#""type":"clear""#));
    assert!(json.contains(r#""type":"draw_rect""#));

    // 2. Script binding with GRAPHICS_DRAW capability
    let engine = RhaiEngine::new();
    let container = ScriptDisplayListContainer::new();

    let mut caps = CapabilitySet::empty();
    caps.grant(Capability::GRAPHICS_DRAW);
    let mut ctx = engine.create_context(caps).unwrap();

    register_graphics_bindings(&mut ctx, container.clone()).unwrap();

    let push_id = engine::Identifier::new("graphics_push_rect").unwrap();
    let serialize_id = engine::Identifier::new("graphics_serialize_json").unwrap();

    ctx.call_function(
        &push_id,
        &[
            EngineValue::Float(10.0),
            EngineValue::Float(15.0),
            EngineValue::Float(120.0),
            EngineValue::Float(80.0),
            EngineValue::String("blue".to_string()),
        ],
    )
    .expect("graphics_push_rect should succeed");

    let result = ctx
        .call_function(&serialize_id, &[])
        .expect("graphics_serialize_json should succeed");
    let json_from_script = result.as_str().expect("String JSON");
    assert!(json_from_script.contains("draw_rect"));

    let inner_list = container.get_display_list();
    assert_eq!(inner_list.len(), 1);

    // 3. Script binding without GRAPHICS_DRAW capability returns PermissionDenied
    let mut denied_ctx = engine.create_context(CapabilitySet::empty()).unwrap();
    register_graphics_bindings(&mut denied_ctx, container).unwrap();

    let denied_res = denied_ctx.call_function(&push_id, &[]);

    match denied_res {
        Err(EngineError::PermissionDenied(cap)) => {
            assert!(cap.contains("GRAPHICS_DRAW"));
        }
        other => panic!("Expected EngineError::PermissionDenied, got {other:?}"),
    }
}

#[test]
fn test_i2_end_to_end_headless_pipeline_to_png() {
    let html = r#"
        <html>
          <head>
            <style>
              body { background-color: #ffffff; color: black; }
              h1 { color: red; font-size: 20px; }
              .box { background-color: blue; height: 30px; }
            </style>
          </head>
          <body>
            <h1>Heading</h1>
            <div class="box"></div>
          </body>
        </html>
    "#;

    let dom = parse_html(html).expect("Parse HTML");
    let css = r#"
        body { background-color: #ffffff; color: black; }
        h1 { color: red; font-size: 20px; }
        .box { background-color: blue; height: 30px; }
    "#;
    let stylesheet = parse_css(css).expect("Parse CSS");
    let styled_tree = StyleCascade::build_styled_tree(&dom, &stylesheet);

    let display_list = LayoutEngine::layout(&dom, &styled_tree, 400.0, 300.0);
    assert!(display_list.len() >= 2);

    let mut backend = GraphicsBackendFactory::create_headless(400, 300);
    backend.render(&display_list).expect("Render display list");

    let out_dir = std::env::temp_dir();
    let out_path = out_dir.join("alloy_test_headless_render.png");

    backend.save_png(&out_path).expect("Save PNG");

    assert!(out_path.exists(), "PNG file must be created on disk");
    let file_bytes = std::fs::read(&out_path).expect("Read generated PNG");
    assert!(file_bytes.len() > 100, "PNG should be non-trivial in size");
    // Verify PNG magic bytes: 0x89 'P' 'N' 'G' 0x0D 0x0A 0x1A 0x0A
    assert_eq!(&file_bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);

    // Clean up
    let _ = std::fs::remove_file(out_path);
}

#[test]
fn test_out_of_bounds_clipping_safety() {
    let mut backend = SoftwareCpuBackend::new(100, 100);

    let mut list = DisplayList::new();
    // Negative coordinates
    list.push(RenderCommand::DrawRect {
        rect: Rect::new(-50.0, -50.0, 80.0, 80.0),
        color: Color::RED,
    });
    // Partially outside right/bottom
    list.push(RenderCommand::DrawRect {
        rect: Rect::new(80.0, 80.0, 50.0, 50.0),
        color: Color::BLUE,
    });
    // Completely outside
    list.push(RenderCommand::DrawRect {
        rect: Rect::new(500.0, 500.0, 100.0, 100.0),
        color: Color::GREEN,
    });

    // Rendering must succeed without panicking
    backend
        .render(&list)
        .expect("Out of bounds commands must be clipped safely without panic");

    let pixels = backend.to_rgba_bytes().unwrap();
    assert_eq!(pixels.len(), 100 * 100 * 4);
}

#[test]
fn test_size_invariants() {
    assert!(Size::new(-1.0, 0.0).is_err());
    assert!(Size::new(0.0, -1.0).is_err());
    assert!(Size::new(f32::NAN, 0.0).is_err());
    assert!(Size::new(0.0, f32::NAN).is_err());
    assert!(Size::new(f32::INFINITY, 0.0).is_err());

    let valid = Size::new(100.0, 200.0).expect("Valid size");
    assert_eq!(valid.width(), 100.0);
    assert_eq!(valid.height(), 200.0);
}

#[test]
fn test_geometry_value_objects() {
    let pt = Point::new(10.0, 20.0);
    assert_eq!(pt.x(), 10.0);
    assert_eq!(pt.y(), 20.0);

    let sz = Size::new(100.0, 50.0).unwrap();
    let rect = Rect::from_origin_size(pt, sz);
    assert_eq!(rect.origin(), pt);
    assert_eq!(rect.size(), sz);
    assert_eq!(rect.x(), 10.0);
    assert_eq!(rect.y(), 20.0);
    assert_eq!(rect.width(), 100.0);
    assert_eq!(rect.height(), 50.0);
    assert_eq!(rect.right(), 110.0);
    assert_eq!(rect.bottom(), 70.0);

    let pos = Position::new(5, 15);
    assert_eq!(pos.x(), 5);
    assert_eq!(pos.y(), 15);
}
