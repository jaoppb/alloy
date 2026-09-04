# ADR-0018: `unsafe` Governed by Threat Surface (rewrite of N-02)

- **Status**: Proposed (drafted in v0.5 Phase 0; finalised in Phase P alongside the `unsafe-audit` gate)
- **Deciders**: Architecture Team
- **Date**: 2026-09-03

---

## Context and Problem Statement

`PRD-001:97` (roadmap label **N-02**) reads _"Memory Safety: **Zero unsafe memory operations exposed to script
runtimes**."_ Taken literally this has **never been true and has never been checked**:

- The pinned `rhai =1.26.0` contains four `unsafe` blocks, **three of them on the binding registration / native-call
  path** — the seam every guarded binding crosses (`src/reify.rs`, `src/func/register.rs:60`, `src/func/call.rs:87`, all
  `transmute_copy` behind a `TypeId` check). `rhai::func::call` **is** the script runtime; `func/register.rs` is the
  path `register_guarded_binding` uses. So N-02 has been violated since v0.1, by the engine choice of `ADR-0002` — see
  `docs/reports/VIOLACAO-N02-UNSAFE-NO-RHAI.md`.
- v0.5 needs `winit` + `softbuffer` for a native window (OS FFI is `unsafe` by construction) and a TLS stack for
  `core/network`. The requester chose a pure-Rust RustCrypto `CryptoProvider` for `rustls` specifically to keep `unsafe`
  off the bytes an attacker controls.

Rejecting `ring` for bringing `unsafe` while dispatching every binding call through `transmute_copy` is not rigour, it
is inconsistency. We need one honest rule, and a gate that enforces it.

`#![forbid(unsafe_code)]` stays on **every hand-written crate in this workspace**, without exception. This ADR governs
the **dependency tree**, which `cargo-deny` does not inspect (it audits CVEs and licences, not `unsafe`).

---

## Decision Drivers

- The rule must describe what the project can actually hold to, and it must be mechanically checkable.
- Attacker-controlled bytes (TLS records, HTTP, HTML, CSS, PNG, fonts) are the surface N-02 exists to protect — an
  overflow there is executed by any web page.
- Platform FFI (window, surface, event loop) has no `unsafe`-free alternative that keeps the 3-OS matrix (`breadx` is
  X11-only; `x11rb` / `wayland-client` still carry FFI `unsafe`). It processes no hostile input; the OS is already
  presupposed trust.
- The trusted muscle script is modelled as **buggy, not adversarial** (`PRD-003:21-24`) — the author is the user.
- Convenience `unsafe` (SIMD, custom allocators) was already the criterion v0.3 used to reject `simd-adler32`.

---

## Considered Options

- **Option 1 — "zero third-party `unsafe`, no exceptions".** Honest only if the muscle engine is replaced, which
  contradicts `ADR-0002`. Also forces v0.5 headless (no `winit`), giving up the roadmap's "first presentable version"
  (`ROADMAP:218`). It does **not** buy an `unsafe`-free tree while `rhai` is the backend.
- **Option 2 — leave N-02 as written, keep ignoring it.** A requirement that is silently false corrodes the other four
  NFRs.
- **Option 3 — govern `unsafe` by threat surface, with a reviewed allowlist and a CI gate.** Describes what the project
  already does, makes each exception explicit and revisable.

---

## Decision Outcome

Chosen option: **Option 3.** `PRD-001:97` is **rewritten** in Phase P to the rule below; this ADR is its rationale.

### The rule

| Surface                                                             | Third-party `unsafe`   | Why                                                                                                                                                                                   |
| ------------------------------------------------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Attacker-controlled bytes — TLS, HTTP, HTML, CSS, PNG, font parsing | **Forbidden**          | The surface N-02 exists to protect; an overflow here is executed by any page.                                                                                                         |
| Trusted muscle script (Rhai binding dispatch)                       | Permitted, **nominal** | `PRD-003:21-24` models the muscle script as buggy, not adversarial; the author is the user. Time-limited: in v0.7 `core/js` runs adversarial script and its engine falls under row 1. |
| Platform FFI with no alternative — window, surface, event loop      | Permitted, **nominal** | No hostile bytes; the boundary with the OS, which is already trusted.                                                                                                                 |
| Convenience — SIMD, allocation, micro-optimisation                  | **Forbidden**          | The criterion v0.3 used to reject `simd-adler32`.                                                                                                                                     |

### The gate

A **blocking CI job `unsafe-audit`** runs `cargo-geiger` over the workspace and fails if `unsafe` appears in any crate
outside `unsafe-allowlist.toml` — a nominal, commented allowlist that is the reviewable record of rows 2 and 3. It is
seeded in Phase 0 with `rhai` (comment citing `PRD-003:21-24`), grows **only by review**, and **must never** contain a
crate that decodes bytes from the network (a test asserts this — classifying a byte decoder as "platform FFI" to pass
the gate fails the build).

### The RustCrypto carve-out

The v0.5 network track opens with a day-1 spike (Phase C0): does a pure-Rust RustCrypto `CryptoProvider` pin cleanly
against `rustls` under toolchain 1.97.1? If **not**, the pre-authorised fallback is the default `rustls` provider
(`aws-lc-rs` / `ring`) under an **explicit exception recorded here and in PRD-009** — assembly `unsafe` on the
byte-decrypt surface, accepted knowingly and temporarily, revisited when a RustCrypto provider is viable.

### Consequences

- **Positive**: the `unsafe` posture is now written, enforced, and matches reality. New TLS / window dependencies show
  up as an allowlist diff in review, not a surprise. The v0.7 content-JS engine choice (`ADR-0012`) is born knowing its
  `unsafe` criterion is the strict one (row 1).
- **Negative**: an allowlist to maintain, and a `cargo-geiger` toolchain-compat risk (spiked in Phase 0). N-02 must be
  rewritten in the same delivery — a half-changed requirement is worse than either state.
