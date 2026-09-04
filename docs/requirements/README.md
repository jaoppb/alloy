# Product Requirements Documents (PRDs)

`docs/requirements/PRD-*.md` are the authoritative feature inputs for the SPDD workflow (ADR-0007). Each replaceable
port also has a **seam PRD** naming the variation model and threat model (ADR-0011 item 1).

---

## 📜 PRD Index

| PRD                                                         | Title                                      | Status   | Target |
| ----------------------------------------------------------- | ------------------------------------------ | -------- | ------ |
| [PRD-001](PRD-001-alloy-core-system.md)                     | Alloy Core System                          | Accepted | v0.1   |
| [PRD-002](PRD-002-abstract-runtime-engine.md)               | Abstract Runtime Engine Port               | Accepted | v0.1   |
| [PRD-003](PRD-003-script-isolation-and-sandboxing.md)       | Script Isolation and Sandboxing            | Accepted | v0.2   |
| [PRD-004](PRD-004-hot-reload-subsystem.md)                  | Hot-Reload Subsystem                       | Proposed | v0.9   |
| [PRD-005](PRD-005-graphics-and-gpu-rendering.md)            | Graphics and GPU Rendering (RenderBackend) | Accepted | v0.3   |
| [PRD-006](PRD-006-web-content-javascript-runtime-port.md)   | Web-Content JavaScript Runtime Port        | Proposed | v0.7   |
| [PRD-007](PRD-007-style-cascade-and-layout-engine-ports.md) | Style Cascade and Layout Engine Ports      | Proposed | v0.5   |
| [PRD-008](PRD-008-html-tokenizer-and-tree-sink-ports.md)    | HTML Tokenizer and Tree Sink Ports         | Proposed | v0.5   |

---

## 🔒 Números reservados

Como os ADRs (`docs/adr/README.md`), os planos de implementação em `docs/reports/` reivindicam números de PRD antes de
os arquivos existirem, e um branch não mergeado já colidiu em `PRD-009`. Esta tabela é o registro de reserva:
**consulte-a antes de numerar um PRD novo**, e adicione a linha aqui no mesmo commit em que o plano reivindica o número.

| Número      | Reservado para                                                     | Origem da reserva                       |
| ----------- | ------------------------------------------------------------------ | --------------------------------------- |
| **PRD-009** | Porta de transporte HTTP + política de requisição (`core/network`) | `IMPLEMENTACAO-DETALHADA-V0-5.md` §2.2  |
| **PRD-010** | Porta de sistema de janela + apresentador (`core/window`)          | `IMPLEMENTACAO-DETALHADA-V0-5.md` §2.2  |
| **PRD-011** | Harness de benchmark web em container                              | branch `docs/benchmark-harness-prd-009` |

> ⚠️ O branch **`docs/benchmark-harness-prd-009`** (`e2f5f1f`, sem PR aberto) tem um arquivo `PRD-009` escrito para o
> harness de benchmark. Ele **colide** com a reserva acima: a v0.5 tem prioridade de fila (é a próxima versão), então o
> arquivo de rede fica com `PRD-009`, o de janela com `PRD-010`, e o harness de benchmark **renumera para `PRD-011` ao
> rebasear**. O harness deve referenciar o portão `hook-benchmark` da Fase P da v0.5 em vez de duplicar o critério de
> `<10μs`.
