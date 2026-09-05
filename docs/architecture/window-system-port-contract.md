# `WindowSystem` / `Presenter` port — ADR-0011 contract record

The `WindowSystem` / `Presenter` seam in `core/window` is a **Replaceable Subsystem Port** under `ADR-0011`. This
document is its contract record: the state of all seven mandatory items at the `I4` freeze point (v0.5 Phase I4,
`alloy <url>` native-window rendering — `alloy/src/application/event_loop.rs` and `alloy/src/main.rs`'s
`run_browse_command`).

| Item | Contract requirement                                                      | State                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ---- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Seam PRD with variation + threat model                                    | ✅ `PRD-010` §2 (variation model: `WindowSystem`/`Presenter` each independently replaceable; threat model: no attacker-controlled bytes on this seam — the concern is misuse ordering, not hostile input, `ADR-0018` row 3)                                                                                                                                                                                                                                                                   |
| 2    | Port traits: assoc types only, no adapter types, object-safe or companion | ✅ Both `WindowSystem` and `Presenter` are object-safe from the start — no generic method, no associated type, every signature speaks only this crate's own boundary types. No companion needed, same shape as `graphics::RenderBackend`/`network::HttpTransport`                                                                                                                                                                                                                             |
| 3    | Boundary aggregates: domain-owned, `#[non_exhaustive]`, schema version    | ✅ `WindowEvent`, `FrameView`, `WindowAttributes`, `WindowError`, … all domain-owned in `core/window`, `#[non_exhaustive]`; `window::PORT_SCHEMA_VERSION = 1`, frozen — see item 7                                                                                                                                                                                                                                                                                                            |
| 4    | Exactly one typed error, source location                                  | ✅ `WindowError`, `#[non_exhaustive]`, one `thiserror` enum; every variant carries the `WindowOperation` attempted, and every variant tied to an existing window carries its `WindowId` — the location metadata for this port                                                                                                                                                                                                                                                                 |
| 5    | Written lifecycle & concurrency contract                                  | ✅ §5 below                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 6    | Conformance suite + reference adapter + `no-<adapter>`                    | ✅ `window::conformance::run_window_suite`; `HeadlessWindowSystem` + `RecordingPresenter` (reference) and `WinitSystem` + `SoftbufferPresenter` (real, feature `winit-system`, default-on) both pass it. `cargo test -p window --no-default-features` links neither `winit` nor `softbuffer` at all — the `layering` CI job holds this. `alloy/tests/e2e_golden.rs` additionally proves `HeadlessWindowSystem`/`RecordingPresenter` drive the real `alloy::run_browser_until` loop end to end |
| 7    | Frozen-API milestone                                                      | ✅ **Frozen at `I4`.** `window::PORT_SCHEMA_VERSION = 1` is that surface. Any future boundary change bumps it and adds a row to §4's migration table below                                                                                                                                                                                                                                                                                                                                    |

---

## 2. Object-safety (item 2)

Neither trait needed a `dyn`-dispatch companion:

- `WindowSystem::create_window(&mut self, attrs: &WindowAttributes) -> Result<WindowId, WindowError>` /
  `WindowSystem::pump_events(&mut self, sink: &mut dyn FnMut(WindowEvent)) -> Result<PumpStatus, WindowError>`
- `Presenter::present(&mut self, frame: FrameView<'_>) -> Result<(), WindowError>`

Every parameter and return type is a concrete, `#[non_exhaustive]` boundary aggregate. `&mut dyn WindowSystem` and
`&mut dyn Presenter` both compile and are exactly what `core/window/src/application/conformance.rs` takes.

---

## 3. Boundary aggregates and the graphics decoupling (item 3)

`FrameView` is deliberately **not** `graphics::Framebuffer` — naming that type here would put `core/graphics` in this
crate's dependency graph, which `ADR-0011` item 2 and the `layering` CI job both forbid (`core/window` names no
`engine`, `rhai`, `dom`, `css`, `graphics`, or `network`). A caller that already depends on both crates (`alloy`) builds
a `FrameView` from a `graphics::Framebuffer` in one borrow, right before calling `Presenter::present` — the seam that
keeps the two ports decoupled while still letting the CPU rasterizer's output reach a real surface.

`WindowError` is one `#[non_exhaustive]` `thiserror` enum. Every variant names the `WindowOperation` attempted
(`create_window`, `pump_events`, `present`, …), and every variant that can be tied to an existing window carries its
`WindowId` — together, the location metadata `ADR-0011:93-95` asks for, adapted to a port with no wire phase to name.

