# `HttpTransport` / `RequestPolicy` port — ADR-0011 contract record

The `HttpTransport` / `RequestPolicy` seam in `core/network` is a **Replaceable Subsystem Port** under `ADR-0011`. This
document is its contract record: the state of all seven mandatory items at the `I4` freeze point (v0.5 Phase I4,
`alloy <url>` native-window rendering — `alloy/src/application/{navigation,subresource,event_loop}.rs`).

| Item | Contract requirement                                                      | State                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ---- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Seam PRD with variation + threat model                                    | ✅ `PRD-009` §2 (variation model: `HttpTransport`/`RequestPolicy` each independently replaceable; threat model: every byte off the wire is hostile by construction, `ADR-0018` row 1)                                                                                                                                                                                                                                             |
| 2    | Port traits: assoc types only, no adapter types, object-safe or companion | ✅ Both `HttpTransport` and `RequestPolicy` are object-safe from the start — no generic method, no associated type, every signature speaks only this crate's own boundary types. No companion needed, same shape as `graphics::RenderBackend`/`css::CascadeResolver`                                                                                                                                                              |
| 3    | Boundary aggregates: domain-owned, `#[non_exhaustive]`, schema version    | ✅ `HttpRequest`, `HttpResponse`, `NetworkError`, `PolicyVerdict` all domain/application-owned in `core/network`, `#[non_exhaustive]`; `network::PORT_SCHEMA_VERSION = 1`, frozen — see item 7                                                                                                                                                                                                                                    |
| 4    | Exactly one typed error, source location                                  | ✅ `NetworkError`, `#[non_exhaustive]`, one `thiserror` enum; every variant carries a `ProtocolPhase` (the location metadata) and, where the cause is a closed set, a typed defect enum (`UrlDefect`, `FramingDefect`, `MalformedPart`, `RedirectDefect`, `DecodeDefect`, `WireLimit`)                                                                                                                                            |
| 5    | Written lifecycle & concurrency contract                                  | ✅ §5 below                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 6    | Conformance suite + reference adapter + `no-<adapter>`                    | ✅ `network::conformance::run_transport_suite`; `MockTransport` + `AllowAllPolicy` (reference) and `RealHttpTransport` (real, feature `real-transport`) both pass it. `cargo test -p network --no-default-features` builds and tests with no real transport linked — the `layering` CI job holds this. `alloy/tests/e2e_golden.rs` additionally proves `MockTransport` drives the real `alloy::run_browser_until` loop end to end |
| 7    | Frozen-API milestone                                                      | ✅ **Frozen at `I4`.** `network::PORT_SCHEMA_VERSION = 1` is that surface. Any future boundary change bumps it and adds a row to §4's migration table below                                                                                                                                                                                                                                                                       |

---

## 2. Object-safety (item 2)

Neither trait needed a `dyn`-dispatch companion — both were designed object-safe from `C1`:

- `HttpTransport::execute(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError>`
- `RequestPolicy::decide(&self, request: &HttpRequest) -> Result<PolicyVerdict, NetworkError>`

Every parameter and return type is a concrete, `#[non_exhaustive]` boundary aggregate — no `impl Trait`, no generic
method. `&dyn HttpTransport` and `&dyn RequestPolicy` both compile and are exactly what
`core/network/src/application/conformance.rs` takes.

---

## 3. Boundary aggregates and threat surface (items 3/4)

`HttpRequest` is built from validated value objects (`Url`, `Method`, `HeaderMap`, `Body`); `HttpResponse` carries a
**fully decoded** body — content-coding and charset already undone by `infrastructure::decode` and
`infrastructure::charset` — so no consumer of this port ever sees compressed bytes or foreign-charset text. No `rustls`,
`TcpStream`, or `webpki` type appears in `application::ports` or in any boundary aggregate; those names exist only
inside `infrastructure`, which is exactly what keeps a future transport swap (HTTP/2, a caching layer, a proxy) from
touching a consumer.

`NetworkError` is one `#[non_exhaustive]` `thiserror` enum. Every variant carries a `ProtocolPhase` — the "where on the
wire it broke" location metadata `ADR-0011:93-95` requires — and, wherever the failure is a closed set of causes, a
typed defect enum rather than a free-form string: `UrlDefect` (malformed URL text), `MalformedPart` (a bad HTTP head),
`FramingDefect` (a body that lied about its own length or chunking), `RedirectDefect` (a cycle or a bound exceeded),
`DecodeDefect` (an unsupported charset or content-coding), `WireLimit` (a hostile size hit a cap).

---

## 4. Boundary-schema migrations (`network::PORT_SCHEMA_VERSION`)

