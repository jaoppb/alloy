# Spike C0 — TLS provider for `core/network`

- **Date**: 2026-09-04 · **Branch**: `feat/v0-5` · **Phase**: C0 (day-1 de-risk of the F8a network track)
- **Feeds**: `ADR-0018` §"The RustCrypto carve-out", `PRD-009` (Phase P), the C1 SPDD canvas
  (`spdd/analysis/202609040302-[Analysis]-network-transport-and-request-policy-port-v0-5-c1.md`)
- **Toolchain**: `rustc 1.97.1` (pinned, `rust-toolchain.toml`); egress available on this machine — the handshake
  **was** exercised here.

---

## 1. The question

Does a **pure-Rust RustCrypto `rustls::crypto::CryptoProvider`** exist, at a version that

1. pins cleanly against a `rustls` version buildable under toolchain **1.97.1**, and
2. gives **`unsafe`-free crypto** on the attacker-controlled-byte surface (`ADR-0018` row 1)?

If yes → **GO**, C1 uses the RustCrypto provider. If no → the pre-authorised fallback in `ADR-0018` applies: the default
`rustls` provider (`aws-lc-rs` / `ring`) under an explicit, highlighted carve-out.

Settle the rest of the stack too: `webpki-roots` (embedded Mozilla roots — decision already taken: **not** the OS trust
store), and whether the surviving crates are `unsafe`-free per `cargo geiger --forbid-only`.

---

## 2. Candidates tried (throwaway `cargo new`, `rust-version = "1.97.1"`)

| Stack                 | Crates pinned                                                                                                               | Result                                                                                       |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| **RustCrypto**        | `rustls =0.23.43` (`default-features=false`, `std`,`tls12`) + `rustls-rustcrypto =0.0.2-alpha` + `webpki-roots =1.0.9`      | Builds under 1.97.1; handshake OK to all 3 hosts — **but** fails conditions 1 and 2 (below). |
| **`ring` (fallback)** | `rustls =0.23.43` (`default-features=false`, `std`,`tls12`,`ring`) + `webpki-roots =1.0.9` (+ `ring =0.17.14` transitively) | Builds under 1.97.1; handshake OK to all 3 hosts; **clean pin, no duplicates**.              |

### 2.1 Why RustCrypto is NO-GO

- **No stable release.** `rustls-rustcrypto` has only ever published `0.0.0` (yanked), `0.0.1-alpha` (yanked) and
  `0.0.2-alpha` (current). There is no `=x.y.z` to pin — only a pre-release. Upstream
  (`github.com/RustCrypto/rustls-rustcrypto`) has been at `0.0.2-alpha` since 2024. **Condition "pins cleanly": fails.**
- **Forces a duplicate `rustls-webpki`.** `rustls-rustcrypto 0.0.2-alpha` requires `webpki ^0.102.0`; `rustls 0.23.43`
  requires `webpki ^0.103.5`. Both are non-optional, and `0.102`↔`0.103` are semver-incompatible, so the lock file
  carries **`rustls-webpki 0.102.8` _and_ `0.103.15`**. `deny.toml` sets `bans.multiple-versions = "deny"` →
  `cargo deny` would fail. Pinning `rustls` back to an old 0.23.x that still used webpki 0.102 means shipping a
  superseded TLS core — not acceptable. **Condition "pins cleanly": fails.**
- **Not `unsafe`-free.** RustCrypto is _pure-Rust_ (no C, no linked asm) but not _`unsafe`-free_.
  `cargo geiger --forbid-only` on the RustCrypto scratch project flags **~54 crates** as "may use unsafe", including the
  entire crypto core on the byte-decrypt path: `aes 0.8.4` (16 source files with `unsafe` — AES-NI / NEON intrinsics),
  `sha2 0.10.9` (8 — SHA hardware intrinsics), `curve25519-dalek 4.1.3` (6 — SIMD field arithmetic),
  `crypto-bigint 0.5.5`, `num-bigint-dig` (RSA), `chacha20poly1305`, `ghash`, `polyval`, `subtle`, `zeroize`,
  `cpufeatures`, `generic-array`. These are SIMD/intrinsic `unsafe` on exactly the surface `ADR-0018` row 1 forbids
  **and** row 4 (convenience/SIMD) forbids. **Condition "`unsafe`-free crypto": fails.**
