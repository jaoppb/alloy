# ADR-0019: A Single Event Loop Owns the Main Thread

- **Status**: Proposed (drafted in v0.5 Phase 0; finalised in Phase C2 / P)
- **Deciders**: Architecture Team
- **Date**: 2026-09-03

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
- **Negative**: a hand-rolled thread pool + channel plumbing in `alloy/src/application/event_loop.rs` instead of a
  batteries-included runtime. Long CPU-bound work on the main thread (large relayout) still blocks the loop — mitigated
  by per-frame coalescing, not by preemption.
