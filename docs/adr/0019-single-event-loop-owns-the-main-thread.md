# ADR-0019: A Single Event Loop Owns the Main Thread

- **Status**: Accepted (drafted in v0.5 Phase 0; finalised in Phase C2 / P)
- **Deciders**: Architecture Team
- **Date**: 2026-09-03 (accepted 2026-09-05)

---

## Context and Problem Statement

The roadmap marks "two loops fighting over the main thread" as a hazard of integration point **I5** (`ROADMAP:318`), and
I5 is v0.9. But v0.5 is the **first version to have a loop at all**: `winit` takes ownership of the main thread, and
that ownership is irreversible once network I/O, hot-reload watching (F11) and a JS event loop (F10) arrive. Deciding
the model in v0.5 costs design; deciding it in v0.9 costs a rewrite.

`core/network` also needs blocking I/O — DNS (`std::net::ToSocketAddrs`), TCP connect, the TLS handshake, body reads —
and `RuntimeEngine` is a **synchronous** trait (`PRD-002`). Introducing an async executor just to cross it at every hook
buys a problem nobody asked for.

---

## Decision Drivers

- `winit`'s `EventLoop::run` consumes the calling thread and must be the main thread on macOS and Windows.
- No async runtime: the `RuntimeEngine` signature is not `async`, and the `<10μs` hook budget (`PRD-001:96`) has no room
  for executor overhead per traversal.
- F11 (hot-reload watcher) and F10 (JS event loop) must slot into whatever model v0.5 picks without a redesign.

---

## Considered Options

- **Option 1 — `tokio` (or any async executor) drives everything**, `winit` on a spawned thread. Fights `winit`'s
  main-thread requirement on macOS/Windows; drags an executor across every synchronous hook.
- **Option 2 — the window event loop owns the main thread; blocking I/O runs on a `std` thread pool and returns results
  as loop events over a channel.** No async runtime. Each future producer (hot-reload watcher, JS loop) is one more
  sender / consumer on the same loop.

---

## Decision Outcome

Chosen option: **Option 2.**

- **The window event loop is the sole owner of the main thread.** `WindowSystem::pump_events` is pull-driven from that
  thread.
- **All blocking I/O runs on a worker of a `std::thread` pool.** `HttpTransport::execute`, DNS resolution, and file
  reads never run on the main thread. Results return over `std::sync::mpsc` as an event the same loop drains.
- **No async runtime, no `tokio`.** The `RuntimeEngine` trait stays synchronous.
- `Presenter` is `Send` (it may live on a render worker); `WindowSystem` is deliberately **not** `Send + Sync` (it owns
  the main thread).

This is written as a contract item in `docs/architecture/window-system-port-contract.md` (ADR-0011 item 5), not left as
a convention F10/F11 could unknowingly break.

### Consequences

- **Positive**: I5 is largely pre-solved. The F11 watcher is one more producer on the channel; the F10 JS loop is one
  more consumer on the same loop. No executor to thread through hooks.
- **Negative**: a hand-rolled thread pool + channel plumbing in `alloy/src/application/event_loop/` instead of a
  batteries-included runtime. Long CPU-bound work on the main thread (large relayout) still blocks the loop — mitigated
  by per-frame coalescing, not by preemption.

---

## Addendum — 2026-09-05: repaint is a distinct signal from relayout (v0.5 Phase I4)

The first `alloy <url>` sessions on Wayland opened a window that stayed blank. Two gaps in the pull-driven loop:

1. **The loop presented a frame only when content changed** (`session.dirty`: a navigation, resize, image, or
   stylesheet). It never asked the platform to repaint and it dropped `WindowEvent::RedrawRequested`. On Wayland the
   first `present` frequently lands on a surface the compositor has not configured yet and is discarded — with no redraw
   path, no further frame is ever produced.
2. **The main-document fetch rendered a silently-empty document.** A non-`2xx`, an empty body, or a non-UTF-8 body all
   became `html::parse("")` → a blank white window, logged only as success. (Fixed in
   `alloy/src/application/navigation.rs`: those are now typed `AlloyError`s that fall back to the visible error card.)

**Decision.** _Relayout_ (rebuild the display list, bump `LoopStats::relayouts`) and _repaint_ (re-blit the cached
frame, no pipeline) are separate operations. `WindowSystem` gains **`request_redraw(&mut self)`** — a
Command–Query-Separation command, no return, no failure channel, a silent no-op before a window exists — asking the
backend to deliver a `RedrawRequested` on a following `pump_events`. The loop:

- caches the pixels of each presented frame in the `Session`;
- calls `request_redraw()` after every relayout, so a compositor that dropped the present re-delivers a redraw once the
  surface is live;
