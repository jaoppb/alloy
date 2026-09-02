# ADR-0017: Performance Tiers, User Profiles and Minimum System Requirements

- **Status**: Proposed
- **Deciders**: Architecture Team
- **Date**: 2026-09-02

---

## Context and Problem Statement

`ADR-0016` decides how Alloy is measured. It does not decide what may be _claimed_ from a measurement. `PRD-009`
requires two claims that are easy to make badly:

- **"Alloy needs at least X."** A minimum-requirements table is a promise to users. Derived from a benchmark score it is
  meaningless — Speedometer says nothing about whether a 2-core, 2 GiB machine can hold twenty tabs open for four hours.
- **"Alloy is better than the others."** Against Chrome or Firefox on Speedometer, JetStream or MotionMark, Alloy will
  lose for years and by wide margins; it is an interpreter-backed browser being built from scratch. A claim of
  superiority stated on that ground is false, and stating it would discredit every other number the project publishes.

What evidence turns a measurement into a published requirement, and on which axis may Alloy claim to be better?

---

## Decision Drivers

- A requirement claim must name the workload it was verified against, not just the hardware.
- The failure that defines a floor is a **budget violation in a realistic session**, not a low score.
- The project's differentiators — malleability, hot reload, footprint, sandbox — are measurable and are where a
  comparison is actually meaningful.
- `PRD-001:96` already fixes one budget (`<10μs` per hook); the rest must be declared, then calibrated, never invented
  retroactively to match whatever was measured.
- Users of an experimental browser need the floor to be honest, including its consequences ("at T0, expect ...").

---

## Considered Options

- **Option 1**: **Tier sweep plus profile budgets** — four hardware tiers (T0–T3) crossed with two deterministic user
  profiles (standard, advanced); the minimum tier is the lowest one that meets every budget of the standard profile, the
  recommended tier the lowest that meets every budget of the advanced profile.
- **Option 2**: **Benchmark-score thresholds** — declare a minimum as the hardware where Speedometer or JetStream
  reaches some score.
- **Option 3**: **Copy the competition** — publish requirements derived from what Chrome and Firefox state.
- **Option 4**: **Publish no requirements** until v1.0.

---

## Decision Outcome

Chosen option: **Option 1 (tier sweep plus profile budgets)**, with an explicit rule about what may be compared.

Option 2 measures the engine, not the session, and would certify a machine that cannot actually hold a working session.
Option 3 inherits assumptions from architectures Alloy does not share (multi-process, JIT, GPU compositing) and is
unverifiable for us. Option 4 leaves users to discover the floor by suffering it, and leaves the project without the
regression signal the tier sweep also produces.

### How a requirement is derived

1. Run the standard and advanced profiles at every tier T0–T3 (`PRD-009` §5–§6), `n ≥ 10`, median with IQR.
2. Evaluate each profile's budgets. A tier **fails** a profile if any primary metric misses its budget outside the
   calibrated noise floor.
3. **Minimum requirement** = lowest tier passing all standard-profile budgets. **Recommended** = lowest tier passing all
   advanced-profile budgets.
4. Publish the table with the evidence: which budget failed at the tier below, by how much, and with which manifest ids.
5. Every requirement claim names the graphics tier of `ADR-0009` it assumed (software, OpenGL, Vulkan). A minimum stated
   without its rendering path is not a minimum.
6. The table is regenerated per release. A regression that moves the floor upward is a release-blocking discussion, not
   a silent edit to a documentation table.

### Initial budgets (all `[modelado]`)

These are proposed targets, not measurements. The first calibrated run on the lab host replaces each with a baseline,
following the same convention as `ROADMAP-IMPLEMENTACAO-V1.md` §5.

| Budget                             | Standard profile | Advanced profile | Source                     |
| ---------------------------------- | ---------------- | ---------------- | -------------------------- |
| Cold start to first input accepted | `< 1.5 s`        | `< 2.5 s`        | `[modelado]`               |
| Input-to-paint p95                 | `< 100 ms`       | `< 150 ms`       | `[modelado]`               |
| Tab-switch p95                     | `< 150 ms`       | `< 300 ms`       | `[modelado]`               |
| RSS at rest                        | `< 250 MiB`      | `< 1.5 GiB`      | `[modelado]`               |
| RSS growth over the session        | `< 5% / h`       | `< 5% / h`       | `[modelado]`, leak signal  |
| Hook overhead p99                  | `< 10μs`         | `< 10μs`         | `PRD-001:96`               |
| Hot-reload swap p95                | n/a              | `< 250 ms`       | `[modelado]`, `PRD-004:42` |
| Dropped-frame rate while scrolling | `< 5%`           | `< 10%`          | `[modelado]`               |

### What Alloy may claim

- **Permitted**: superiority on the axes of the Alloy Malleability Suite (`PRD-009` §7) — hot-reload latency and state
  survival, hook overhead, footprint at rest, cold start, customization cost — measured in the lab lane, at a stated
  tier, against the same version of the harness.
- **Permitted**: an honest gap report on the industry suites — the score, the lane, the tier, and the list of features
  still missing.
- **Forbidden**: any comparative claim from CI-lane data, any comparison against numbers this harness did not produce,
  any aggregate "faster than X" statement, and any score for a suite that reported `unsupported`.
- **Forbidden**: quoting a WebKitGTK result as Safari.

### Consequences

- **Positive**:
    - The minimum-requirements table becomes falsifiable evidence with named failing budgets.
    - The same sweep doubles as regression signal per tier and as evidence for the graphics fallback criteria `C-15`,
      `C-16` and `C-17`.
    - The project gets a defensible competitive story — malleability metrics the competition cannot even run — instead
      of an indefensible one.
    - Budgets stated up front cannot be retrofitted to flatter a result.
- **Negative**:
    - Eight tier×profile runs per release is real machine time, and the advanced profile alone is a four-hour session.
    - Budgets are guesses until the first calibration, and the first calibration will move several of them.
    - Profile corpora are frozen page sets that age; refreshing them re-baselines the trend lines.
    - The publication rules constrain marketing, deliberately, and that constraint has to survive contact with a release
      announcement.
