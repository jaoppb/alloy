# PRD-009: Browser Benchmark and Performance Harness

- **Status**: Proposed
- **Author**: Core Architecture Team
- **Date**: 2026-09-02
- **Target Release**: v0.3 (harness + reference lane) · v0.9 (Alloy under test) · v1.0 (published comparison)

---

## 1. Executive Summary

Alloy has fourteen quality gates (`ROADMAP-IMPLEMENTACAO-V1.md` §5) and exactly one performance number in the whole
specification: `<10μs` per event hook (`PRD-001:96`). Nothing measures how the browser behaves as a browser — how fast a
page becomes interactive, how much memory a twenty-tab session costs, how it compares to Chrome or Firefox on the same
hardware, or what machine a user actually needs to run it.

This PRD specifies a **containerized, reproducible benchmark harness** that runs the four industry web benchmarks —
**Speedometer**, **JetStream**, **MotionMark** and **Basemark Web** — plus an Alloy-specific suite, against the current
stable releases of the reference browsers **and** against Alloy itself, under declared hardware tiers and two simulated
user profiles (standard and advanced). Its three outputs are:

1. a **comparative position** of Alloy against the market, honest about what is not yet runnable;
2. the **minimum and recommended system requirements** table published with each release, derived from measurement
   rather than assumption;
3. a **regression gate**: a performance drop between releases fails the nightly build instead of being discovered by
   users.

The harness is the subject of `ADR-0016` (containerization and lane model) and `ADR-0017` (tiers, user profiles and how
a minimum-requirement claim is derived). The container topology, the CLI and the result schemas live in
`docs/architecture/benchmark-harness.md`.

---

## 2. Problem Statement & Motivation

### 2.1 Three questions the project cannot answer today

1. **"Is Alloy fast?"** — There is no measurement of anything user-visible. `criterion` micro-benchmarks are planned for
   `F13` (`ROADMAP-IMPLEMENTACAO-V1.md:270`) and cover the hook overhead only.
2. **"What do I need to run it?"** — `README.md` and the PRDs state no CPU, RAM, GPU or OS requirement. A browser that
   promises a software rasterizer fallback (`ADR-0009`) and a scriptable muscle layer has a genuinely wide hardware
   envelope; guessing at its floor is how projects ship a build that swaps on 8 GiB machines.
3. **"How does it compare?"** — Any comparison published without a fixed methodology, pinned versions and a stated lane
   is marketing, not engineering, and will be dismantled in public the first time someone re-runs it.

### 2.2 Why the four industry benchmarks, and what each one costs

The four suites are not interchangeable, and only two of them are self-hostable:

| Suite             | Measures                                                      | Web features required (probe keys)                                                  | Self-hostable   | Lane          |
| ----------------- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------- | --------------- | ------------- |
| **Speedometer 3** | Responsiveness on real framework workloads; score in runs/min | `es2020`, `dom_l3_events`, `css_flexbox`, `svg`, `canvas2d`, `history_api`, `fetch` | Yes (vendored)  | CI + Lab      |
| **JetStream 2**   | JS/Wasm start-up, throughput and latency; geometric mean      | `es2020`, `wasm_mvp`, `web_workers`, `intl` (subset-dependent)                      | Yes (vendored)  | CI + Lab      |
| **MotionMark**    | Animation complexity sustainable at the display refresh rate  | `canvas2d`, `svg`, `css_animation`, `raf`, real refresh-rate clock                  | Yes (vendored)  | **Lab only**  |
| **Basemark Web**  | Broad HTML5 / CSS / JS / **WebGL** battery, vendor-scored     | all of the above plus `webgl1`                                                      | **No** (hosted) | Lab, external |

Consequences that this PRD accepts explicitly:

- **MotionMark is meaningless under software rasterization and without a real vsync clock.** It is excluded from the
  containerized CI lane and produces publishable numbers only on lab hardware with a pinned refresh rate (`ADR-0016`).
- **Basemark Web 3 is a hosted, proprietary service.** It cannot be vendored into an image, cannot run offline, and its
  terms of service must be reviewed before any comparative result is published (`§10`, V-03). It is a **best-effort,
  manually triggered, never-gating** lane.
- **Speedometer and JetStream are the reproducible core** of the harness: vendored at a pinned commit, served from a
  local mirror container, no external network at run time.

### 2.3 Alloy will lose these benchmarks for years — and that is a measurement, not a failure

At v0.1 Alloy has no DOM, no HTML parser, no CSS, no JavaScript engine and no window: **all four suites are
unrunnable**, and they stay unrunnable until `F10` lands a content JS engine (`ROADMAP-IMPLEMENTACAO-V1.md:267`). When
`boa_engine` does land, it is an interpreter competing against JIT engines — a two-to-three order-of-magnitude gap on
JetStream is the expected, correct result.

