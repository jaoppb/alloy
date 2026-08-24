# ADR-0009: Vulkan Rendering with OpenGL and Software Fallback

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-22

---

## Context and Problem Statement

Alloy requires a modern, hardware-accelerated 2D rendering pipeline capable of high frame rates and low latency.
However, graphics hardware and driver capabilities vary across operating environments (e.g. discrete GPUs, integrated
GPUs, Wayland/X11 compositors, VMs, and headless CI runners). How should we architect the graphics subsystem to maximize
performance while guaranteeing universal portability?

---

## Decision Drivers

- Maximize rendering performance and leverage explicit modern GPU synchronization via Vulkan.
- Provide reliable fallback on hardware/VMs lacking Vulkan driver support.
- Ensure headless tests and CI runs can execute without physical display servers or GPU hardware.
- Decouple layout and DOM subsystems from concrete GPU driver APIs.

---

## Considered Options

- **Option 1**: **3-tier graphics architecture**: Primary Vulkan backend via `vulkano`, runtime fallback to OpenGL via
  `glow`/`glutin`, and headless fallback to CPU software rasterization, with a declarative `DisplayList` command buffer
  abstraction.
- **Option 2**: Single Vulkan-only backend without fallback (incompatible with legacy systems/VMs).
- **Option 3**: Immediate-mode OpenGL-only rendering backend.
- **Option 4**: `wgpu` WebGPU unified abstraction layer.

---

## Decision Outcome

Chosen option: **Option 1 (Vulkan via `vulkano` + OpenGL fallback via `glow` + CPU software rasterizer)**.

### Rationale

1. **`vulkano` as Primary**:
    - Idiomatic Rust bindings with compile-time Vulkan memory and synchronization safety.
    - Full control over Vulkan render passes, swapchain lifecycle, and command buffer batching.
2. **`glow` + `glutin` as Fallback**:
    - High compatibility across older Linux drivers, legacy macOS, and virtualized GPU environments.
3. **Software CPU Fallback**:
    - Allows pixel-accurate automated unit testing and headless scraping in headless CI environments.
4. **Retained `DisplayList` Abstraction**:
    - Keeps DOM, CSS, and windowing crates completely agnostic of whether Vulkan or OpenGL is actively rendering.
    - Protects against GPU driver crashes by sanitizing draw calls before GPU submission.

### Consequences

- **Positive**:
    - High-performance, modern low-overhead rendering with Vulkan.
    - 100% environment compatibility across desktops, VMs, and headless test servers.
    - Clean separation of concerns between layout calculation and GPU rasterization.
- **Negative**:
    - Maintaining two hardware backend implementations (`VulkanBackend` and `OpenGLBackend`) alongside the CPU fallback.