| Version | Change                                                                                                                                                                   | Adapter action |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------- |
| **1**   | Surface introduced in v0.5 Phase C1 (transport) and Phase M (policy); frozen at integration point `I4` (v0.5 Phase I4, `alloy::run_browser`/`navigation`/`subresource`). | —              |

---

## 5. Lifecycle and concurrency contract (item 5)

### 5.1 Ownership of durable state

**The Skeleton (Rust) owns all durable state** (`ADR-0003`). `RealHttpTransport` holds no state between calls beyond its
own connection-pool configuration (if any); `MockTransport` holds an immutable fixture map set at construction. Neither
adapter remembers anything about one `execute` call when the next one arrives. `RequestPolicy` implementations are
required to be pure — see §5.3.

### 5.2 Threading model

- Both traits are `Send + Sync`. One adapter value may be shared across threads (`&dyn HttpTransport`,
  `&dyn RequestPolicy`) with no interior mutability to race on.
- Both methods take `&self` and may be called concurrently from multiple threads on the same adapter instance.
- **Synchronous and blocking, by `ADR-0019` decision.** There is no async runtime in this workspace and the window event
  loop owns the main thread. A consumer runs `HttpTransport::execute` on a worker of a `std::thread` pool and receives
  the `HttpResponse` back over `std::sync::mpsc` as one more loop event — Phase I4's
  `alloy/src/application/event_loop.rs`, not this crate's job. `RequestPolicy::decide` is required to be cheap enough to
  run inline on whichever thread calls it (§5.3).

### 5.3 Purity and determinism

`RequestPolicy::decide` must be pure and I/O-free by contract: it opens nothing and blocks on nothing. This is what lets
"policy runs before mechanism" (§3.2 of `PRD-009`) cost nothing when it denies — a policy that needed I/O to decide
would belong on the other side of the seam. `HttpTransport::execute` is **not** required to be pure (a real transport
does I/O by definition); it is required to be **total** (§5.6) instead.

### 5.4 Re-entrancy and suspension

There is no suspend/resume point on this port. `execute` and `decide` are ordinary blocking calls to completion — unlike
`RuntimeEngine`'s native-binding re-entrancy concern, nothing on this port calls back into script or another port
mid-call.

### 5.5 Cancellation

There is no cooperative cancellation token. Every socket operation is under a timeout
(`infrastructure::deadline::{Deadline, PhaseTimeouts}`); a `HttpTransport` that exceeds its budget returns a typed
`NetworkError`, it does not hang waiting for an external cancel signal.

### 5.6 Resource ceilings and fault behaviour

- **Wire limits**: `infrastructure::limits::{ByteCap, FieldCap, WireLimits}` bound header size, body size, and field
  count — a peer that tries to exhaust memory with an unbounded head or body hits a typed `WireLimit` refusal, never an
  unbounded allocation.
- **Redirect limits**: `infrastructure::redirect::RedirectLimit` bounds a redirect chain; a cycle or an excessive chain
  is `NetworkError::Redirect { defect: RedirectDefect::.. }`, never an infinite loop.
- **Fault behaviour**: `execute`/`decide` are always `Result`; a hostile or broken peer, a lying `Content-Length`, a
  malformed chunk, a rejected TLS handshake — all typed `NetworkError`, never a panic (pinned against a real server in
  `core/network/tests/hostile_responses.rs`). This is the same trapping discipline `PRD-003:62-70` establishes for the
  script engine, applied here to hostile network input instead of hostile script.
- A `.rhai` `RequestPolicy` (Phase M) that panics is trapped by `rhai-bindings/tests/fault_injection.rs`'s matrix and
  falls back to the built-in Rust policy via `run_with_fallback` — the same fallback contract every muscle script gets.

### 5.7 Memory ceilings

Bounded by the wire limits of §5.6 — unlike `RuntimeEngine`'s v0.1 gap (unbounded script-local memory), this port's
attacker-facing surface (the HTTP head and body) is capped by construction, because that is exactly the surface
`ADR-0018` row 1 exists to protect.

---

## Audit

Re-run `cargo test -p network` (conformance is `application::conformance::run_transport_suite`, exercised from
`core/network/tests/`), `cargo test -p network --no-default-features` (item 6's `no-<adapter>` proof),
`cargo test -p rhai-bindings --test fault_injection -- --test-threads=1` (Phase M's policy-panic row), and
`cargo test -p alloy --test e2e_golden` (the full `navigate → subresource → render` path over this port's reference
adapters). `just layering` additionally proves `core/network` links neither `engine`, a script runtime, nor
`dom`/`css`/`graphics`. Check `network::PORT_SCHEMA_VERSION` against the last recorded value here whenever
`HttpRequest`/`HttpResponse`/`NetworkError`/a trait signature changes, and add a row to §4 for the bump — this boundary
is frozen as of `I4`.