A harness that reports this as a score of `0` lies twice: it hides _why_ nothing ran, and it invites a comparison the
architecture never intended to win. Therefore:

- A suite that cannot run reports **`unsupported`** with the exact list of missing feature probes — never a number, and
  never a zero (`§4.4`).
- The unsupported list is a **coverage scoreboard**: the shrinking set of missing features is Alloy's progress metric
  from v0.3 to v1.0, and it is published alongside the scores.
- Alloy's own competitive axis — startup, footprint, hook overhead, hot-reload latency, customization cost — is measured
  by the **Alloy Malleability Suite** (`§7`), which the reference browsers cannot run at all, and which is where the
  project's actual claim of superiority is either earned or refuted.

---

## 3. Goals and Non-Goals

### 3.1 Goals

- One command runs one suite against one subject at one tier, in a container, and emits a signed, complete run manifest.
- Every published number is reproducible from its manifest alone: image digests, benchmark commit, browser build,
  kernel, CPU model, governor, refresh rate, cgroup limits and seed.
- Reference browsers (Chrome, Firefox, WebKitGTK) and Alloy are driven through the **same** runner and the same subject
  port — no bespoke path for the home team.
- Minimum and recommended system requirements are **derived** from tier sweeps plus user-profile budgets (`ADR-0017`),
  and regenerated every release.
- A nightly regression gate catches a performance drop between releases with a statistical test, not with an eyeballed
  percentage.

### 3.2 Non-Goals

- **No CI-lane competitive claims.** Shared-vCPU cloud runners cannot produce publishable cross-browser numbers; the CI
  lane exists to detect _self_-regression only (`ADR-0016`).
- **No vendor-number comparison.** Alloy's containerized results are never compared against scores published by browser
  vendors or press; only against results this harness produced in the same lane, tier and session.
- **No benchmark modification.** Suites run unmodified at a pinned commit. Subsetting is allowed only through the
  suite's own supported mechanism and must be declared in the manifest.
- **No energy, thermal or battery measurement** in v1 of the harness (recorded as future work in `ADR-0017`).
- **No Safari.** Safari does not run in a Linux container and has no Linux build; WebKitGTK is included as an
  engine-family proxy and is labelled as such in every artefact — it is **not** Safari and its numbers are never
  presented as Safari's.

---

## 4. Architecture & Specifications

### 4.1 Two lanes

| Lane    | Where                                         | Purpose                                                   | May publish comparative numbers |
| ------- | --------------------------------------------- | --------------------------------------------------------- | ------------------------------- |
| **CI**  | GitHub-hosted runner, containers, software GL | Self-regression detection, harness smoke, manifest schema | **No**                          |
| **Lab** | Pinned bare-metal host, containers, real GPU  | Published comparisons, tier sweeps, minimum requirements  | Yes, with methodology attached  |

A manifest carries its lane. The publication tooling refuses to render a comparative table from CI-lane data, and
refuses to mix lanes or tiers in a single table (`§8` I-07).

### 4.2 Container topology

Five image families, one `compose` project per run (details and Dockerfile layout in
`docs/architecture/benchmark-harness.md`):

- `bench/mirror` — nginx serving the vendored suites over loopback; removes network variance and external dependency.
- `bench/chrome`, `bench/firefox`, `bench/webkit` — one stable browser each, pinned **by image digest**, driven by
  Playwright inside the image; `--cpuset-cpus`, `--memory` and `--pids-limit` set by the tier.
- `bench/alloy` — the workspace binary built from the commit under test, driven through the Alloy automation port
  (`§4.5`).
- `bench/runner` — the orchestrator; the only container with a writable results volume.

### 4.3 `BenchmarkSubject` port

The runner treats every browser — Alloy included — through one port. Following `ADR-0011`, this is a replaceable seam
with a conformance suite and a mock subject:

```rust
pub trait BenchmarkSubject: Send + Sync {
    fn identity(&self) -> SubjectIdentity;              // engine family, version, build digest
    fn probe(&self) -> Result<FeatureSet, SubjectError>; // §4.4
    fn open(&mut self, target: SuiteUrl) -> Result<Session, SubjectError>;
    fn collect(&mut self, session: &Session) -> Result<RawMetrics, SubjectError>;
    fn resources(&self, session: &Session) -> Result<ResourceSample, SubjectError>;
    fn shutdown(&mut self, session: Session) -> Result<(), SubjectError>;
}
```

No Playwright, CDP or `rhai` type appears in any signature. `SuiteUrl`, `FeatureSet`, `RawMetrics`, `ResourceSample` and
`SubjectIdentity` are `#[non_exhaustive]` value objects owned by the runner's `domain/`, carrying
`BENCH_SCHEMA_VERSION`.