- **Surface cost.** RustCrypto pulls **105 crates**; the `ring` stack pulls **26**.

Net: the pure-Rust provider buys no `unsafe` reduction on the hostile-byte surface while multiplying the crate count and
dragging in an alpha dependency and a banned duplicate. The premise `IMPLEMENTACAO-DETALHADA-V0-5.md` §2.1 bet on ("a
pure-Rust RustCrypto provider keeps `unsafe` off the bytes an attacker controls") **does not hold on this toolchain** —
there is no `unsafe`-free TLS stack in the Rust ecosystem today.

---

## 3. Build + handshake result (this machine, `rustc 1.97.1`)

Both scratch projects compiled first try under 1.97.1 (MSRV headroom: `rustls` 1.71, `webpki-roots` 1.70,
`rustls-rustcrypto` 1.75, `ring` ~1.66). A ~40-line `main.rs` built a `ClientConfig` with the provider + `webpki-roots`,
opened a `std::net::TcpStream`, completed the handshake and read the status line:

```text
# ring stack
OK   example.com            HTTP/1.1 200 OK
OK   github.com             HTTP/1.1 200 OK
OK   www.cloudflare.com     HTTP/1.1 200 OK        (all TLS1.3, TLS13_AES_128_GCM_SHA256)

# RustCrypto stack (before it was ruled out on pinning/unsafe grounds)
OK   example.com            HTTP/1.1 200 OK   [suite Some(TLS13_AES_128_GCM_SHA256)]
OK   github.com             HTTP/1.1 200 OK   [suite Some(TLS13_AES_128_GCM_SHA256)]
OK   www.cloudflare.com     HTTP/1.1 200 OK   [suite Some(TLS13_AES_128_GCM_SHA256)]
```

Both are functionally fine. The decision is not about "does it connect" — it is about pinning cleanliness and the
`unsafe` posture.

---

## 4. `cargo geiger --forbid-only` — the surviving (`ring`) stack

Full normal-dependency tree, 26 crates. `:)` = declares `#![forbid(unsafe_code)]`; `?` = may use unsafe.

```text
? tls-spike-ring 0.1.0
:) |-- rustls 0.23.43
? |   |-- once_cell 1.21.4
? |   |-- ring 0.17.14
? |   |   |-- cfg-if 1.0.4
? |   |   |-- getrandom 0.2.17
? |   |   |   |-- cfg-if 1.0.4
? |   |   |   `-- libc 0.2.189
? |   |   `-- untrusted 0.9.0
? |   |-- rustls-pki-types 1.15.1
? |   |   `-- zeroize 1.9.0
? |   |-- rustls-webpki 0.103.15
? |   |   |-- ring 0.17.14
? |   |   |-- rustls-pki-types 1.15.1
? |   |   `-- untrusted 0.9.0
? |   |-- subtle 2.6.1
? |   `-- zeroize 1.9.0
:) `-- webpki-roots 1.0.9
?     `-- rustls-pki-types 1.15.1
```

Reading past the crate-level attribute (grep for `unsafe` in each crate's `src/`):

| Crate                                                 | `?` reason                                                                                                                                                        | On hostile-byte path?            |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- |
| **`ring 0.17.14`**                                    | Hand-written x86-64 / aarch64 **assembly** + a little C (compiled at build with `cc`). The crypto primitives that decrypt TLS records and verify cert signatures. | **Yes** — this is the carve-out. |
| `rustls 0.23.43`                                      | `:)` — geiger reports it as forbidding `unsafe` on our build config (`std`, no `read_buf`).                                                                       | n/a (clean)                      |
| `rustls-webpki 0.103.15`                              | **0** `unsafe` in `src/` — the X.509 / cert-path parser is effectively `unsafe`-free; it just lacks the crate attribute.                                          | Yes, but clean.                  |
| `untrusted 0.9.0`                                     | **0** `unsafe` in `src/` — the DER byte reader is `unsafe`-free.                                                                                                  | Yes, but clean.                  |
| `rustls-pki-types 1.15.1`                             | 1 file (trivial helper).                                                                                                                                          | Marginal.                        |
| `subtle 2.6.1`                                        | 1 file — a constant-time optimisation barrier (`asm!` "black box"). Well-known, benign.                                                                           | Marginal.                        |
| `zeroize`, `once_cell`, `getrandom`, `libc`, `cfg-if` | Ubiquitous ecosystem / platform crates; no hostile parsing.                                                                                                       | No.                              |

So with `ring`, the parts that actually parse attacker bytes — `rustls-webpki` and `untrusted` — carry **no** `unsafe`.
The `unsafe` is confined to `ring`'s vetted primitives (assembly, FIPS lineage, one of the most-audited crates in the
ecosystem) plus two trivial helpers. That is a materially **cleaner** posture than the RustCrypto path, which spreads
intrinsic `unsafe` across `aes` / `sha2` / `curve25519-dalek` / `crypto-bigint`.

---

## 5. `just unsafe-audit` (advisory) after C0

C0 adds the TLS crates to `[workspace.dependencies]` **only** — no crate opts in until C1 — so the crates are not in
`alloy`'s dependency graph yet and `just unsafe-audit` output is **unchanged from the Phase 0 baseline**:

```text
Symbols:
    :) = All entry point .rs files declare #![forbid(unsafe_code)].
    ?  = This crate may use unsafe code.
