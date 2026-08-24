# ADR-0007: SPDD Methodology and REASONS Canvas Integration

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-08-22

---

## Context and Problem Statement

To maintain architectural rigor, traceable implementation, and high engineering quality across a modular codebase with
AI assistance, we need a structured process for feature planning and code generation.

---

## Decision Drivers

- Prevent AI hallucination and architectural drift.
- Ensure all code changes are preceded by rigorous domain analysis and structured technical specs.
- Maintain bidirectional traceability between requirements, prompts, and code.

---

## Considered Options

- **Option 1**: Adopt the **SPDD (Structured Prompt-Driven Development)** framework using the **REASONS-Canvas**
  methodology.
- **Option 2**: Ad-hoc conversational prompt-driven code changes.
- **Option 3**: Traditional static waterfall PRD documents without structured prompt integration.

---

## Decision Outcome

Chosen option: **Option 1 (SPDD with REASONS-Canvas)**.

### Pipeline Workflow

1. **Phase 0 (`/spdd-analysis`)**: Ingests requirements from `docs/requirements/` and codebase context to produce
   strategic enriched context files in `spdd/analysis/`.
2. **Phase 1 (`/spdd-reasons-canvas`)**: Generates an implementation-ready, 7-stage REASONS canvas prompt in
   `spdd/prompt/` (Requirements, Entities, Approach, Structure, Operations, Norms, Safeguards).
3. **Phase 2 (`/spdd-generate`)**: Executes the Operations plan to write Rust domain code and test suites.
4. **Phase 3 (`/spdd-sync`)**: Syncs any necessary adjustments back into the prompt file to maintain parity.

### Consequences

- **Positive**:
    - Deterministic, verifiable code generation adhering to domain invariants and ADRs.
    - Transparent documentation of entity models, Mermaid diagrams, and safeguards before coding begins.
- **Negative**:
    - Requires maintaining prompt files in `spdd/prompt/` alongside code changes.
