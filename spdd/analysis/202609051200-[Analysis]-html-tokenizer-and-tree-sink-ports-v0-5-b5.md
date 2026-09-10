# SPDD Analysis — v0.5 B5 (`core/html`): HTML5 tokenizer and tree sink over `core/dom`

| Campo      | Valor                                                                                             |
| ---------- | ------------------------------------------------------------------------------------------------- |
| Fase       | B5 do plano v0.5 (`docs/v0-5-handoff/02-b5-html-tokenizer.md`)                                    |
| Realiza    | `PRD-008` — portas `TokenSink` / `TreeSink`, tokenizer WHATWG §13.2.5 e `DomTreeSink`             |
| Port       | Contrato de Porta Substituível `ADR-0011`; conformidade `run_html_conformance(&mut dyn TreeSink)` |
| Depende de | B0 (entregue); `core/dom` (v0.2 estável, arena `DomTree`)                                         |
| Isolamento | Zero dependência de `core/css` ou `core/engine`; paralelismo total com B4                         |

## 1. Original Business Requirement

Conforme `docs/v0-5-handoff/02-b5-html-tokenizer.md`: Transformar o stub de `core/html` em um parser HTML5 completo
sobre `core/dom`:

- **Domínio**: `token.rs` (tokens de tag, doctype, caractere, comentário, eof), `tag.rs` (elementos void, rawtext,
  formatação, bloco), `error.rs` (`HtmlError` com `thiserror`).
- **Aplicação**: `ports.rs` (`TokenSink`, `TreeSink`), `conformance.rs` (`run_html_conformance`).
- **Infraestrutura**: `tokenizer.rs` (máquina de estados WHATWG §13.2.5 com modo RAWTEXT para `<script>`/`<style>`),
  `tree_builder.rs` (construção de árvore sobre `TreeSink` com omissão de tags para `<p>` e `<li>`), `dom_sink.rs`
  (`DomTreeSink`), `mock.rs` (`MockTreeSink`).
- **Manifesto e Corpus**: `MANIFEST.md` + `manifest_runner.rs` bidirecional e teste com corpus real `example.com`.

## 2. REASONS Canvas Architecture

- **Role**: O crate `core/html` atua como o parser HTML5 Skeleton-side, convertendo sequências de caracteres UTF-8 em
  nós válidos na arena `DomTree`.
- **Entities & VOs**: `DoctypeToken`, `TagToken`, `AttributeEntry`, `AttributeList`, `Token`.
- **Architecture**: Inward Clean Architecture (`domain/` <- `application/` <- `infrastructure/`).
  `#![forbid(unsafe_code)]`. Object Calisthenics estrito (sem `else`, sem `unwrap`, coleções de primeira classe).
- **Security**: Entrada hostil tratada sem pânico; decodificação segura de entidades de caracteres.