---

## 4. Boundary-schema migrations (`window::PORT_SCHEMA_VERSION`)

| Version | Change                                                                                                       | Adapter action |
| ------- | ------------------------------------------------------------------------------------------------------------ | -------------- |
| **1**   | Surface introduced in v0.5 Phase C2; frozen at integration point `I4` (v0.5 Phase I4, `alloy::run_browser`). | —              |

---

## 5. Lifecycle and concurrency contract (item 5)

### 5.1 Ownership of durable state

**The Skeleton (Rust) owns all durable state** (`ADR-0003`). `WinitSystem` holds the live `winit` event loop and window
handles it was constructed with; `SoftbufferPresenter` holds the surface it blits to. Neither hides any state a consumer
needs to survive a reload — script-local policy state (Phase M's `.rhai` UI logic) lives in the `ExecutionContext`,
never in this crate.

### 5.2 Threading model

- `WindowSystem` is deliberately **not** `Send + Sync`: the window event loop is the sole owner of the main thread
  (`ADR-0019`), and a type that could cross a thread boundary would invite a second loop fighting it for that thread.
  `WinitSystem` must be constructed on the main thread on macOS and Windows, and stay there.
- `Presenter` **is** `Send`: `SoftbufferPresenter` may be handed to a render worker, blitting frames a different thread
  produced.
- `pump_events` is **pull-driven** from the main thread — never a callback that hands the thread away. Blocking I/O
  (`core/network`'s `HttpTransport`, DNS, file reads) never runs on this loop's thread; that is Phase I4's
  `alloy/src/application/event_loop.rs`, not this crate.

### 5.3 Purity and determinism

Not applicable in the sense `CascadeResolver`/`LayoutEngine` use it — this port drives real OS state (a window, a
surface) and is expected to have side effects. What is pinned instead is **ordering safety** (§5.6): a call made before
its precondition holds is a typed refusal, never undefined behaviour.

### 5.4 Re-entrancy and suspension

No suspend/resume point. `pump_events`'s `sink: &mut dyn FnMut(WindowEvent)` callback is invoked synchronously once per
drained event and must not call back into `pump_events` itself — the same non-reentrant discipline
`RuntimeEngine::eval_*` uses for its native bindings, applied here to the event sink.

### 5.5 Cancellation

`PumpStatus::Exit` is the only "stop" signal, reported by `pump_events` itself (e.g. the OS close button); there is no
external cancellation token. `pump_events` called again after `Exit` is a typed `WindowError::EventLoopExited`, never a
panic.

### 5.6 Resource ceilings and fault behaviour

- **Ordering safety**: calling `pump_events` before any `create_window` call has ever succeeded is a typed
  `WindowError::NoWindowYet`, never a panic and never a hang — pinned by
  `check_pump_events_before_create_window_is_refused` in the conformance suite.
- **Creation failure**: a backend that cannot create a window at all (no display server, a headless CI runner) returns
  `WindowError::CreationFailed`, never a process abort. `HeadlessWindowSystem` exists precisely so CI can exercise this
  port with no real display.
- **Reusability**: a second `pump_events` or a second `present` call on the same instance must not corrupt state —
  pinned by `check_pump_events_is_reusable` / `check_presenting_is_reusable`.
- A `.rhai` UI script (Phase M's `WINDOW_BINDINGS`) that panics inside a guarded binding is trapped and falls back
  through `run_with_fallback`; the window keeps pumping events afterward.

### 5.7 Memory ceilings

Not applicable — this port holds no unbounded script- or network-facing buffer; `FrameView` is sized exactly to the
surface it targets.

---

## Audit

Re-run `cargo test -p window` (conformance is `application::conformance::run_window_suite`, exercised from
`core/window/tests/`), `cargo test -p window --no-default-features` (item 6's `no-<adapter>` proof — no `winit`, no
`softbuffer`), `cargo test -p alloy --test e2e_golden` (the full `navigate → subresource → render → present` path over
this port's reference adapters), and `cargo tree -p window` (must name neither `engine`, `rhai`, `dom`, `css`,
`graphics`, nor `network` — item 2/6). Check `window::PORT_SCHEMA_VERSION` against the last recorded value here whenever
`WindowEvent`/`FrameView`/`WindowError`/a trait signature changes, and add a row to §4 for the bump — this boundary is
frozen as of `I4`.
