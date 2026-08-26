# SPDD Analysis: Graphics Pipeline, DisplayList & SoftwareCpuBackend (core/graphics & I2)

## Original Business Requirement

### ROADMAP-IMPLEMENTACAO-V1: Fase F7 (Trilha C), I2 & PRD-005 (§Graphics & Rendering)

Complete the headless visual rendering pipeline and deliver **Release v0.3 ("Headless: HTML -> DOM -> DisplayList ->
PNG")**:

- Define the abstract `RenderBackend` trait in `core/graphics` (**C-14**).
- Implement declarative 2D `DisplayList` and `RenderCommand` stream (**C-18**).
- Implement `SoftwareCpuBackend` with pixel buffer rasterization and PNG export (**C-17**).
- Provide multi-tier fallback factory choosing `SoftwareCpuBackend` in headless mode (**C-17**).
- Provide Rhai bindings for display list serialization and manipulation guarded by `Capability::GRAPHICS_DRAW`
  (**C-18**).
- Implement the headless pipeline in `alloy` CLI: `alloy render page.html -o out.png` (**I2**).

---

## Domain Concept Identification

### Existing Concepts (from codebase)

- `DomTree`, `DomNode`, `NodeId` (`core/dom`).
- `HtmlParser`, `parse_html` (`core/html`).
- `StyledTree`, `StyledNode`, `ComputedStyle`, `Color`, `Px`, `DisplayType` (`core/css`).
- `Capability::GRAPHICS_DRAW`, `ExecutionContext`, `EngineValue` (`core/engine`).

### New Concepts Required

- `Rect`, `Point`, `Size`: 2D geometry primitives.
- `RenderCommand`: Declarative draw instructions (`Clear`, `DrawRect`, `DrawText`, `DrawBorder`).
- `DisplayList`: First-class collection of `RenderCommand` items (ADR-0010).
- `RenderBackend`: Abstract trait for graphics rasterization engines.
- `SoftwareCpuBackend`: CPU-based rasterizer rendering display lists to an RGBA pixel framebuffer.
- `GraphicsError`: Domain error enum for initialization, rendering, or encoding failures.
- `LayoutEngine`: Service translating `StyledTree` into a positioned `DisplayList`.
- `GraphicsBridge`: Integration bridge exposing display list operations to script execution contexts.

### Key Business Rules

- **Decoupled Graphics**: Subsystems (HTML, CSS, Layout) never make direct raster calls. They only produce declarative
  `DisplayList`s.
- **Headless Guarantee**: In headless environments (CLI, CI, servers), the engine automatically uses
  `SoftwareCpuBackend`.
- **Capability Gate**: Script access to display lists requires `Capability::GRAPHICS_DRAW`.
- **Command Line Interface (I2)**: `alloy render input.html -o output.png` executes the complete pipeline:
  `HTML -> DomTree -> StyledTree -> DisplayList -> PNG`.

---

## Strategic Approach

### Solution Direction

- In `Cargo.toml`:
    - Add `image = { version = "0.25", default-features = false, features = ["png"] }` under `[workspace.dependencies]`.
- In `core/graphics`:
    - Domain: `geometry.rs`, `command.rs`, `display_list.rs`, `backend.rs`, `error.rs`.
    - Application: `cpu_backend.rs`, `layout.rs`, `factory.rs`.
    - Infrastructure: `rhai_bridge.rs`.
    - Conformance and integration tests: `tests/graphics_conformance.rs`.
- In `alloy`:
    - Add dependencies on `graphics`, `html`, `css`, `dom`.
    - Add `render` subcommand in `src/main.rs`.

---

## Acceptance Criteria Coverage

| AC#      | Descrição                                                 | Endereçável nesta Fase (F7 & I2)? | Notas                                           |
| :------- | :-------------------------------------------------------- | :-------------------------------- | :---------------------------------------------- |
| **C-14** | Trait `RenderBackend` definida em `core/graphics`         | Sim                               | Trait pura `RenderBackend`.                     |
| **C-17** | Fallback automático para `SoftwareCpuBackend` em headless | Sim                               | `GraphicsBackendFactory::create_headless(...)`. |
| **C-18** | Serialização de display list e binding com Rhai testados  | Sim                               | `GraphicsBridge` e testes automatizados.        |
| **I2**   | Pipeline headless `alloy render page.html -o out.png`     | Sim                               | Subcomando CLI em `alloy/src/main.rs`.          |