### 4.4 Capability probe — the anti-zero rule

Each suite declares the feature keys it requires (`§2.2`). Before a run, the runner obtains the subject's `FeatureSet`:

- for reference browsers, by executing a probe page in the subject itself;
- for Alloy, by executing the same probe page when it can load one, and before that by reading
  `alloy --benchmark-capabilities`, which prints the declared feature set as JSON derived from the compiled crate
  features.

`required − supported = missing`. If `missing` is non-empty the run terminates with status `unsupported`, records
`missing`, and emits **no score field at all**. A missing score and a zero score are different states in the schema and
render differently in every report.

### 4.5 Alloy automation port

Alloy has no CDP and will not grow one for this. The subject adapter drives it through the existing seams:

- `devtools` exposes a `bench` command channel (introspection protocol of `PRD-001:70`) for navigation, input injection
  and metric readout;
- metrics are emitted as `tracing` spans (`ADR-0014`) exported to a JSON sink, so the same instrumentation serves
  DevTools and the harness;
- the channel is gated by a dedicated capability and is **off** unless the binary is started with the automation flag —
  the sandbox model of `PRD-003` is not weakened for benchmarking.

### 4.6 Statistics

- `n ≥ 10` measured iterations per (suite, subject, tier), plus discarded warm-ups declared in the manifest.
- Reported statistic is the **median with the interquartile range**, never a bare mean.
- Two results differ only if a bootstrap 95% confidence interval of the median difference excludes zero **and** the
  median moves by more than the suite's declared noise floor, calibrated per lane at harness bootstrap (`C-26`).
- The regression gate fires on a statistically significant drop against the previous release's manifest for the same
  lane and tier.

### 4.7 Result artefacts

Every run writes an immutable `run.json` (schema in `docs/architecture/benchmark-harness.md`) plus the suite's raw
output. Manifests are the only input to report generation; reports are regenerable and never hand-edited.

---

## 5. Hardware Tiers

Tiers are enforced with cgroup limits in the CI lane and with real machines plus cgroup limits in the lab lane. They are
the axis along which minimum requirements are derived (`ADR-0017`).

| Tier   | Name       | vCPU | RAM    | Graphics                    | Represents                              |
| ------ | ---------- | ---- | ------ | --------------------------- | --------------------------------------- |
| **T0** | Floor      | 2    | 2 GiB  | Software rasterizer         | Netbook, VM, CI runner, Raspberry-class |
| **T1** | Baseline   | 4    | 8 GiB  | Integrated GPU, OpenGL only | Office laptop ~5 years old              |
| **T2** | Mainstream | 8    | 16 GiB | Modern iGPU with Vulkan     | Current mid-range laptop                |
| **T3** | Enthusiast | 16   | 32 GiB | Discrete GPU with Vulkan    | Developer workstation                   |

T0 exists to find the floor, not to be comfortable. The three graphics rows map onto the fallback cascade of `ADR-0009`,
so the tier sweep doubles as evidence for `C-15`, `C-16` and `C-17`.

---

## 6. User Profiles

Benchmarks measure engines; profiles measure sessions. Both are required — a minimum-requirement claim derived from
Speedometer alone would be indefensible. Each profile is a deterministic, replayable journey over a pinned corpus of
locally mirrored pages, driven at a fixed cadence with a fixed seed.

| Profile                   | Session shape                                                                                           | Primary metrics                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| **Standard** (`padrão`)   | 1 window, ≤5 tabs, article + form + navigation history, 30 min, no user scripts                         | Cold/warm start to interactive, RSS at rest, tab-switch p95, input-to-paint p95, idle CPU         |
| **Advanced** (`avançado`) | 3 windows, ≥20 tabs, DevTools attached, 5 user `.rhai` scripts, 3 hot-reloads, SPA workload, 4 h uptime | All of the above plus hook overhead p99, hot-reload swap p95, RSS growth slope, script-error rate |

The advanced profile is the one that exercises Alloy's differentiators, and the only one that can fail on the hot-reload
and sandbox budgets (`PRD-003`, `PRD-004`).

---

## 7. Alloy Malleability Suite

An in-tree suite measuring what the industry benchmarks do not, and where the project's claim to be _better_ rather than
_different_ has to be settled. Reference browsers report `unsupported` on it, which is the symmetric, honest inverse of
Alloy reporting `unsupported` on Speedometer.

| Metric                    | Definition                                                                      | Budget `[modelado]`     |
| ------------------------- | ------------------------------------------------------------------------------- | ----------------------- |
| Hook overhead             | p99 of one `on_event` round trip through `RuntimeEngine`                        | `< 10μs` (`PRD-001:96`) |
| Hot-reload swap latency   | p95 from file write to first event served by the new AST (after 50 ms debounce) | `< 250 ms`              |
| Hot-reload state survival | DOM + window state retained across 10 consecutive reloads (`C-13`)              | 100%                    |
| Cold start to interactive | Process spawn to first input accepted, T1                                       | `< 1.5 s`               |
| Footprint at rest         | RSS, 1 blank tab, T1                                                            | `< 250 MiB`             |
| Customization cost        | Score delta on the standard profile with 0 vs. 5 user scripts loaded            | `< 5%`                  |

