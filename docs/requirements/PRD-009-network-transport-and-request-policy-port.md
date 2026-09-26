# PRD-009: HTTP Transport and Request-Policy Ports

- **Status**: Accepted
- **Author**: Core Architecture Team
- **Date**: 2026-09-05 (retrofitted to the `ADR-0011` Replaceable Port Contract for the `core/network` crate delivered
  in v0.5 Phase C1)
- **Target Release**: v0.5

---

## 1. Executive Summary

`core/network` is the render pipeline's network stage: an `HttpRequest` goes in, a `RequestPolicy` judges it, an
`HttpTransport` performs it, and a fully-decoded `HttpResponse` (or a typed `NetworkError` naming the wire phase it
failed in) comes out. Mechanism (_how_ bytes are fetched) and policy (_whether_ they may be) are two separate ports on
purpose (`ADR-0011`), so Phase M's `.rhai`-scripted network policy plugs into the second seam without touching the
first. This PRD is the seam PRD `ADR-0011` item 1 requires and names the variation and threat models the `C1` transport
implementation and the `C0` TLS-provider spike already assume.

---

## 2. Variation and Threat Models

### 2.1 Variation model

A consumer may replace:

- **`HttpTransport`** — the mechanism. The shipped adapter is a hand-written HTTP/1.1 client over `rustls`; a future
  adapter could add HTTP/2, a caching layer, or route through a corporate proxy, without any consumer of the port
  noticing beyond the `Result` it gets back.
- **`RequestPolicy`** — the policy. The shipped default is `AllowAllPolicy` (send everything as asked); Phase M's
  `.rhai` policy can allow, deny, or rewrite a request (`PolicyVerdict`) under `Capability::network_interceptor()`
  (`NETWORK_FETCH | FS_WRITE_CACHE`) without touching `HttpTransport` at all.

`MockTransport` (the in-repo reference mechanism adapter, fixture-driven) is what the golden e2e tests of I2/I4 and the
conformance suite exercise instead of real sockets.

### 2.2 Threat model

Every byte `HttpTransport::execute` reads comes from a network peer and is **hostile by construction** (`ADR-0018` row
1: this is exactly the surface third-party `unsafe` is forbidden on). The threat model in scope:

- A peer that lies about `Content-Length`, sends a malformed chunked body, or never closes the connection — must produce
  a typed `NetworkError` (`FramingDefect` / `WireLimit`), never a hang or an unbounded allocation
  (`infrastructure::limits::{ByteCap, FieldCap, WireLimits}`, `infrastructure::deadline::{Deadline, PhaseTimeouts}`).
- A peer with an invalid or self-signed TLS certificate — must produce `NetworkError::HandshakeRejected`, never a silent
  downgrade to plaintext or a bypassed verification step.
- A redirect chain that cycles or exceeds a bound — `RedirectDefect`, via `infrastructure::redirect::RedirectLimit`.
- A response whose declared charset or content-encoding this crate cannot decode — a typed `DecodeDefect`, never a panic
  on the decode path (`infrastructure::decode`, `infrastructure::charset`, `infrastructure::inflate`).

A `RequestPolicy` is, by contrast, **trusted input**: it is either the built-in Rust default or a `.rhai` muscle script
the user authored (`PRD-003:21-24` — buggy, not adversarial). `RequestPolicy::decide` is pure and I/O-free by contract,
so a misbehaving policy can deny or misroute a request but cannot itself open a socket, block, or corrupt the
transport's state.

---

## 3. Architecture & Port Specifications

### 3.1 Boundary aggregates (owned by `core/network`, `#[non_exhaustive]`, versioned)

- `HttpRequest` / `HttpResponse` — the two message aggregates.
- Value objects: `Url`, `HeaderMap` / `HeaderName` / `HeaderValue`, `StatusCode`, `Method`, `MediaType` / `Charset`,
  `Body`, `Authority` / `Host` / `Port`, `Scheme`, `RequestTarget` / `Path` / `Query`.
- `NetworkError`, `#[non_exhaustive]`, one `thiserror` enum, every variant carrying a `ProtocolPhase` (`ADR-0011`
  item 4) and a typed defect enum (`UrlDefect`, `FramingDefect`, `MalformedPart`, `RedirectDefect`, `DecodeDefect`,
  `WireLimit`) wherever the cause is a closed set.
- `network::PORT_SCHEMA_VERSION` — the single version knob (`core/network/src/lib.rs`).

### 3.2 `HttpTransport` trait (`network::application::ports`)

```rust
pub trait HttpTransport: Send + Sync {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError>;
}
```

Object-safe with no companion needed — every parameter and return type is a concrete boundary aggregate. Implementations
must be **total**: a hostile or broken peer produces a typed `NetworkError`, never a panic and never an unbounded wait
(every socket read/write is under a timeout, `infrastructure::deadline`).

