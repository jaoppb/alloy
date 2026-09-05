# B5 — `core/html`: tokenizer + tree sink

## Contexto

`core/html` ainda é o stub original (8 linhas: doc comment + `#![forbid(unsafe_code)]`). Esta fase constrói o parser
HTML5 que transforma bytes de uma página real em um `dom::DomTree` — é o caminho crítico de I2 (pipeline headless) e I4
(`alloy <url>`), porque sem ele o pipeline só consegue processar HTML montado à mão em Rust, nunca uma página baixada da
rede. `PRD-008` já existe (target v0.5) e reserva o escopo; esta fase o realiza.

**Pode começar em paralelo com o fim de B4** — depende só de B0 (já entregue) e de `core/dom` (v0.2, já entregue e
estável). Não precisa esperar o Flexbox de B4 terminar.

## Estado atual

Confirmação de que não existe nenhum código além do stub:

```bash
wc -l core/html/src/lib.rs
# → 8 core/html/src/lib.rs
ls core/html/src
# → lib.rs (só)
```

`core/dom` (o alvo da construção) já expõe tudo que um _tree sink_ precisa: `DomTree::create_element`, `create_text`,
`create_comment`, `append_child`, `set_attribute` — todas validando invariantes e nunca entrando em pânico num nó
malformado (ver `core/dom/src/domain/tree.rs`).

## Passos

1. **Domínio** (`core/html/src/domain/`):
    - `token.rs` — os tipos de token HTML5 (`StartTag`, `EndTag`, `Character`, `Comment`, `Doctype`, `EndOfFile`), cada
      um um _value object_ validado (nome de tag lowercased, sem primitivo cru escapando).
    - `tag.rs` — vocabulário de tags que o parser precisa reconhecer por nome (elementos void: `br`, `img`, `input`,
      `hr`, …; elementos de conteúdo bruto: `script`, `style`, `textarea`).
    - `error.rs` — `HtmlError`, `#[non_exhaustive]`, `#[derive(thiserror::Error)]` (não é o carve-out de `core/engine` —
      essa correção já está registrada: as fases de `core/html`/`core/css`/`core/network`/ `core/window` usam
      `thiserror` como o resto do domínio, só `core/engine` escreve `Display` à mão por ADR-0015).

2. **Application** (`core/html/src/application/`):
    - `ports.rs` — dois traits object-safe: um `TokenSink` (recebe tokens do tokenizer) e um `TreeSink` (constrói a
      árvore a partir dos tokens) — no molde de como `html5ever` separa as duas responsabilidades, mas sem depender
      dele.
    - `conformance.rs` — `run_html_conformance(&dyn TreeSink)`, no molde de `core/css/src/application/conformance.rs`:
      `pub fn`, cada checagem uma função privada, header `#![allow(clippy::panic, clippy::expect_used)]`.

3. **Infrastructure** (`core/html/src/infrastructure/`):
    - `tokenizer.rs` — a máquina de estados do tokenizer HTML5 (WHATWG HTML Standard §13.2.5), pelo menos os estados que
      um documento real usa: dados, tag aberta, nome de tag, atributo, valor de atributo (com e sem aspas), comentário,
      doctype. `<script>`/`<style>` entram no modo de texto bruto (_RAWTEXT_/_script data_) — o conteúdo deles não é
      tokenizado como marcação.
    - `tree_builder.rs` — implementa `TreeSink` sobre um `dom::DomTree`: mantém a pilha de elementos abertos, insere
      texto/elemento/comentário no ponto de inserção corrente, fecha tags conforme as regras de _tag omission_ mínimas
      que um corpus real exige (não precisa implementar a árvore de algoritmos completa do §13.2.6 — só o suficiente
      para não produzir uma árvore visivelmente errada em HTML bem formado).

4. **`core/html/Cargo.toml`** — dependência: `dom`. **Não** `engine` (a mesma correção de `overview.md` que já foi
   aplicada na Fase 0 para `css`/`network`/`window` vale aqui).

5. **Corpus e manifesto** — reuse o padrão inventado em B1 (`core/css/tests/data/MANIFEST.md` + `manifest_runner.rs`,
   cuja lógica de comparação nos dois sentidos você pode copiar quase literalmente, trocando
   `SUPPORTED_PROPERTIES`/`SUPPORTED_SELECTORS` por um registro equivalente de formas HTML suportadas). Corpus alvo: uma
   página de complexidade parecida com `example.com` — cabeçalho, parágrafos, uma lista, um link, um `<script>` cujo
   conteúdo nunca deve ser tokenizado como marcação.

6. **Feature `no-default-features`** — `core/html` deve compilar sem nenhuma feature ligada (não há adaptador "de rede"
   aqui para gatear, mas o padrão de todo crate de porta desta campanha é ter esse caminho testável; se não houver nada
   a gatear, documente por quê em vez de inventar uma feature vazia).

## Crates de referência

- `core/dom` — domínio puro já entregue; é o alvo desta fase, não um molde de estilo.
- `core/css` (B0–B1) — o padrão de porta + conformance + MANIFEST.md a espelhar.

## Definition of Done

- [ ] `cargo build -p html --all-targets` limpo.
- [ ] `cargo clippy -p html --all-targets --all-features -- -D warnings` limpo.
- [ ] `core/html` parseia o corpus classe-`example.com` para um `dom::DomTree` correto (verificado por teste, não por
      inspeção visual).
- [ ] `core/html/tests/data/MANIFEST.md` verde nos dois sentidos (`manifest_runner.rs` equivalente).
- [ ] `cargo test -p html --no-default-features` compila e passa.
- [ ] `cargo tree -p html` sem `engine`/`rhai`.
- [ ] `cargo test --workspace` continua todo verde.
- [ ] `just no-engine` / `just gate` verdes.

## Convenção de commit

```text
feat(html): HTML5 tokenizer and tree sink over core/dom (v0.5 B5)

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
```
