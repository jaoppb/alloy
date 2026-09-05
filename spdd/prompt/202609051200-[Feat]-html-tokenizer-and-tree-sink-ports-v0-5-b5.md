# SPDD Feat Prompt — v0.5 B5 (`core/html`): HTML5 tokenizer and tree sink over `core/dom`

## Contexto e Escopo

Implementação do parser HTML5 e portas `TokenSink` / `TreeSink` sobre `core/dom`:

- Crate: `core/html`
- Entregáveis:
    - `core/html/Cargo.toml`
    - `core/html/src/domain/{error.rs, tag.rs, token.rs, mod.rs}`
    - `core/html/src/application/{ports.rs, conformance.rs, mod.rs}`
    - `core/html/src/infrastructure/{tokenizer.rs, dom_sink.rs, tree_builder.rs, mock.rs, mod.rs}`
    - `core/html/src/lib.rs`
    - `core/html/tests/data/MANIFEST.md`
    - `core/html/tests/data/fixtures/example_com.html`
    - `core/html/tests/manifest_runner.rs`
    - `core/html/tests/corpus_test.rs`
    - `core/html/tests/conformance_test.rs`

## Definition of Done

- [x] `cargo build -p html --all-targets` limpo.
- [x] `cargo clippy -p html --all-targets --all-features -- -D warnings` limpo.
- [x] `core/html` parseia o corpus classe-`example.com` para um `dom::DomTree` correto.
- [x] `core/html/tests/data/MANIFEST.md` verde nos dois sentidos (`manifest_runner.rs`).
- [x] `cargo test -p html --no-default-features` compila e passa.
- [x] `cargo tree -p html` sem `engine`/`rhai`.
- [x] `#![forbid(unsafe_code)]` respeitado em todos os módulos.
