# PRD-010: Window System and Presenter Ports

- **Status**: Accepted
- **Author**: Core Architecture Team
- **Date**: 2026-09-05 (retrofitted to the `ADR-0011` Replaceable Port Contract for the `core/window` crate delivered in
  v0.5 Phase C2)
- **Target Release**: v0.5

---

## 1. Executive Summary

`core/window` exposes a native window and its event queue (`WindowSystem`, mechanism, owns the main thread) on one side,
and a place to blit rendered pixels (`Presenter`, mechanism, may live on a render worker) on the other (`ADR-0011`).
Policy — when to repaint, how to react to a resize — is Phase M's `.rhai` muscle, not this crate. This PRD is the seam
PRD `ADR-0011` item 1 requires and names the variation and threat models the `C2` `winit` + `softbuffer` implementation
and `ADR-0019` (single event loop) already assume.

---

## 2. Variation and Threat Models

### 2.1 Variation model

A consumer may replace:

- **`WindowSystem`** — the mechanism that opens a window and pumps its event queue. The shipped adapter is `WinitSystem`
  (`winit` 0.30); a headless build swaps in `HeadlessWindowSystem`, used by every CI test and by the golden e2e suite.
- **`Presenter`** — the mechanism that turns a rendered frame into pixels on a surface. The shipped adapter is
  `SoftbufferPresenter`; `RecordingPresenter` (headless) records frames for a byte-exact comparison instead of drawing
  them.

`FrameView` is deliberately not `graphics::Framebuffer` — naming that type here would put `core/graphics` in this
crate's dependency graph. A caller that already depends on both crates (`alloy`) builds a `FrameView` from a
`graphics::Framebuffer` in one borrow, right before calling `Presenter::present`, which is what keeps the two ports
decoupled.

### 2.2 Threat model

This port's input is **not** hostile in the `ADR-0018` row-1 sense: `WindowSystem`/`Presenter` process OS window-manager
and input-device events and blit a buffer this workspace already produced into OS-owned shared memory — never
attacker-controlled network bytes. `winit` and `softbuffer` are `ADR-0018` row-3 platform FFI with no `unsafe`-free
alternative on any of the three target OSes, recorded in `unsafe-allowlist.toml`.

What this port _does_ have to defend against is **misuse ordering**, not hostile bytes:

- Calling `pump_events` before any `create_window` call has succeeded — must be a typed `WindowError::NoWindowYet`,
  never a panic and never a hang.
- A backend refusing to create a window (no display server, headless CI runner) — must be a typed `WindowError`, never a
  process abort. `HeadlessWindowSystem` exists precisely so CI can exercise this port with no real display.
- A second `pump_events` or a second `present` call on the same instance must not corrupt state — both adapters are
  built to be called repeatedly across a session's lifetime.

---

## 3. Architecture & Port Specifications

### 3.1 Boundary aggregates (owned by `core/window`, `#[non_exhaustive]`, versioned)

- Value objects: `WindowEvent` / `PointerButton`, `SurfaceSize` / `ScaleFactor` / `PhysicalPosition`, `FrameView`,
  `WindowAttributes` / `WindowTitle` / `WindowId`, `KeyCode`.
- `WindowError`, `#[non_exhaustive]`, one `thiserror` enum tagged with `WindowOperation` (`ADR-0011` item 4).
- `window::PORT_SCHEMA_VERSION` — the single version knob (`core/window/src/lib.rs`).

### 3.2 `WindowSystem` trait (`window::application::ports`)

```rust
pub trait WindowSystem {
    fn create_window(&mut self, attrs: &WindowAttributes) -> Result<WindowId, WindowError>;
    fn pump_events(&mut self, sink: &mut dyn FnMut(WindowEvent)) -> Result<PumpStatus, WindowError>;
}
```

Deliberately **not** `Send + Sync`: the window event loop is the sole owner of the main thread (`ADR-0019`), and a type
that could cross a thread boundary would invite a second loop fighting it for that thread. `pump_events` is pull-driven
from the main thread — never a callback that hands the thread away.

