# P — portões, ADRs, PRDs, _contract records_

## Contexto

A última fase: fecha a documentação e os portões de CI que as seis fases de código deixaram em aberto. Depende de
**todas** as fases anteriores. Termina em PR final, fundindo a v0.5 inteira.

## Passos

### Documentos

1. Finalizar **ADR-0018** (`unsafe` por superfície de ameaça) e **ADR-0019** (event loop único) — hoje `Proposed`, viram
   `Accepted`. Atualize `docs/adr/README.md`.
2. **Reescrever N-02** em `docs/requirements/PRD-001-alloy-core-system.md:97` — "zero `unsafe` exposto a runtimes de
   script" nunca foi verdade (`rhai` transmuta na costura de _binding_ desde a v0.1, já documentado em
   `docs/reports/VIOLACAO-N02-UNSAFE-NO-RHAI.md`); substitua pelo critério por superfície de ameaça do ADR-0018.
3. **PRD-009** (`docs/requirements/PRD-009-network-transport-and-request-policy-port.md`) e **PRD-010**
   (`docs/requirements/PRD-010-window-system-and-presenter-port.md`) — os dois _seam PRDs_ que C1/C2 pressupõem mas
   ainda não têm documento formal.
4. _Contract records_:
   `docs/architecture/{http-transport-port-contract.md, window-system-port-contract.md, html-tree-sink-port-contract.md}`
   — os 7 itens do ADR-0011 cada (`style-cascade-port-contract.md` já deveria existir, escrito ao final de B4).
5. Nota de migração `PRD-002` §4.2 confirmando o schema 3 do `engine` (já registrada em EE — só confirme que
   `docs/requirements/README.md` está consistente).
6. `CLAUDE.md` — reescrever "Current State" para refletir a v0.5 inteira. `docs/README.md` — linha da v0.5. `deny.toml`
   — confirmar que todas as licenças da pilha TLS + janela + `ttf-parser` estão cobertas.

### Portões de CI novos (`.github/workflows/ci.yml`, `justfile`, `arch-lint.toml`)

| Portão            | O que faz                                                                                          | Bloqueante                        |
| ----------------- | -------------------------------------------------------------------------------------------------- | --------------------------------- |
| `hook-benchmark`  | `cargo bench -p rhai-runtime --bench hook_overhead`; compara à _baseline_ de M; falha em regressão | Sim                               |
| `unsafe-audit`    | deixa de ser _advisory_ — vira bloqueante contra `unsafe-allowlist.toml`                           | Sim                               |
| `css-conformance` | `-p css --test manifest_runner` **e** `-p html --test manifest_runner`                             | Sim                               |
| `fuzz`            | `cargo-fuzz` em `{css_parse, inflate, png_decode}`, 10 min/alvo                                    | Sim                               |
| `layering`        | renomeia `no-engine`; adiciona os `cargo tree`/`--no-default-features` de todas as fases           | Sim                               |
| `coverage`        | `--package css --package network --package window --package html` domínio ≥ 85 %                   | Sim (domínio)                     |
| `supply-chain`    | `deny.toml` com todas as licenças novas                                                            | Sim (já existe, só amplia escopo) |

## Definition of Done

- [ ] ADR-0018/0019 em `Accepted`.
- [ ] N-02 reescrito em PRD-001.
- [ ] PRD-009/PRD-010 escritos.
- [ ] Os quatro _contract records_ de `docs/architecture/` completos.
- [ ] Todos os portões da tabela acima bloqueantes em CI.
- [ ] `CLAUDE.md` e `docs/README.md` atualizados.
- [ ] `just gate` verde no estado final da branch inteira.
- [ ] **PR final** abrindo a fusão de `feat/v0-5` para `main`.

## Convenção de commit

```text
docs(v0.5): finalize ADRs, PRDs, contract records, CI gates (v0.5 P)

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
```

Depois deste commit, abra o PR final via `gh pr create` com a branch `feat/v0-5` contra `main`.