:) alloy 0.1.0
:) |-- dom 0.1.0
? |   `-- thiserror 2.0.17
...
:) |-- rhai-runtime 0.1.0
? |   |-- rhai 1.26.0
...
:) |-- rhai-bindings 0.1.0
? |-- clap 4.5.51
? |-- thiserror 2.0.17
? `-- tracing 0.1.44
✓ unsafe-allowlist.toml present
```

(`cargo geiger 0.13` still prints its `Failed to match … valuable@0.1.1` / `Scanning done` noise — the known 0.13 bug
recorded in `unsafe-allowlist.toml`; `--forbid-only` output itself is correct.) C1 will re-run this once `core/network`
depends on `rustls`, and the `ring` line must then appear under an allowlisted entry.

---

## 6. Verdict

### NO-GO on the pure-Rust RustCrypto provider

`rustls-rustcrypto` is alpha-only (`0.0.2-alpha`, no stable release since 2024), forces a `deny.toml`-banned duplicate
`rustls-webpki`, and is itself not `unsafe`-free (AES-NI / SHA / dalek intrinsics). It fails two of the three conditions
in §1 outright.

### GO with the pre-authorised `ADR-0018` carve-out — the `ring` provider

**Finalised TLS stack for `core/network` (C1):**

| Crate          | Version (exact pin) | Role                                                                                                            |
| -------------- | ------------------- | --------------------------------------------------------------------------------------------------------------- |
| `rustls`       | `=0.23.43`          | TLS 1.2/1.3 state machine (`default-features = false`, `std`,`tls12`,`ring`); `unsafe`-free itself.             |
| `webpki-roots` | `=1.0.9`            | Embedded Mozilla CA set — **not** the OS trust store (decision confirmed).                                      |
| `ring`         | `=0.17.14`          | `CryptoProvider` primitives via `rustls::crypto::ring::default_provider()`. **The `ADR-0018` row-1 EXCEPTION.** |

Transitive, single-version, no duplicates, staged in `Cargo.lock` when C1 lands: `rustls-webpki 0.103.15`,
`rustls-pki-types 1.15.1`, `untrusted 0.9.0`, `subtle 2.6.1`, `zeroize 1.9.0`, `once_cell 1.21.4`, `getrandom 0.2.17`,
`cfg-if 1.0.4`, `libc 0.2.189`.

`aws-lc-rs` is the other pre-authorised option but has a heavier build (cmake; NASM on Windows) for the 3-OS matrix and
no `unsafe` advantage over `ring`. Pick `ring`; revisit `aws-lc-rs` only if FIPS certification is ever a requirement.

### What the carve-out costs

1. **`unsafe` (assembly + C) on the byte-decrypt / signature-verify surface**, via `ring`. This is precisely what
   `ADR-0018` row 1 forbids for third parties; it is accepted **knowingly and temporarily** under the ADR's "RustCrypto
   carve-out" clause, and must be spelled out in `PRD-009` §threat-model when that file is written in Phase P.
2. **A build-time C toolchain** on all three OSes (`cc` compiles `ring`'s pregenerated asm + C). Standard CI images
   already have it; note it in the C1 CI job and the `PRD-009` non-functional section.
3. **`unsafe-allowlist.toml` gains one row-1 entry** — the first and (for v0.5) only one. Added in this phase:

    ```toml
    [[allow]]
    crate = "ring"
    row = 1  # EXCEPTION — see ADR-0018 §"The RustCrypto carve-out"
    reason = """… pure-Rust rustls-rustcrypto is 0.0.2-alpha (no stable release), forces a
    deny-banned duplicate rustls-webpki, and is not itself unsafe-free (AES-NI / SHA / dalek
    intrinsics), so it buys no safety over `ring` while adding ~80 crates of attack surface.
    `ring` concentrates the assembly `unsafe` in one FIPS-lineage, widely-audited crate.
    Revisit when rustls-rustcrypto ships a stable release pinning current rustls-webpki. …"""
    ```

4. **Follow-up for Phase P** (already an open item in `unsafe-allowlist.toml`'s "Spike finding" note): decide how the
   `unsafe-audit` gate treats the ubiquitous `?` crates the `ring` stack still pulls (`subtle`, `zeroize`, `once_cell`,
   `getrandom`, `rustls-pki-types`, `rustls-webpki`) — per-crate allowlist rows, or a counted baseline rather than a
   zero-match rule. `rustls-webpki` and `untrusted` were checked to contain **no** `unsafe` in `src/`, which should make
   that decision easy.

---

## 7. Notes for C1

- **Provider API.** `rustls::crypto::ring::default_provider()` returns a `CryptoProvider` by value; pass it to
  `ClientConfig::builder_with_provider(Arc::new(provider))`. If C1 ever swaps to `aws-lc-rs`, the call is
  `rustls::crypto::aws_lc_rs::default_provider()` — same shape, different feature flag. Do **not** rely on
  `ClientConfig::builder()` (the no-argument form), which picks a provider from a compile-time default and hides the
  choice.
- **`webpki-roots 1.0.x` API.** `webpki_roots::TLS_SERVER_ROOTS` is `&[TrustAnchor<'static>]`; build the store with
  `RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() }`. (The 0.26.x API differs — pin 1.x.)
- **Handshake is blocking.** Per `ADR-0019`, run it on a `std::thread` pool worker and return the `HttpResponse` over
  `mpsc`; the `HttpTransport` trait stays synchronous. `rustls` is sans-I/O — pair `ClientConnection` with a plain
  `TcpStream` via `StreamOwned`, or drive it manually if C1 wants its own timeout-per-phase loop.
- **No `rustls` type in a port signature** (`PRD-009` item 2 / `ADR-0011`). `ClientConfig`, `ClientConnection`,
  `TrustAnchor` all stay inside `core/network/src/infrastructure/tls.rs`.
- **`deny.toml`** gains `ISC`, `BSD-3-Clause`, `CDLA-Permissive-2.0` in C0 (see that file's comments). No advisory
  ignores needed — `ring 0.17.14` and the `rustls` stack are current and unyanked.
- **Handshake re-verification.** Tested here against `example.com`, `github.com`, `www.cloudflare.com` (all TLS 1.3).
  C1's `run_transport_suite` "real client" arm should re-run against the same set on the maintainer's machine and in the
  3-OS matrix.
