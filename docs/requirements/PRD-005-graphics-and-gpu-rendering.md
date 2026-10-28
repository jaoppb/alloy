# PRD-005: Graphics Pipeline & Multi-Tier GPU Rendering

- **Status**: Accepted
- **Author**: Graphics Architecture Team
- **Date**: 2026-08-22
- **Target Release**: v0.1.0-alpha

---

## 1. Executive Summary

Alloy features a decoupled, high-performance 2D graphics rendering architecture. The primary hardware acceleration
backend uses **Vulkan** (via `vulkano`), with automatic runtime fallback to **OpenGL** (via `glow`/`glutin`), and an
ultimate fallback to a **CPU Software Rasterizer** for headless environments, virtual machines, and CI test pipelines.

---

## 2. Problem Statement

Modern browser rendering requires hardware-accelerated rasterization with low latency and predictable frame pacing.
However:

1. Vulkan driver support varies across Linux distros, VMs, and older GPUs.
2. Direct coupling of DOM/CSS layout to a specific GPU API breaks testability and modularity.
3. Exposing raw GPU pointers to user scripts creates severe memory-safety and crash risks.

---

## 3. Multi-Tier Rendering Architecture

### 3.1 Three-Tier Fallback Cascade

```text
               [ Start Graphics Initialization ]
                               │
                               ▼
               [ Attempt Vulkan Init (vulkano) ]
                               │
                ┌──────────────┴──────────────┐
                │                             │
          [ Success ]                     [ Failed ]
                │                             │
                ▼                             ▼
       (Active: VulkanBackend)     [ Log Warning to DevTools ]
                                              │
                                              ▼
                                 [ Attempt OpenGL Init (glow) ]
                                              │
                                ┌─────────────┴─────────────┐
                                │                           │
                          [ Success ]                   [ Failed ]
                                │                           │
                                ▼                           ▼
                      (Active: OpenGLBackend)     [ Log Warning to DevTools ]
                                                            │
                                                            ▼
                                              (Active: SoftwareCpuBackend)
```

### 3.2 Display List / Command Buffer Model

Subsystems (Layout, CSS, UI, DevTools) never issue direct GPU draw calls. Instead, they produce declarative
`DisplayList` streams containing render commands:

- `DrawRect { rect: Rect, color: Color, border_radius: f32 }`
- `DrawText { glyphs: Vec<GlyphInstance>, color: Color, font_id: FontId }`
- `DrawImage { image_id: ImageId, src_rect: Rect, dst_rect: Rect }`
- `DrawPath { path: Path2D, fill: Option<Color>, stroke: Option<Stroke> }`
- `PushClip { clip_rect: Rect }` / `PopClip`
- `PushOpacity { opacity: f32 }` / `PopOpacity`

The active `RenderBackend` processes, optimizes, batches, and rasterizes the display list to the presentation surface.

---

## 4. Script Engine Graphics Boundary ("Muscle")

- **Capability Gate**: Scripts require the `GRAPHICS_DRAW` capability flag to interact with rendering.
- **Safe Builder**: Scripts interact exclusively via `DisplayListBuilder` and `RenderPass` pipeline hooks.
- **Fault Trapping**: Malformed draw commands (e.g. `NaN` coordinates, out-of-bounds colors) are sanitized at the
  builder boundary and do not trigger GPU driver crashes.

---

## 5. Acceptance Criteria

- [ ] `RenderBackend` trait defined in `core/graphics`.
- [ ] `VulkanBackend` (`vulkano`) initialized and capable of clearing/drawing display lists.
- [ ] Automatic fallback to `OpenGLBackend` (`glow`/`glutin`) when Vulkan instance creation fails.
- [ ] Automatic fallback to `SoftwareCpuBackend` when running headless without a GPU driver.
- [ ] Display list serialization and script binding tested with Rhai engine.

---

## 6. Port Migration Notes

The port is frozen at `F4` (ADR-0011 item 3); each bump of `graphics::PORT_SCHEMA_VERSION` is recorded here.

- **3 → 4 (#108, epic #110)**: `RenderBackend::bind_images(Arc<dyn ImageProvider>)` added, required. Backends are
  long-lived (one per compositor layer) while decoded images belong to the current document, so the image source must be
  swappable after construction. Call it between frames; it takes effect from the next `begin_frame` (a mid-frame bind is
  unspecified). A backend that draws no images accepts and ignores it. Every implementer adds the method; fonts stay
  constructor-bound.