All six are `[modelado]` targets. The first calibrated run replaces each of them with a measured baseline, exactly as
`ROADMAP-IMPLEMENTACAO-V1.md` §5 prescribes for its own gates.

---

## 8. Requirements & Invariants

1. **I-01 Reproducibility**: a manifest plus the repository is sufficient to re-run a measurement; every version in the
   stack is pinned by digest or commit SHA.
2. **I-02 Isolation**: at run time the subject container reaches the mirror over loopback and nothing else; external
   egress is disabled except in the declared Basemark lane.
3. **I-03 No fabricated numbers**: `unsupported` and `error` are terminal statuses that carry no score field; report
   generators must not coerce them to `0`.
4. **I-04 Same path for Alloy**: the Alloy subject uses the same runner, statistics and schema as the reference
   browsers; no suite-specific special case may exist for it.
5. **I-05 Sandbox integrity**: the automation channel is capability-gated and disabled by default; benchmarking must not
   require weakening `PRD-003`.
6. **I-06 No workspace contamination**: the harness is a separate `bench/` tree; no `core/*` crate gains a dependency on
   it, and the `no-engine` and `arch-lint` gates keep passing unchanged.
7. **I-07 Lane discipline**: comparative tables refuse to mix lanes, tiers or sessions, and CI-lane data is never
   published as a comparison.
8. **I-08 Licence compliance**: each vendored suite keeps its upstream licence file and attribution; a suite whose terms
   forbid published comparison is run manually and reported internally only.
9. **I-09 Contract compliance**: `BenchmarkSubject` satisfies all seven items of `ADR-0011`, including a mock subject
   and a `bench-conformance` target.

---

## 9. Acceptance Criteria

Numbering continues the roadmap's criteria list, which ends at `C-18` (`ROADMAP-IMPLEMENTACAO-V1.md` §2).

- [ ] **C-19** `just bench <suite> <subject> <tier>` runs Speedometer and JetStream against Chrome, Firefox and
      WebKitGTK in containers and writes a schema-valid `run.json` for each.
- [ ] **C-20** Suites are served by the local mirror; a run completes with external egress blocked (Basemark excluded
      and declared).
- [ ] **C-21** A subject missing required features terminates with `unsupported` plus the missing-feature list, emits no
      score field, and Alloy at the current commit produces exactly such a manifest for all four suites.
- [ ] **C-22** The tier sweep T0–T3 runs with cgroup limits verified from inside the container, and a T0 vs. T3 delta is
      visible on the same suite and subject.
- [ ] **C-23** The standard and advanced profiles replay deterministically: 10 repetitions on one host give an IQR
      within the calibrated noise floor for every primary metric.
- [ ] **C-24** `docs/reports/` gains a generated minimum-and-recommended-requirements table with the tier evidence and
      the budget that each tier failed or met.
- [ ] **C-25** The nightly workflow fails on a statistically significant regression against the previous release
      manifest in the same lane and tier, and passes on noise.
- [ ] **C-26** Lab-lane calibration is published: 10 Speedometer runs on Chrome on the pinned host, with the resulting
      noise floor recorded as the comparison threshold.
- [ ] **C-27** The Alloy Malleability Suite runs on every release and its six metrics are tracked over time; the
      reference browsers report `unsupported` on it.
- [ ] **C-28** A mock `BenchmarkSubject` passes the `bench-conformance` suite and drives an end-to-end run with no real
      browser present.

---

## 10. Open Verification Items

These are unverified at authoring time and must be checked before the corresponding work starts. Each one can change
scope.

- **V-01** Exact pinned versions and licence text of Speedometer, JetStream and MotionMark at bootstrap; confirm
  redistribution terms for vendoring into an image.
- **V-02** Whether the pinned MotionMark version's score is stable under a fixed software-rendered refresh rate — if it
  is, a non-publishable CI lane for it becomes possible after all.
- **V-03** Basemark Web 3 terms of service regarding automated execution and publication of comparative results.
- **V-04** Playwright's browser matrix and image digests for the three reference browsers on the pinned base image.
- **V-05** Whether GPU passthrough (`/dev/dri`) is available on the intended lab host, and its effect on tier T1–T3
  fidelity.
- **V-06** Whether `boa_engine` supports enough of the JetStream subset to produce a non-`unsupported` result at v0.7,
  and which subtests are excluded by missing Wasm support.
