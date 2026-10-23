# `WindowSystem` / `Presenter` port — ADR-0011 contract record

The `WindowSystem` / `Presenter` seam in `core/window` is a **Replaceable Subsystem Port** under `ADR-0011`. This
document is its contract record: the state of all seven mandatory items at the `I4` freeze point (v0.5 Phase I4,
`alloy <url>` native-window rendering — `alloy/src/application/event_loop/` and `alloy/src/main.rs`'s
`run_browse_command`).

| Item | Contract requirement                                                      | State                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ---- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Seam PRD with variation + threat model                                    | ✅ `PRD-010` §2 (variation model: `WindowSystem`/`Presenter` each independently replaceable; threat model: no attacker-controlled bytes on this seam — the concern is misuse ordering, not hostile input, `ADR-0018` row 3)                                                                                                                                                                                                                                                                   |
| 2    | Port traits: assoc types only, no adapter types, object-safe or companion | ✅ Both `WindowSystem` and `Presenter` are object-safe from the start — no generic method, no associated type, every signature speaks only this crate's own boundary types (`create_window`, `pump_events`, `request_redraw`, `set_title`, `set_cursor`, `present`). No companion needed, same shape as `graphics::RenderBackend`/`network::HttpTransport`                                                                                                                                                               |
| 3    | Boundary aggregates: domain-owned, `#[non_exhaustive]`, schema version    | ✅ `WindowEvent`, `FrameView`, `WindowAttributes`, `WindowError`, … all domain-owned in `core/window`, `#[non_exhaustive]`; `window::PORT_SCHEMA_VERSION = 4` (`1 → 2`: `WindowSystem::request_redraw` added; `2 → 3`: `WindowEvent::TextInput` and `WindowEvent::ControlKey` added, issue #44; `3 → 4`: unified `KeyEvent`, `TextCommitted`, `set_title`, `set_cursor`, issue #112 — see §4) |
| 4    | Exactly one typed error, source location                                  | ✅ `WindowError`, `#[non_exhaustive]`, one `thiserror` enum; every variant carries the `WindowOperation` attempted, and every variant tied to an existing window carries its `WindowId` — the location metadata for this port                                                                                                                                                                                                                                                                 |
| 5    | Written lifecycle & concurrency contract                                  | ✅ §5 below                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 6    | Conformance suite + reference adapter + `no-<adapter>`                    | ✅ `window::conformance::run_window_suite`; `HeadlessWindowSystem` + `RecordingPresenter` (reference) and `WinitSystem` + `SoftbufferPresenter` (real, feature `winit-system`, default-on) both pass it. `cargo test -p window --no-default-features` links neither `winit` nor `softbuffer` at all — the `layering` CI job holds this. `alloy/tests/e2e_golden.rs` additionally proves `HeadlessWindowSystem`/`RecordingPresenter` drive the real `alloy::run_browser_until` loop end to end |
| 7    | Frozen-API milestone                                                      | ✅ **Frozen at `I4`** (`alloy <url>` is on `main`, `7aa4bdb`). Every change since needs a `PRD-010` §6 migration note: `2 → 3` (#44, landed after the freeze without one — backfilled) and `3 → 4` (#112, `ADR-0027`) are recorded in §4 and in `PRD-010` §6. `window::PORT_SCHEMA_VERSION = 4` is the current surface |

---

## 2. Object-safety (item 2)

Neither trait needed a `dyn`-dispatch companion:

- `WindowSystem::create_window(&mut self, attrs: &WindowAttributes) -> Result<WindowId, WindowError>` /
  `WindowSystem::pump_events(&mut self, sink: &mut dyn FnMut(WindowEvent)) -> Result<PumpStatus, WindowError>` /
  `WindowSystem::request_redraw(&mut self)` / `WindowSystem::set_title(&mut self, title: WindowTitle)` /
  `WindowSystem::set_cursor(&mut self, icon: CursorIcon)`
- `Presenter::present(&mut self, frame: FrameView<'_>) -> Result<(), WindowError>`

`request_redraw` is a command (Command–Query Separation): no return value, no failure channel, a silent no-op before any
window exists. It asks the backend to deliver a `WindowEvent::RedrawRequested` on a following `pump_events`. It exists
because a repaint signal cannot come only from content changes: a Wayland compositor can drop the first `present` onto a
not-yet-configured surface, and expose/occlusion redraws have no other trigger. The event loop calls it after every
frame it presents; `WinitSystem` forwards to `winit`'s `Window::request_redraw`, `HeadlessWindowSystem` enqueues a
`RedrawRequested` for its next `pump_events`.

`set_title` and `set_cursor` (v4) are commands of the same kind: no return value, no failure channel, a silent no-op
before any window exists. `set_title` backs the page title in the title bar (#84) and `set_cursor` the link-hover
pointer (#68). `WinitSystem` forwards to `Window::set_title` / `Window::set_cursor`; `HeadlessWindowSystem` records the
last value, readable through its `title()` and `cursor()` queries so a test can assert on it.

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

### 3.1 Input vocabulary (`ADR-0027`)

A key press is **one** event, `WindowEvent::Key(KeyEvent)`, carrying everything about it: `logical` (`LogicalKey`,
including `Character(InputText)`), `physical` (`KeyCode`, the layout-independent fallback for chords), `text`
(`Option<InputText>`), `modifiers` (`Modifiers`, including `alt_graph()`) and `state` (`Pressed | Repeated | Released`).
`WindowEvent::TextCommitted(InputText)` is for input-method commits only.

- The adapter never classifies a press as shortcut or typing; the consumer's keymap does (`core/ui`, #113).
- `text` is `winit`'s `KeyEvent::text`, which ignores Ctrl on Linux: **Ctrl+L carries `"l"`**. A consumer must read
  `modifiers` before treating `text` as typing. The adapter does not use `text_with_all_modifiers`.
- `KeyboardState` (`infrastructure/keyboard.rs`) is the adapter's only memory: it stamps `ModifiersChanged` onto each
  event and tracks `NamedKey::AltGraph` press/release (cleared on focus loss) because `winit`'s `ModifiersState` has no
  AltGr bit. Windows reports AltGr as Ctrl+Alt, so `alt_graph()` is never set there.
- `InputText` is non-empty and free of control characters; the adapter drops `\r`, `\x08` and the like from `text` and
  keeps the press (as `LogicalKey::Enter`, `Backspace`, …).
- `winit::event::KeyEvent` cannot be constructed outside `winit`, so the mapping is split at `KeyInput` (the five fields
  it reads) and unit-tested through it (`core/window/tests/event_map.rs`).

---

## 4. Boundary-schema migrations (`window::PORT_SCHEMA_VERSION`)

| Version | Change                                                                                                                                                                                                                                                                                   | Adapter action                                                                                                    |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **1**   | Surface introduced in v0.5 Phase C2.                                                                                                                                                                                                                                                     | —                                                                                                                 |
| **2**   | `WindowSystem::request_redraw(&mut self)` added — the event loop needs an explicit repaint signal so a Wayland compositor that dropped the first `present` still gets the frame, and so expose/occlusion redraws are served at all. Landed during v0.5 Phase I4, before the `I4` freeze. | Every `WindowSystem` impl adds the method. A backend with no discrete redraw request may implement it as a no-op. |
| **3**   | `WindowEvent::TextInput` and `WindowEvent::ControlKey` added to capture typed text and logical control keys for Omnibox (issue #44). `window.set_ime_allowed(true)` enabled on window creation. | Callers matching exhaustively on `WindowEvent` handle the new variants or rely on a `_` arm. |
| **4**   | One `WindowEvent::Key(KeyEvent)` per physical press, carrying logical key, physical key, `text`, `Modifiers` (with `alt_graph()`) and `Pressed \| Repeated \| Released` (`ADR-0027`, issue #112). `TextInput` and `ControlKey` removed; `TextCommitted(InputText)` carries IME commits only. `LogicalKey` gains `Character(InputText)` and more named keys and loses `Copy`. `KeyState` gains `Repeated`. `WindowSystem::set_title(WindowTitle)` and `set_cursor(CursorIcon)` added. | See the migration note below. |

**`3 → 4` migration note.** There is no `ControlKey` alias.

- `WindowSystem` implementers add `set_title` and `set_cursor` (a silent no-op before a window exists is conformant).
- `WindowEvent::Key { code, pressed }` becomes `WindowEvent::Key(KeyEvent)`: read `physical()` for `code`, and
  `state().is_pressed()` / `is_released()` for `pressed` (an auto-repeat is `Repeated`, not a second `Pressed`).
- `WindowEvent::TextInput { text }` from a key press becomes `KeyEvent::text()` on that press. It is **not** a separate
  event: handle it only after checking `modifiers()` — Ctrl+L carries `"l"`.
- `WindowEvent::ControlKey { key, state }` becomes a `Key` whose `logical()` is the same `LogicalKey` variant.
- An input-method commit is `WindowEvent::TextCommitted(InputText)`.
- Stop reconstructing modifiers from raw `Key` events; read `KeyEvent::modifiers()`.

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
  `alloy/src/application/event_loop/`, not this crate.

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
  `check_pump_events_before_create_window_is_refused` in the conformance suite. `request_redraw`, `set_title` and
  `set_cursor` have no failure channel, so the same premature call is a **silent no-op** instead — pinned by
  `check_window_commands_before_create_window_are_silent`.
- **Creation failure**: a backend that cannot create a window at all (no display server, a headless CI runner) returns
  `WindowError::CreationFailed`, never a process abort. `HeadlessWindowSystem` exists precisely so CI can exercise this
  port with no real display.
- **Reusability**: a second `pump_events`, `present`, `request_redraw`, `set_title` or `set_cursor` call on the same
  instance must not corrupt state — pinned by `check_pump_events_is_reusable` / `check_window_commands_are_reusable` /
  `check_presenting_is_reusable`.
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
frozen as of `I4`: every bump also needs a `PRD-010` §6 note.