### 3.3 `RequestPolicy` trait (`network::application::ports`)

```rust
pub trait RequestPolicy: Send + Sync {
    fn decide(&self, request: &HttpRequest) -> Result<PolicyVerdict, NetworkError>;
}
```

`PolicyVerdict` is `Allow | Rewrite(HttpRequest) | Deny { reason: String }`, `#[non_exhaustive]`. **Policy runs before
mechanism**: a consumer calls `decide` before opening any socket, so a `Deny` costs no connection and leaks no DNS query
— this is the seam Phase M's scriptable network policy plugs into.

### 3.4 Threading (`ADR-0019`)

Both traits are synchronous and blocking, by decision of `ADR-0019` — there is no async runtime in this workspace and
the window event loop owns the main thread. A consumer runs `HttpTransport::execute` on a worker of a `std::thread` pool
and receives the `HttpResponse` back over `std::sync::mpsc` as one more event the loop drains
(`alloy/src/application/event_loop.rs`, Phase I4). `RequestPolicy::decide` is pure and cheap enough to run inline on
whichever thread calls it.

### 3.5 Script-driven adapters

A `.rhai` `RequestPolicy` runs through `RuntimeEngine` (`PRD-002`) under `Capability::network_interceptor()`
(`NETWORK_FETCH | FS_WRITE_CACHE`) — never `HttpTransport` itself, which stays a Rust-only mechanism in v0.5. The
binding lives in `core/runtime/rhai-bindings/src/net_bindings.rs` (`NETWORK_BINDINGS`), following the same
self-guarding-per-method pattern as `dom_bindings.rs`. A policy script that panics is trapped
(`rhai-bindings/tests/fault_injection.rs`) and falls back to `AllowAllPolicy` via `run_with_fallback`, exactly as a DOM
script fault falls back to the embedded default DOM.

### 3.6 Reference implementations

`RealHttpTransport` (feature `real-transport`, `rustls` + `ring` + `webpki-roots`) is the shipped mechanism.
`MockTransport` (fixture map, always available) is the reference adapter the conformance suite and every golden e2e test
exercise instead of real sockets. `AllowAllPolicy` is the reference policy.

---

## 4. Requirements & Invariants

1. **Totality**: `execute` and `decide` always return — a peer or a policy cannot make either hang or panic.
2. **No foreign types**: no `rustls`, `TcpStream`, or `webpki` type appears in `application::ports` — those names exist
   only inside `infrastructure`.
3. **Policy-before-mechanism**: no socket opens and no DNS query fires before `RequestPolicy::decide` returns `Allow` or
   `Rewrite`.
4. **Layering** (`ADR-0002` / arch-lint): `core/network` names no `engine`, no `rhai`, no `dom`, no `css`, no
   `graphics`. The scriptable policy adapter lives in `rhai-bindings`.
5. **`unsafe`** (`ADR-0018`): `#![forbid(unsafe_code)]` on this crate without exception; the sole third-party exception
   on this dependency tree is `ring` (the `rustls` `CryptoProvider`), a pre-authorised row-1 carve-out recorded in
   `unsafe-allowlist.toml` and justified in `docs/reports/SPIKE-C0-TLS-PROVIDER.md`.

---

## 5. Acceptance Criteria

- [x] `HttpTransport` and `RequestPolicy` traits defined in `core/network`, both object-safe, no companion needed.
- [x] `network::conformance::run_transport_suite` — totality, boundedness, re-entrancy, self-consistency — passed by
      both `RealHttpTransport` and `MockTransport`.
- [x] `core/network/tests/hostile_responses.rs` pins the hostile-response classes (lying `Content-Length`, bad chunk,
      giant header, redirect cycle) against a real fixture server.
- [x] `cargo test -p network --no-default-features` builds and passes with no TLS stack linked.
- [x] `cargo tree -p network` names no `engine`, `rhai`, `dom`, `css`, or `graphics` (`layering` CI job).
- [x] `NETWORK_BINDINGS` scriptable policy: a script without `NETWORK_FETCH` that calls `fetch` gets
      `EngineError::PermissionDenied`; a script that panics inside a guarded binding falls back to `AllowAllPolicy` and
      the request still resolves (`core/runtime/rhai-bindings/tests/scriptable_network.rs`, v0.5 Phase M).
- [x] `network::PORT_SCHEMA_VERSION` and the message aggregates frozen at integration point `I4` (`alloy::run_browser`,
      `docs/v0-5-handoff/06-i4-alloy-url.md`) — see §6.

---

## 6. Boundary-schema migrations (`network::PORT_SCHEMA_VERSION`)

| Version | Change                                                                                                       | Adapter action |
| ------- | ------------------------------------------------------------------------------------------------------------ | -------------- |
| **1**   | Surface introduced in v0.5 Phase C1; frozen at integration point `I4` (v0.5 Phase I4, `alloy::run_browser`). | —              |
