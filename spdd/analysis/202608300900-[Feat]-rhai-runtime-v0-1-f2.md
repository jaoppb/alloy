# SPDD Analysis — v0.1 F2 (`core/runtime/rhai`) + the `alloy` binary

> Phase 0 artefact for the second half of v0.1 "O engine vive". Builds on
> `spdd/prompt/202608291200-[Feat]-engine-v0-1-f0-f1.md` (F0 + F1, already landed). Consolidated from `PRD-002`,
> `PRD-003`, `ADR-0002`, `ADR-0003`, `ADR-0004`, `ADR-0005`, `ADR-0011`, and
> `docs/reports/IMPLEMENTACAO-DETALHADA-V0-1.md` §3 (F2 steps) as amended.

## 1. State inherited from F1

`core/engine` is implemented: `RuntimeEngine` / `ExecutionContext` (object-safe core + PRD-002 provided sugar),
`EngineValue`, one `EngineError`, `Capability` / `CapabilitySet`, `ExecutionLimits`, `EngineType`, `EngineFunction`, and
the public `engine::conformance` suite. `MockEngine` (in `core/engine/tests/`) closes C-01 and C-05. During F2
integration one F1 signature was tightened: `ExecutionContext::register_native_fn` gained an `Arity` parameter so an
adapter can reserve a fixed-shape binding.

## 2. What F2 adds

| #    | Criterion (`PRD-002`)                                           | Closed by                                                                                                                                    |
| ---- | --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| C-02 | `RhaiEngine` passes trait-compliance tests                      | `core/runtime/rhai/tests/conformance.rs` runs `engine::conformance::run_core_suite`; `fixture_node.rs` covers the "registered struct" clause |
| C-04 | Infinite loop aborts with `EngineError::ExecutionLimitExceeded` | `execution_limits.rs` — operation ceiling **and** wall-clock ceiling                                                                         |

Also delivered (v0.1 micro-deliverables, `ROADMAP §3.1`): the `alloy` binary (`cargo run -p alloy` opens/exits 0;
`alloy --script <path>` compiles + runs a Rhai script under the sandbox and prints the result).

Still open after v0.1: C-03 (needs the real `core/dom` `DomNode`, v0.2 I1), C-06 … C-13 (later phases). C-09 has its
_mechanism_ here (`ScriptPanic` trapping); the DevTools-logging fallback handler is F6/v0.2.

## 3. Design decisions

1. **`rhai-runtime` is the only crate that names a `rhai` type.** `#[forbid(unsafe_code)]`; everything `rhai::*` lives
   under `infrastructure/`.
2. **One `rhai::Engine` per `RhaiContext`.** Registrations (native fns, custom types) and scope are per-context → real
   isolation (PRD-003:78, C-08). The `RhaiEngine` keeps a separate compiler engine for `compile()`.
3. **`CompiledScript = RhaiCompiledScript(Arc<rhai::AST>)`** — the exact shape F11 hot-reload swaps (ADR-0005).
   `eval_compiled_value` clones the `Arc`.
4. **Limits** (v0.1 report §3 F2 step 4): `set_max_operations` / `set_max_call_levels` / `set_max_expr_depths` from
   `ExecutionLimits`, plus a wall-clock `on_progress` guard reading an `Arc<Mutex<Option<Instant>>>` armed per
   evaluation. `ErrorTooManyOperations` → `Operations`, `ErrorStackOverflow` → `CallDepth`, our `ErrorTerminated` →
   `Duration`.
5. **Fault trapping**: `std::panic::catch_unwind(AssertUnwindSafe(…))` around `eval_ast_with_scope`; a caught panic →
   `EngineError::ScriptPanic`. No `panic = "abort"` in any profile (would break this). `AssertUnwindSafe` is safe API,
   so `#[forbid(unsafe_code)]` holds.
6. **Marshaling** (`marshal.rs`): `EngineValue` ⇄ `rhai::Dynamic` by `match` / typed accessors. Workspace pins `rhai`
   features so `INT = i64`, `FLOAT = f64` — no 64-bit truncation.
7. **`EngineType` bridge**: `RhaiContext::register_custom_type::<T: EngineType + rhai::CustomType>()` is an _adapter
   extension_ (the `rhai::CustomType` bound only appears in `rhai-runtime`). The port's `register_type_erased` records
   the name only; the generic capability-guarded registrar is v0.2 (I1).
8. **`call_function_value`** invokes a registered _native_ binding directly from Rust (kept in a `HashMap` on the
   context). Calling a _script-defined_ `fn` from Rust needs the AST-retention machinery of F11 and is out of scope.
9. **`alloy` CLI**: hand-rolled arg parsing (`--script`, `-h`, `-V`), no dependency (v0.1 report decision 2.8). Added as
   an explicit workspace member — the `core/runtime/*` glob is untouched (`ADR-0006:59-61`).

## 4. Supply chain — verified with `cargo deny 0.20`

`rhai 1.26` adds a transitive tree (`ahash`, `smallvec`, `smartstring`, `num-traits`, `rhai_codegen` →
`syn`/`quote`/`proc-macro2`, …). `bitflags` stays at one version (`=2.13.1`, shared with `engine`) → no
`multiple-versions` breach. `cargo deny check` was run and is green (`advisories ok, bans ok, licenses ok, sources ok`)
after three adjustments to `deny.toml`:

- Licenses `MPL-2.0` (`smartstring`) and `CC0-1.0` (`tiny-keccak` via `ahash`) added; the allow-list was then trimmed to
  exactly the encountered set.
- `RUSTSEC-2026-0249` **ignored with a written rationale**: `smartstring` was archived by its author 2026-05-03 — an
  _unmaintained_ notice, not a vulnerability, transitive via `rhai`, with no upgrade path until `rhai` migrates off it.
  Re-review on every `rhai` bump.
- `private = { ignore = true }` (the workspace crates carry no `license` key — no LICENSE file in the repo yet — and are
  all `publish = false`); `allow-wildcard-paths = true` (internal `path` deps read as wildcards).

The nine stub-crate manifests were aligned to `*.workspace = true` inheritance so `publish = false` reaches them.

## 5. Out of scope (unchanged from F1 analysis)

Per-binding capability enforcement (F6); hot-reload watcher (F11); `core/dom` (v0.2); `criterion` hook benchmark (v0.5);
the DevTools fallback handler (F6).