### 3.3 `Presenter` trait (`window::application::ports`)

```rust
pub trait Presenter: Send {
    fn present(&mut self, frame: FrameView<'_>) -> Result<(), WindowError>;
}
```

`Send` on purpose — `SoftbufferPresenter` may be handed to a render worker, blitting frames a different thread produced.

### 3.4 Threading (`ADR-0019`)

The window event loop is the **sole owner of the main thread**; `WinitSystem` must be constructed on the main thread on
macOS and Windows and stay there. Blocking I/O (`core/network`'s `HttpTransport`, DNS, file reads) never runs on this
loop's thread — that is Phase I4's `alloy/src/application/event_loop.rs`, not this crate.

### 3.5 Script-driven adapters

Phase M's scriptable UI policy (`WINDOW_BINDINGS` in `core/runtime/rhai-bindings/src/window_bindings.rs`) runs under
`Capability::ui_window()` (`WINDOW_MANAGE | GRAPHICS_DRAW | DOM_READ`) and decides _when_ to repaint or how to react to
a resize — it never implements `WindowSystem` or `Presenter` itself. The two ports stay Rust-only mechanism; policy is
muscle.

### 3.6 Reference implementations

`WinitSystem` + `SoftbufferPresenter` (feature `winit-system`, default-on) are the shipped adapters.
`HeadlessWindowSystem` + `RecordingPresenter` are the always-available reference pair the conformance suite and every
golden e2e test run against instead of a real display.

---

## 4. Requirements & Invariants

1. **Ordering safety**: `pump_events` before any successful `create_window` is a typed error, never a panic.
2. **No foreign types**: no `winit` or `softbuffer` type appears in `application::ports`; no `graphics::Framebuffer`
   appears anywhere in this crate — `FrameView` is the seam that keeps that edge from existing.
3. **`--no-default-features` (`no-window`) links neither `winit` nor `softbuffer` at all** — proven by the `layering` CI
   job's `cargo tree -p window --no-default-features` check.
4. **Layering** (`ADR-0002` / arch-lint): `core/window` names no `engine`, no `rhai`, no `dom`, no `css`, no `graphics`,
   no `network`. The scriptable UI-policy adapter lives in `rhai-bindings`.
5. **`unsafe`** (`ADR-0018`): `#![forbid(unsafe_code)]` on this crate without exception; `winit` and `softbuffer` are
   the row-3 nominal exceptions recorded in `unsafe-allowlist.toml`.

---

## 5. Acceptance Criteria

- [x] `WindowSystem` and `Presenter` traits defined in `core/window`, both object-safe, no companion needed.
- [x] `window::conformance::run_window_suite` — surface-size round trip, refusal ordering, reusability — passed by both
      `WinitSystem`/`SoftbufferPresenter` and `HeadlessWindowSystem`/`RecordingPresenter`.
- [x] `cargo test -p window --no-default-features` builds and passes with neither `winit` nor `softbuffer` linked.
- [x] `cargo tree -p window` names no `engine`, `rhai`, `dom`, `css`, `graphics`, or `network` (`layering` CI job).
- [x] `WINDOW_BINDINGS` scriptable UI policy: a script without `WINDOW_MANAGE` that calls a window-control binding gets
      `EngineError::PermissionDenied`; a panic inside a guarded binding falls back safely and the window keeps pumping
      events (`core/runtime/rhai-bindings/tests/scriptable_window.rs`, v0.5 Phase M).
- [x] `window::PORT_SCHEMA_VERSION` and the event/attribute vocabulary frozen at integration point `I4`
      (`alloy::run_browser`, `docs/v0-5-handoff/06-i4-alloy-url.md`) — see §6.

---

## 6. Boundary-schema migrations (`window::PORT_SCHEMA_VERSION`)

| Version | Change                                                                                                       | Adapter action |
| ------- | ------------------------------------------------------------------------------------------------------------ | -------------- |
| **1**   | Surface introduced in v0.5 Phase C2; frozen at integration point `I4` (v0.5 Phase I4, `alloy::run_browser`). | —              |