- serves `RedrawRequested` (compositor expose/occlusion included) with a **repaint** — no relayout, `relayouts`
  untouched, so the I4 per-pump coalescing proof is unaffected.

`request_redraw` is the only idiomatic winit signal robust against compositors that repaint only when asked. `alloy`
owns every `WindowSystem` impl (`WinitSystem` → `Window::request_redraw`; `HeadlessWindowSystem` → enqueue a
`RedrawRequested`), so the port method is small. `window::PORT_SCHEMA_VERSION` moves `1 → 2`; the
`window-system-port-contract.md` migration table records it. The port had not yet frozen (`I4` is not merged to `main`),
so this is a pre-freeze change, not a contract break.

---

## Amendment — 2026-10-23: navigation supersession by `NavigationId` (#111, `ADR-0026`)

The loop used to supersede a navigation by **replacing the channel** its workers report on (`e832b6a`): dropping the old
receiver discarded what its workers had already queued and made their later `send` fail. That kept stale results out,
but only a real thread, a real channel and a sleep could prove it.

**Decision.** The document state moves into the pure `core/browser` crate, which never spawns or owns a channel. Every
navigation mints a `NavigationId { context, sequence }`; every `BrowserEffect::Fetch` is stamped with it, the worker
sends back a `FetchResult` carrying the same stamp, and `BrowsingContext::complete` drops (and logs) any result whose id
is not the latest. This covers subresources too: a stylesheet or image result from the page the user left is dropped, so
an `ImageId` — a DOM node index every document reuses — cannot name an `<img>` of the new page.

The trade: stamping is a comparison the channel swap did not need, and one channel now lives for the whole session. What
it buys is a browser crate whose supersession rules are plain unit tests (`core/browser` `browsing_context/tests.rs`)
with no threads and no sleeps. `alloy/src/application/event_loop/worker.rs` is the thin executor: one `std::thread` per
`FetchRequest`, unchanged in spirit (thread per fetch, no pool, no cancellation).

Link clicks use the same discipline: a `RegionId` carries a `LayoutEpoch` that bumps on every layout and document swap,
and a stale one is logged and ignored instead of resolving to whatever now sits at its index.

---

## Amendment — 2026-10-27: three damage signals and a layered compositor (#108, epic #110)

The addendum above has two signals: relayout (everything) and repaint (re-present the cached frame). A change to the
chrome alone — omnibox text, focus, selection — fits neither: it needs new chrome pixels but not a new layout. With only
two signals it had no correct path, so typing was invisible until the next relayout (#116), and a viewport shorter than
the chrome made the relayout fail and end the session (#118).

**Decision.** `alloy`'s `Compositor` (`alloy/src/application/compositor/`) owns the raster backends, reaches them only
through the `RenderBackend` port (injected at the composition root), and caches the **chrome** and **content** layers as
separate pixel buffers. A change is reported as `Damage`, accumulated per pump cycle:

| Signal                 | Re-rasters                       | Bumps `stats.relayouts` |
| ---------------------- | -------------------------------- | ----------------------- |
| `Damage::Chrome`       | the chrome strip, then recompose | **never**               |
| `Damage::Content`      | cascade → layout → paint → content raster, then recompose | yes — the only path |
| `Damage::Viewport`     | both layers, at sizes recomputed from the viewport | only if content rows were laid out |
| `RedrawRequested`      | nothing — re-presents the composed frame | no       |

- **`Damage::Chrome` never increments `stats.relayouts`.** Typing in the omnibox must not relayout the DOM (epic #43),
  and the presented chrome rows must show the typed text.
- **Coalescing is unchanged in kind:** damage is one flag per layer, so N signals in a pump cycle cost at most one
  chrome raster and one content relayout, and one present.
- **An empty content layer is valid.** A viewport no taller than the chrome composes the top of the chrome and no
  content; the document is not laid out and `relayouts` does not move. Growing the viewport lays it out again.
- **Nothing is presented until both layers exist:** a chrome change before the first document is rastered but not shown,
  as a resize before the first document always was.
- `RenderBackend` gains `bind_images` (`graphics::PORT_SCHEMA_VERSION` 3 → 4, `PRD-005` §6): the compositor keeps one
  long-lived backend per layer, so the content backend's image source must follow the current document instead of being
  fixed at construction. Fonts stay constructor-bound.

The trade: two backends and two cached buffers instead of one, and a compose (a memory copy of the viewport) on every
chrome change. What it buys is that the chrome and the page invalidate independently, and that the compositor is the one
place that knows how a viewport divides into layers.
