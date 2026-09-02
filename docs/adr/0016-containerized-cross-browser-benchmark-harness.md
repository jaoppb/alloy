# ADR-0016: Containerized Cross-Browser Benchmark Harness

- **Status**: Proposed
- **Deciders**: Architecture Team
- **Date**: 2026-09-02

---

## Context and Problem Statement

`PRD-009` requires Alloy to be measured against the current stable releases of the reference browsers on the four
industry web benchmarks (Speedometer, JetStream, MotionMark, Basemark Web), and requires those measurements to justify a
published minimum-system-requirements table. The measurement has three hostile properties:

1. **The subjects are moving targets.** Chrome and Firefox auto-update roughly monthly; a number measured against
   "Chrome stable" without a build digest is unreproducible within weeks.
2. **The host is a variable.** CPU model, frequency governor, thermal state, SMT, background load, GPU driver and
   display refresh rate move benchmark scores by more than the differences the harness is supposed to detect.
3. **The subjects are heavy and mutually hostile.** Three browser stacks, their system libraries and a driver framework
   do not coexist cleanly on a developer machine, and installing them there makes the measurement machine-specific.

How should Alloy execute cross-browser benchmarks so that results are reproducible, comparable, and safe to publish —
and so that they degrade honestly when the environment cannot support the claim?

---

## Decision Drivers

- A result must be re-derivable from its manifest alone (`PRD-009` I-01).
- Alloy must be measured through the same path as its competitors — no privileged home-team route (`PRD-009` I-04).
- The measurement must not require weakening the capability sandbox of `PRD-003`.
- The harness must not leak into the workspace: `core/*` crates keep their dependency graph and their `no-engine` and
  `arch-lint` gates (`PRD-009` I-06).
- Honest degradation: an environment that cannot support a claim must invalidate the claim, not quietly produce a
  number.
- Cost: the reproducible core must run on ordinary CI hardware, even if publishable numbers cannot.

---

## Considered Options

- **Option 1**: **Containerized subjects with a two-lane model** — every browser and Alloy in its own OCI image pinned
  by digest, suites served by a local mirror container, cgroup-enforced tiers, and a hard split between a
  non-publishable CI lane and a publishable lab lane on pinned bare metal.
- **Option 2**: **Bare-metal only** — install the browsers on a dedicated physical machine and script them directly.
- **Option 3**: **Vendor-published numbers** — compare Alloy's own measurements against scores published by browser
  vendors and the technical press.
- **Option 4**: **Full VM images per subject** (QEMU/KVM) instead of containers.

---

## Decision Outcome

Chosen option: **Option 1 (containerized subjects, two lanes)**.

Option 2 gives the best signal but no reproducibility for anyone without that exact machine, and no way to run the
harness in CI at all; it survives as the **lab lane inside** Option 1, which is where publishable numbers come from.
Option 3 is rejected outright: comparing our measurement against someone else's methodology, hardware and browser build
is the classic benchmarking fallacy, and would be indefensible the moment it were checked. Option 4 buys stronger
isolation at a cost — virtualized GPU and timer paths — that damages exactly the graphics and latency signals the
harness exists to capture.

### What the decision fixes

1. **One image per subject, pinned by digest.** Browser images pin the browser build; suite images vendor Speedometer,
   JetStream and MotionMark at a commit SHA. The Alloy image is built from the commit under test. No image resolves a
   floating tag at run time.
2. **A local mirror.** Suites are served over loopback by an nginx container. External egress is disabled during a run,
   removing network variance and third-party availability from the measurement.
3. **Tiers as cgroup limits.** `--cpuset-cpus`, `--memory` and `--pids-limit` implement T0–T3 of `PRD-009` §5. The
   runner verifies the limits from inside the container and records them in the manifest.
4. **Two lanes, never mixed.**
    - **CI lane** — GitHub-hosted runners, software rendering, shared vCPUs. Detects Alloy's _self_-regression against
      its own previous manifests. **Never** produces a cross-browser claim, and the report generator refuses to render
      one from CI data.
    - **Lab lane** — pinned bare-metal host, performance governor, SMT and turbo settings recorded, real GPU, fixed
      display refresh rate. The only source of published comparisons and of minimum-requirement claims.
5. **MotionMark is lab-only** and Basemark Web is a manual, external, non-gating lane, because a hosted proprietary
   service can be neither vendored nor made deterministic (`PRD-009` §2.2).
6. **WebKitGTK is not Safari.** It is included as an engine-family proxy and is labelled as a proxy in every manifest
   and every rendered table. Safari has no Linux build and is out of scope.
7. **A `BenchmarkSubject` port under `ADR-0011`.** Alloy is one implementation among four, with a mock subject and a
   `bench-conformance` target, so the harness cannot special-case the home team without failing its own contract.
8. **The harness lives in `bench/`,** outside `core/*`. Its orchestrator is a Rust crate for consistency with the
   workspace's tooling and typed-error rules (`ADR-0015`); the browser driver inside the reference images is Playwright,
   which is the only mature way to drive three browser families uniformly and is already matched by the repository's
   existing pnpm toolchain.

### Consequences

- **Positive**:
    - A published number is falsifiable: manifest, digests, commit SHAs and host facts travel with it.
    - The CI lane gives per-night regression signal on hardware nobody has to own.
    - Adding a fifth subject (a future engine, a fork, another Alloy backend) is an image plus a port implementation.
    - Container limits make the tier sweep — and therefore the minimum-requirements derivation of `ADR-0017` — cheap and
      repeatable.
    - The sandbox is untouched: automation is a capability-gated `devtools` channel, off by default.
- **Negative**:
    - Two lanes mean two sets of rules and a standing discipline problem: the temptation to quote CI numbers is real, so
      the prohibition is enforced in the tooling, not in a convention.
    - Containers add measurable variance versus bare metal; the noise floor must be calibrated per lane before any
      threshold means anything (`PRD-009` C-26).
    - Image maintenance is recurring work: pinned browser digests go stale, and refreshing them re-baselines every trend
      line.
    - GPU access from containers is host-specific (`/dev/dri` passthrough), so graphics tiers T1–T3 are only fully
      faithful on the lab host.
    - The lab host is a single point of failure for publishable results, and its retirement invalidates comparability
      across the boundary.
