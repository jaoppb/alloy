# I2 — pipeline headless (`alloy render`)

## Contexto

Até aqui, `alloy` é só `src/main.rs` (90 linhas) rodando um `.rhai` sob sandbox — não depende de `graphics`, `css` ou
`html`. I2 é o primeiro ponto em que as peças (parser HTML, cascata, layout, _display list_, _backend_ de software) se
conectam de ponta a ponta: bytes de HTML → PNG num arquivo, sem janela nenhuma. É o **checkpoint de integração** do
plano — depois deste commit, dá _push_ e abre um PR draft.

**Depende de B4 e B5 completos e commitados.** Não comece antes dos dois.

## Estado atual

```bash
ls alloy/src/lib.rs alloy/src/application 2>&1
# → No such file or directory (alloy ainda não tem split lib/bin nem application/)
grep -n "graphics\|css\|html" alloy/Cargo.toml
# → nenhuma dependência ainda
```

## Passos

1. **Split lib/bin** — `alloy/Cargo.toml` ganha `[lib]` + `[[bin]]`; `alloy/src/lib.rs` novo com a lógica,
   `alloy/src/main.rs` fica fino (só chama a lib e trata o código de saída do processo).

2. **`alloy/src/application/pipeline.rs`** — a função que encadeia:

    ```text
    bytes → (html) DomTree → (css snapshot) DomSnapshot → CascadeResolver → StyledTree
         → LayoutEngine → LayoutBoxTree → paint → DisplayList → RenderBackend → Framebuffer
         → png::encode
    ```

    Cada seta é uma chamada de função já existente em algum crate — esta fase não inventa lógica nova de
    parsing/cascata/layout, só encadeia o que B0–B5 já produziram.

3. **`alloy/src/application/paint.rs`** — `LayoutBoxTree → DisplayList` via `DisplayListBuilder` (de `core/graphics`).
   Decisão já tomada no plano original: fica em `alloy`, não em `core/css` nem `core/graphics` — nenhum "segundo
   consumidor" do `LayoutBoxTree` apareceu que justificasse promover isso a porta; reavaliar na v0.7 se aparecer.

4. **CLI** — subcomando `alloy render <file.html> -o <out.png> [--width W] [--height H]`, via `clap` subcommands.
   Mantenha `--script` funcionando por compatibilidade (não regrida o caminho da v0.1/v0.2). `alloy/Cargo.toml` ganha as
   dependências `css`, `graphics`, `html`.

5. **Teste golden ponta a ponta** — `alloy/tests/render_golden.rs`: um HTML local (pode reusar/estender o corpus de B5)
   renderizado para PNG, comparado byte a byte contra uma imagem abençoada (`UPDATE_GOLDEN=1`, o mesmo mecanismo de
   `core/graphics/src/infrastructure/golden.rs`).

## Crates de referência

- `core/graphics/src/infrastructure/golden.rs` — o mecanismo de _golden image_ a reusar tal como está.
- `alloy/src/main.rs` atual — o padrão de tratamento de erro tipado (`AlloyError`, `thiserror`) e de `tracing` para
  diagnóstico (ADR-0014) que o `pipeline.rs` novo deve seguir.

## Definition of Done

- [x] `alloy render page.html -o out.png` produz um PNG.
- [x] O PNG é byte-idêntico entre execuções (determinismo, ADR-0016).
- [x] Golden ponta a ponta em CI.
- [x] `--script <path>` continua funcionando como antes (nenhuma regressão do caminho v0.1/v0.2).
- [x] `cargo test --workspace` continua todo verde.
- [ ] **Checkpoint:** depois do commit, `git push -u origin feat/v0-5` e abrir um PR draft "v0.5 · I2 render headless"
      via `gh`.

## Convenção de commit

```text
feat(alloy): headless render pipeline, `alloy render` subcommand (v0.5 I2)

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
```
