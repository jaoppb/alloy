# Architecture Decision Records (ADRs)

This directory contains the Architecture Decision Records (ADRs) for Alloy using the
[MADR (Markdown Architectural Decision Records)](https://adr.github.io/madr/) format.

---

## 📜 ADR Index

| ADR                                                                        | Title                                                    | Status   | Date       |
| -------------------------------------------------------------------------- | -------------------------------------------------------- | -------- | ---------- |
| [ADR-0001](0001-record-architecture-decisions.md)                          | Record Architecture Decisions                            | Accepted | 2026-08-22 |
| [ADR-0002](0002-abstract-runtime-engine-and-rhai-backend.md)               | Abstract Runtime Engine and Rhai Backend                 | Accepted | 2026-08-22 |
| [ADR-0003](0003-skeleton-and-muscle-domain-separation.md)                  | Skeleton and Muscle Domain Separation                    | Accepted | 2026-08-22 |
| [ADR-0004](0004-hierarchical-capability-sandboxing-and-fault-isolation.md) | Hierarchical Capability Sandboxing & Fault Isolation     | Accepted | 2026-08-22 |
| [ADR-0005](0005-atomic-hot-reloading-with-stateless-script-swaps.md)       | Atomic Hot-Reloading with Stateless Script Swaps         | Accepted | 2026-08-22 |
| [ADR-0006](0006-cargo-workspace-modular-crate-structure.md)                | Cargo Workspace Modular Crate Structure                  | Accepted | 2026-08-22 |
| [ADR-0007](0007-spdd-methodology-and-reasons-canvas-integration.md)        | SPDD Methodology and REASONS Canvas Integration          | Accepted | 2026-08-22 |
| [ADR-0008](0008-git-hooks-and-code-quality-tooling.md)                     | Git Hooks and Code Quality Tooling                       | Accepted | 2026-08-22 |
| [ADR-0009](0009-vulkan-rendering-with-opengl-fallback.md)                  | Vulkan Rendering with OpenGL and Software Fallback       | Accepted | 2026-08-22 |
| [ADR-0010](0010-clean-architecture-ddd-and-object-calisthenics.md)         | Clean Architecture, DDD, and Object Calisthenics         | Accepted | 2026-08-23 |
| [ADR-0011](0011-replaceable-subsystem-ports-and-conformance-contract.md)   | Replaceable Subsystem Ports and Conformance Contract     | Accepted | 2026-08-29 |
| [ADR-0013](0013-object-safe-runtime-engine-companion.md)                   | Object-Safe `dyn` Companion for the `RuntimeEngine` Port | Accepted | 2026-08-30 |
| [ADR-0014](0014-structured-logging-with-tracing.md)                        | Structured Logging with `tracing`                        | Accepted | 2026-08-30 |
| [ADR-0015](0015-typed-errors-with-thiserror.md)                            | Typed Errors with `thiserror`                            | Accepted | 2026-08-30 |

---

## 🔒 Números reservados

Os planos de implementação em `docs/reports/` reivindicam números de ADR antes de os arquivos existirem, e três
documentos já colidiram em `0014`–`0017`. Esta tabela é o registro de reserva: **consulte-a antes de numerar um ADR
novo**, e adicione a linha aqui no mesmo commit em que o plano reivindica o número.

| Número       | Reservado para                                                            | Origem da reserva                              |
| ------------ | ------------------------------------------------------------------------- | ---------------------------------------------- |
| **ADR-0012** | Escolha do motor de JavaScript de conteúdo (`core/js`)                    | `ADR-0011:128`                                 |
| **ADR-0016** | Unidades fixas (`Au`) e política de determinismo de rasterização          | `IMPLEMENTACAO-DETALHADA-V0-3.md` §2.5         |
| **ADR-0017** | Exceção medida de Object Calisthenics e de lints numéricos no laço quente | `IMPLEMENTACAO-DETALHADA-V0-3.md` §2.10, §2.15 |
| **ADR-0018** | `unsafe` por superfície de ameaça (reescrita de N-02)                     | `IMPLEMENTACAO-DETALHADA-V0-5.md` §2.1         |
| **ADR-0019** | Event loop único                                                          | `IMPLEMENTACAO-DETALHADA-V0-5.md` §2.3         |

> ⚠️ O branch **`docs/benchmark-harness-prd-009`** (`e2f5f1f`, sem PR aberto) tem `0016-…benchmark-harness.md` e
> `0017-…performance-tiers….md` escritos, ambos `Proposed`. Eles colidem com as reservas acima e **renumeram para
> `0020`/`0021` ao rebasear** — a v0.3 tem prioridade de fila por ser a próxima versão.
