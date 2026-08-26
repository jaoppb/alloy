# SPDD Analysis: 000 - F0 Foundation & Tooling Bootstrap

| Campo           | Valor                                                             |
| :-------------- | :---------------------------------------------------------------- |
| **Status**      | Concluído (Bootstrap Exception)                                   |
| **Data**        | 2026-08-26                                                        |
| **Fase**        | F0 (Fundação)                                                     |
| **Responsável** | Architecture & Core Team                                          |
| **Origem**      | ROADMAP-IMPLEMENTACAO-V1 / ADR-0006, ADR-0007, ADR-0008, ADR-0010 |

---

## 1. Contexto e Motivação

A Fase F0 estabelece a infraestrutura técnica indispensável para viabilizar qualquer desenvolvimento em Rust no
workspace do Alloy:

- Fixação da toolchain (`rust-toolchain.toml`) e do MSRV (1.85.0).
- Instalação e execução de `rustfmt` e `clippy` nos hooks locais e no CI.
- Versionamento do `Cargo.lock` e centralização de dependências (`[workspace.package]` e `[workspace.dependencies]`).
- Criação do crate binário executável `alloy/` (com CLI via `clap`).
- Aplicação das diretrizes de segurança N-02 (`#![forbid(unsafe_code)]` em crates de domínio puro e
  `#![deny(unsafe_code)]` em crates de hardware/sistema).
- Configuração de auditoria de segurança e licenças via `cargo-deny` (`deny.toml`).
- Pipeline de CI contínuo nos 3 sistemas operacionais (`.github/workflows/ci.yml`).

## 2. Exceção Bootstrap de Processo SPDD

Conforme acordado no alinhamento estratégico da v1.0, a Fase F0 consiste em scaffolding de infraestrutura e governança
de build, não envolvendo modelagem de lógica de domínio ou mecânica de script. Por esta razão, a F0 é registrada como
**exceção de bootstrap documentada**.

A partir da **Fase F1 (`core/engine`)**, todo incremento funcional segue obrigatoriamente o ciclo SPDD completo:

1. `/spdd-analysis` (em `spdd/analysis/`)
2. `/spdd-reasons-canvas` (em `spdd/prompt/`)
3. `/spdd-generate` (geração e implementação)
4. `/spdd-sync` (sincronização de volta ao canvas)
