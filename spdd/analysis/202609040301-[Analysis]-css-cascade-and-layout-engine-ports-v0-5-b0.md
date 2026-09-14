# SPDD Analysis — v0.5 B0 (`core/css`): boundary aggregates and the cascade/layout ports

| Campo        | Valor                                                                                                      |
| ------------ | ---------------------------------------------------------------------------------------------------------- |
| Fase         | B0 do plano `~/.claude/plans/verifique-o-docs-reports-implementacao-d-fancy-dijkstra.md` (F4c minimizada)  |
| Realiza      | `PRD-007` — portas `CascadeResolver` / `LayoutEngine` / `TextMeasurer`, agregados de fronteira             |
| Port         | Contrato de Porta Substituível `ADR-0011`; agregados **congelam em I3** (fim da B4), não nesta fase        |
| Depende de   | Fase 0 (entregue); `core/dom` (API de leitura de `DomTree`); `core/graphics` (só `Au`/`Px`/`Color`/`Rect`) |
| Estado atual | `core/css/src/lib.rs` tem 8 linhas — doc-comment e `#![forbid(unsafe_code)]`, zero funções                 |

## Original Business Requirement

Seção **"## Fase B0"** do plano (`~/.claude/plans/verifique-o-docs-reports-implementacao-d-fancy-dijkstra.md:338-398`),
verbatim:

```text
## Fase B0 — Esqueleto de portas do `core/css` (F4c minimizada)

**Objetivo:** a fronteira `PRD-007` congelável + um caminho de referência mínimo dogfooded.

### Módulos
- core/css/src/domain/{dom_snapshot.rs, stylesheet_set.rs, styled_tree.rs, layout_box_tree.rs,
  viewport.rs, computed/mod.rs, length.rs, color.rs, error.rs, mod.rs}
- core/css/src/application/{ports.rs, conformance.rs, snapshot.rs, mod.rs} — snapshot.rs faz o
  mapeamento explícito dom::DomTree -> DomSnapshot (nenhum tipo interno de core/dom vaza).
- core/css/src/infrastructure/{ua_sheet.rs, cascade/mod.rs, layout/block.rs, mock.rs, mod.rs}
- core/css/src/lib.rs — pub const PORT_SCHEMA_VERSION: u32 = 1;

### Portas (core/css/src/application/ports.rs, object-safe, sem tipo estrangeiro)
pub trait CascadeResolver: Send + Sync {
    fn resolve(&self, dom: &DomSnapshot, sheets: &StyleSheetSet) -> Result<StyledTree, CssError>;
}
pub trait LayoutEngine: Send + Sync {
    fn layout(&self, styled: &StyledTree, constraints: &ViewportConstraints)
        -> Result<LayoutBoxTree, CssError>;
}
pub trait TextMeasurer: Send + Sync {
    fn measure(&self, run: &TextRun, style: &ComputedText) -> Result<TextMetrics, CssError>;
}

CssError — #[non_exhaustive], "localização" = CssStage { Parse, Selector, Cascade, Layout, Measure }
+ SourceSpan opcional. Geometria em LayoutBoxTree é graphics::Au (ADR-0016); css depende de graphics
só para Au/Px/Color/Rect.

### Adaptadores de referência (placeholders que a F9 substitui — manter mínimos)
- UaCascade — regras UA hard-coded em Rust, origens/!important stubados para UA-only, herança só das
  propriedades que block.rs lê.
- BlockLayout — margin/padding/width/height, sem colapso.
- MonospaceMetrics — avanço fixo (font_size * 0.6), uma altura de linha.

### Mocks (infrastructure/mock.rs, também sob no-script)
MockCascadeResolver (força uma propriedade a um sentinel), MockLayoutEngine, MockTextMeasurer.

### Conformidade
- core/css/src/application/conformance.rs::run_css_conformance(&dyn CascadeResolver, &dyn LayoutEngine)
  — determinismo (100 runs idênticos), sem-tipo-estrangeiro, granularidade de árvore inteira,
  contrato de fallback.
- core/css/tests/manifest_runner.rs lendo core/css/tests/data/MANIFEST.md (recorte vazio em B0,
  cresce por fatia; falha na divergência nos dois sentidos).

### Feature
core/css/Cargo.toml — default = ["builtin-adapters"]; no-script (portas + adaptadores Rust + mocks).
O adaptador scriptável mora em rhai-bindings, nunca em css, então no-script é trivialmente satisfeito.

**Entregável:** cargo test -p css verde; --no-default-features verde; MockCascadeResolver troca e muda
um valor computado sem tocar core/dom/core/graphics; uma golden reproduzida ponta a ponta por
DomSnapshot -> CascadeResolver -> LayoutEngine -> LayoutBoxTree + shim de paint.
```

Correção do topo do plano aplicável a esta fase
(`~/.claude/plans/verifique-o-docs-reports-implementacao-d-fancy-dijkstra.md:35-38`), verbatim:

```text
- CssError / NetworkError / WindowError / HtmlError usam #[derive(thiserror::Error)] + #[error("…")]
  — o Display à mão é carve-out só de core/engine (ADR-0015). core/dom e core/graphics já usam
  thiserror; os crates novos seguem esse caminho. (O texto antigo das fases B0/C1/C2/B5 dizia
  "Display à mão" — ignorar.)
```

`PRD-007` (`docs/requirements/PRD-007-style-cascade-and-layout-engine-ports.md`), critérios de aceitação verbatim
(`PRD-007:90-101`):

```text
- [ ] CascadeResolver, LayoutEngine, and the four boundary aggregates defined in core/css, frozen at
      integration point I3.
- [ ] Built-in Rust cascade and layout adapters pass the css-conformance suite.
- [ ] A mock CascadeResolver swaps in and changes computed styles without changing core/dom or core/graphics.
- [ ] A .rhai cascade adapter alters a computed property and the screen repaints, with capability limited
      to DOM_READ | GRAPHICS_DRAW.
- [ ] A script adapter that panics falls back to the built-in resolver and the page still renders.
- [ ] core/css builds and tests with --no-default-features (feature no-script), using only Rust adapters.
- [ ] Determinism test: 100 repeated runs of the same input produce the identical LayoutBoxTree.
```

Invariantes de `PRD-007:76-87` verbatim:

```text
1. No per-node callbacks cross the seam; the unit of exchange is the whole tree.
2. Determinism: the same DomSnapshot + StyleSheetSet yields a byte-identical LayoutBoxTree...
3. Fallback: a script adapter that errors, panics, or exceeds its instruction budget falls back to the
   built-in Rust adapter, and the page still renders (PRD-003:66-69).
4. No foreign types: no core/dom or core/graphics internal type appears in a port signature or a
   boundary aggregate.
5. Contract compliance: this port satisfies all seven items of ADR-0011, including the no-script
   feature (Rust adapters only) and the css-conformance target.
```

## Domain Concept Identification

### Existing Concepts (from codebase)

- **`RenderBackend` port + `run_backend_suite`** (`core/graphics/src/application/ports.rs:37`,
  `core/graphics/src/application/conformance.rs:41`): o molde de porta `ADR-0011` completo — trait `Send + Sync`
  object-safe, suíte `pub` de biblioteca (não `#[cfg(test)]`) que os `tests/` dos adaptadores chamam, cada checagem uma
  `fn` privada, header `#![allow(clippy::panic, clippy::expect_used)]` comentado (`:29`). B0 replica esse molde para as
  três portas de CSS.
- **`PORT_SCHEMA_VERSION`** (`core/graphics/src/lib.rs:53`, `core/engine/src/lib.rs`): a versão observável dos agregados
  de fronteira (`ADR-0011` item 3). `core/graphics` começou no `1`; `core/css` faz o mesmo, e o valor **congela em I3**
  (fim da B4), não nesta fase.
- **`Au` / `Px` / `Color` / `Rect` / `Point` / `Size`** (`core/graphics/src/domain/{unit,geometry,color}.rs`): a
  geometria de ponto fixo determinística (`ADR-0016`). `Au(i32)` = 1/64 px, aritmética inteira; `Px(f32)` é a
  **entrada** do autor, cruzada uma única vez por `Au::from_px` (`core/graphics/src/domain/unit.rs:96`). `LayoutBoxTree`
  usa `Au`; `css` importa só esses quatro tipos de `graphics` (plano `:296-298`, `overview.md` linha da tabela de
  `core/css`).
- **`DomTree` API de leitura** (`core/dom/src/domain/tree.rs:130-195`): `document()`, `node_kind()`, `tag()`,
  `attribute()`, `text()`, `first_child`/`last_child`/`next_sibling`/`previous_sibling`, `children()`, `descendants()`,
  `ancestors()`. Tudo devolve `Result<_, DomError>` ou um iterador não-recursivo. É a única superfície que `snapshot.rs`
  lê. `core/dom/src/application/serialize.rs:16-24` é o consumidor `&DomTree` não-recursivo de referência (work-stack de
  `Step`s, nunca auto-chamada).
- **`DomError` com `thiserror`** (`core/dom/src/domain/error.rs:10-11`):
  `#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]` + `#[error("…")]` + `#[non_exhaustive]` + helpers
  `#[must_use]`. É a convenção fora de `core/engine` (`ADR-0015`); `CssError` a segue (correção do plano `:35-38`),
  **não** o `Display` à mão de `EngineError`.
- **Primeira-classe collections + newtypes** (`core/dom/src/domain/attributes.rs:59` `AttributeMap`,
  `core/dom/src/domain/traversal.rs` `Children`/`Descendants`): sem `Vec`/`HashMap` público; iteração via
  `impl Iterator`; VOs validados e lowercased (`TagName::new` `core/dom/src/domain/tag_name.rs:194`).
- **`engine::capability::profiles::css_style()`** (`core/engine/src/domain/capability.rs:92`, já
  `DOM_READ | GRAPHICS_DRAW`): o perfil que o adaptador scriptável de `PRD-007:63-67` recebe. **Fora do escopo de B0** —
  registrado só porque a Fase M o usará; `core/css` não nomeia `engine`.
- **Blessing por env var** (`core/graphics/src/infrastructure/golden.rs:49-66`, `UPDATE_GOLDEN`): o primitivo reusável
  para gates cujo caminho default **falha** na divergência e só grava quando explicitamente pedido. `manifest_runner`
  espelha esse formato (`UPDATE_MANIFEST`).
- **`no-engine` / `layering`** (`justfile:127-139`, `.github/workflows/ci.yml:94-123`): o job que prova que um crate de
  domínio não linka `engine`/`rhai`. Recon: `just gate` (`justfile:85`) **não** inclui `no-engine` hoje apesar do
  comentário — B0 corrige.

### New Concepts Required

- **`DomSnapshot`** — projeção imutável, somente-leitura, de `DomTree` (`PRD-007:35-36`): forma da árvore + tag +
  atributos, com handles opacos (`SnapshotId`) e uma visão emprestada (`NodeRef`). Nenhum tipo interno de `core/dom`
  (`NodeId`, `DomTree`, `TagName`) aparece na API pública — tag e atributos viram `String`/`&str`. Produzido **só** por
  `application/snapshot.rs::snapshot(&DomTree, NodeId)`.
- **`StyleSheetSet`** — regras ordenadas com `Origin { UserAgent, User, Author }` (`PRD-007:37-38`). Coleção de primeira
  classe; sem `Vec` público. B0 não tem parser (isso é B1), então o conjunto é um andaime honesto: texto de seletor +
  bloco de declarações cru, populável por builder.
- **`StyledTree`** — valor computado por nó depois da cascata (`PRD-007:39`). Espelha a forma da árvore
  (`parent`/`children` por nó) porque `LayoutEngine::layout` recebe **só** `&StyledTree` — a estrutura tem de viajar
  dentro do agregado. B0 computa exatamente 6 propriedades: `display`, `color`, `background-color`, `margin`, `padding`,
  `font-size`.
- **`ComputedStyle`** — o pacote das 6 propriedades computadas. `SUPPORTED_PROPERTIES` é a lista canônica que o
  `manifest_runner` confere nos dois sentidos.
- **`LayoutBoxTree`** — caixas com geometria resolvida em `Au` (`PRD-007:40`), prontas para gerar `DisplayList`.
- **`ViewportConstraints`** — largura/altura em `Au`; a entrada de `LayoutEngine::layout`.
- **`Length`** — soma `px`/`em`/`rem`/`%`/`pt`; a unidade que a cascata produz e o layout resolve para `Au`.
- **`CssColor`** — embrulha/paraleliza `graphics::Color`, para o vocabulário de cor de CSS não vazar `graphics` na API
  dos agregados.
- **`CssError` / `CssStage` / `SourceSpan`** — o erro único da porta (`ADR-0011` item 4) + metadado de localização
  (`ADR-0011:93-95`): `CssStage { Parse, Selector, Cascade, Layout, Measure }` + `SourceSpan { line, column }` opcional.
- **`TextRun` / `ComputedText` / `TextMetrics`** — entrada/estilo/resultado de `TextMeasurer::measure`.
- **`run_css_conformance(&dyn CascadeResolver, &dyn LayoutEngine)`** — a suíte que fixa a **porta**, não um adaptador:
  determinismo (100 runs), granularidade de árvore inteira, ausência de tipo estrangeiro, robustez de documento vazio.
- **Adaptadores de referência** — `UaCascade` (regras UA hard-coded, herança só de `color`/`font-size`), `BlockLayout`
  (box model sem colapso), `MonospaceMetrics` (avanço `font_size * 0.6`). Placeholders que B2/B4 substituem.
- **Mocks** — `MockCascadeResolver` (força `color` a um sentinel), `MockLayoutEngine`, `MockTextMeasurer`. Provam a
  troca da porta.
- **`MANIFEST.md` + `manifest_runner`** — o padrão novo (nenhum crate tem `tests/data/MANIFEST.md` hoje, plano `:44`): o
  `MANIFEST.md` lista propriedades e formas de seletor suportadas; o runner falha se o código suporta algo não-listado
  **ou** lista algo não-suportado. B0 semeia com as 6 propriedades e zero seletores; o mecanismo é real (reusado em B5).

### Key Business Rules

- **Nenhum callback por nó cruza a fronteira** (`PRD-007:78`, `:51`): a unidade de troca é a árvore inteira. Governa as
  três portas — a assinatura recebe/devolve um agregado, nunca um handler.
- **Mesma entrada, mesma saída** (`PRD-007:52`, `:80`, `ADR-0016`): `resolve` e `layout` são puras e determinísticas.
  Governa `StyledTree`, `LayoutBoxTree`, a resolução `Length -> Au`, a ordem de iteração do `DomSnapshot`.
- **Nenhum tipo estrangeiro na fronteira** (`PRD-007:83-84`): nem `core/dom` nem tipo interno de `core/graphics` numa
  assinatura de porta ou num agregado. Exceção documentada: `Au`/`Px`/`Color`/`Rect` de `graphics` são unidades
  compartilhadas, não tipos internos (plano `:296-298`). `snapshot.rs` é o **único** ponto que nomeia `dom::DomTree` /
  `dom::NodeId`.
- **`DomSnapshot` só nasce pelo mapeamento explícito** (`PRD-007:36`): sem construtor público de `Vec`; o caminho é
  `snapshot(&DomTree, NodeId)`. Governa `DomSnapshot`, `snapshot.rs`.
- **A cascata cai para o built-in** (`PRD-007:82`): um adaptador scriptável que erra/entra em pânico cai para o
  adaptador Rust e a página ainda renderiza. B0 não tem caminho scriptado (mora em `rhai-bindings`), então a regra é
  exercitada como robustez: ambos os adaptadores built-in têm sucesso num documento mínimo e num `Document` vazio.
- **Object Calisthenics integral** (`CLAUDE.md`, `ADR-0010:127-137`): sem primitivo cru no domínio, first-class
  collections, sem `else`, um nível de indentação por função, entidades < ~100 linhas, sem campo público mutável, sem
  abreviação. `core/dom` e `core/graphics` são a referência sem exceção.
- **`css` não linka `engine`/`rhai`** (`ADR-0002`, `PRD-001:99`): `cargo tree -p css` mostra só `dom`, `graphics`,
  `thiserror`. Governa `Cargo.toml`, o job `no-engine`/`layering`.

## Strategic Approach

### Solution Direction

`core/css` vira um crate de três camadas (`ADR-0010`) que **não** parseia CSS ainda (isso é B1): B0 entrega só a
fronteira `PRD-007` — agregados imutáveis versionados, três portas object-safe, e um caminho de referência dogfooded que
prova a fronteira sem bypass. O fluxo é
`dom::DomTree --snapshot()--> DomSnapshot --CascadeResolver--> StyledTree --LayoutEngine--> LayoutBoxTree`, com
`TextMeasurer` consumido pelo layout. Os adaptadores built-in (`UaCascade`, `BlockLayout`, `MonospaceMetrics`) são
deliberadamente mínimos — B2 reescreve a cascata, B4 reescreve o layout, e o freeze I3 só acontece no fim da B4. A suíte
`run_css_conformance` fixa a porta contra **dois** conjuntos de adaptadores (built-in e mock) para provar que ela pina o
contrato, não uma implementação. O `manifest_runner` estreia o padrão de manifesto bidirecional que B5 reusa.

### Key Design Decisions

- **`StyledTree` carrega a forma da árvore**: trade-off — duplica `parent`/`children` que já estão no `DomSnapshot`. →
  Recomendado: `LayoutEngine::layout(&self, styled: &StyledTree, …)` (`PRD-007:56-60`) recebe **só** o styled tree; sem
  a estrutura dentro do agregado o layout não teria como ordenar caixas. É o preço da granularidade de árvore inteira.
- **Tag e atributos do `DomSnapshot` como `String`/`&str`, não `dom::TagName`**: trade-off — perde a validação do
  newtype de `core/dom` e realoca strings. → Recomendado: `PRD-007:36` e `:83` proíbem tipo interno de `core/dom` na
  API; `TagName` é um tipo interno. `String` desacopla de verdade e o custo é um clone por elemento, uma vez, na
  fronteira.
- **`Length` como soma `px/em/rem/%/pt` com payload `f32`**: trade-off — `f32` é primitivo. → Recomendado: `Length`
  **é** o value object (uma soma tipada), exatamente como `graphics::Px(f32)` (`core/graphics/src/domain/unit.rs:27`) é
  um VO com payload `f32`. A resolução para `Au` acontece uma vez, no `BlockLayout`, com `checked_*` sob o portão de
  clippy.
- **`CssError` com `thiserror`, não `Display` à mão**: trade-off — proc-macro no grafo de deps de `css`. → Recomendado:
  a correção do plano (`:35-38`) é explícita — o carve-out manual é **só** de `core/engine` (`ADR-0015`). `core/dom` e
  `core/graphics` já pagam `thiserror`; `css` segue. `CssError` deriva também `Eq` (o brief lista só `PartialEq`): todo
  campo é `Eq`-capaz e o lint `nursery` `derive_partial_eq_without_eq` exigiria — igual a `DomError`/`GraphicsError`.
- **Feature `builtin-adapters` nominal em B0**: trade-off — uma feature que não gateia nada parece morta. → Recomendado:
  o único adaptador que `no-script` removeria é o scriptável, que **nunca** mora aqui (mora em `rhai-bindings`).
  `--no-default-features` compila e passa idêntico a `--all-features`; o comentário no `[features]` explica, espelhando
  `core/graphics/Cargo.toml:9-15`. B1 pode passar a gatear conteúdo real.
- **`manifest_runner` com registro programático (`css::SUPPORTED_PROPERTIES`)**: trade-off — a lista é sincronizada à
  mão em B0 (não derivada de um parser). → Recomendado: `ComputedStyle` tem exatamente esses 6 campos, então a lista é
  um registro real, se manual; B1 a deriva do parser. O runner compara `SUPPORTED_PROPERTIES` ⇄ `MANIFEST.md` e falha
  nos dois sentidos, com um teste que prova a detecção de divergência mecanicamente.
- **Sem golden PNG ponta-a-ponta em B0**: trade-off — o entregável do plano menciona "uma golden reproduzida ponta a
  ponta … + shim de paint". → Recomendado: o DoD do brief desta rodada pede só determinismo **estrutural** (100 runs →
  `StyledTree` + `LayoutBoxTree` idênticos), não pixel. O `paint`/`RenderBackend` shim é I2 (`plano §I2`,
  `alloy/src/application/paint.rs`). O caminho `DomSnapshot -> Cascade -> Layout -> LayoutBoxTree` é exercitado
  ponta-a-ponta em `run_css_conformance` e num `tests/pipeline.rs` dedicado.

### Alternatives Considered

- **Escrever o parser de CSS agora (juntar B0+B1)**: rejeitado. `PRD-007:12` mantém o parsing como Rust nativo separado
  das portas; o freeze I3 é dos **agregados**, e um parser incompleto os faria mudar de forma. B0 é a fronteira
  congelável; B1 a preenche.
- **`DomSnapshot` emprestando `&DomTree` em vez de projetar**: rejeitado. `PRD-007:35` pede projeção **imutável** com
  "no core/dom internal type leaks"; um empréstimo vaza `DomTree` e prende o lifetime do styled tree ao da árvore DOM.
- **Portas genéricas (`fn resolve<S: Sheets>(…)`)**: rejeitado. `PRD-007:45` e `:57` fixam as assinaturas object-safe;
  `run_css_conformance` recebe `&dyn CascadeResolver` (molde de `run_backend_suite`,
  `core/graphics/src/application/conformance.rs:41`). Genéricos quebram `&dyn` e o `ADR-0011` item 2.
- **`Display` à mão em `CssError` (texto antigo das fases)**: rejeitado pela correção `:35-38` do topo do plano.
- **Geometria do `LayoutBoxTree` em `Px(f32)`**: rejeitado por `ADR-0016` — acumula erro de ponto flutuante e o teste de
  determinismo de 100 runs viraria loteria. Tudo em `Au(i32)`.

## Risk & Gap Analysis

### Requirement Ambiguities

- **`TextRun` / `ComputedText` / `TextMetrics` não têm arquivo na lista de módulos** (`plano:344-345` só cita
  `dom_snapshot/stylesheet_set/styled_tree/layout_box_tree/viewport/computed/length/color/error/mod`). Resolução: novo
  `core/css/src/domain/text.rs` para os três — é o lar natural e mantém `computed/mod.rs` focado em enums de valor
  computado. Desvio mínimo, registrado.
- **O plano não numera os critérios de `PRD-007`** — não existe "C-…" no arquivo; a numeração vem do
  roadmap/`CLAUDE.md`. A Fase P retrofita os rótulos no PRD.
- **`PRD-007:92` diz "the four boundary aggregates"** mas lista `DomSnapshot`, `StyleSheetSet`, `StyledTree`,
  `LayoutBoxTree` (`:35-40`) e a fase pede também `ViewportConstraints`. Resolução: os quatro do PRD são os agregados
  versionados; `ViewportConstraints` é uma entrada de VO da porta de layout, não um agregado de fronteira. Ambos
  `#[non_exhaustive]`.
- **"granularidade de árvore inteira … um property de tempo de compilação, asserte estruturalmente"**: não há como
  asserir a ausência de um parâmetro em tempo de execução. Resolução: `check_whole_tree_granularity` prova o análogo
  comportamental — uma única chamada a `resolve` estiliza **todos** os nós do snapshot (contagem igual), e a assinatura
  não tem `FnMut` de callback (documentado).

### Edge Cases

- **`Document` sem filhos**: `snapshot(&DomTree::new(), root)` — o snapshot tem um nó só. `resolve`/`layout` não podem
  entrar em pânico; devolvem `StyledTree`/`LayoutBoxTree` com um nó / zero caixas. Coberto por
  `check_empty_document_is_handled`.
- **`display: none`**: `head`, `style`, `script`, `title` não geram caixa. `BlockLayout` pula o nó e sua subárvore. Se
  não pular, o layout conta caixas fantasma.
- **Herança de raiz**: o `Document` não tem pai; `color`/`font-size` caem para o valor inicial (`CssColor::BLACK`,
  `16px`). A ordem de iteração tem de garantir pai antes de filho — `snapshot()` numera em pré-ordem DFS, então `0..len`
  é ordem de documento e todo `parent_id < child_id`, e a cascata é um laço para frente de uma passada.
- **`Length::Percent` sem contêiner dimensionado**: `%` de `font-size` ou de largura quando o pai tem `Au::ZERO`.
  Resolução B0: `%` resolve contra a largura de contêiner conhecida (viewport para a raiz); `0%` de `0` é `0`, sem
  divisão por zero.
- **`TextRun` vazio**: `measure("")` → `TextMetrics` de largura `0`, altura de uma linha. Sem `checked_mul` estourando.
- **Snapshot de um nó que não é `Document` nem `Element` como raiz**: `snapshot(&tree, text_node_id)` — a raiz vira um
  nó `Text`; `resolve` o estiliza com o valor inicial, `layout` não gera caixa. Não é erro.

### Technical Risks

- **O portão de clippy do workspace proíbe o vocabulário de resolução de unidade**: `arithmetic_side_effects`,
  `as_conversions`, `indexing_slicing`, `string_slice` são `deny` (`Cargo.toml:51-66`). Mitigação: `domain/` e os
  adaptadores mantêm o portão integral — `checked_*`/`saturating_*` em `Au`, `i32::try_from`/`u32::try_from` para
  estreitar, `.get()` para acesso a coleção. `core/graphics/src/domain/unit.rs` e `core/dom/src/domain/tree.rs:351` são
  a referência. Nenhum `#[allow(...)]` em código de lib nesta fase — só em `conformance.rs` (header comentado citando
  `core/graphics/src/application/conformance.rs:29`) e em `tests/`.
- **`derive_partial_eq_without_eq` (`nursery` = `deny`)**: derivar só `PartialEq` num tipo `Eq`-capaz falha o portão.
  Mitigação: `LayoutBoxTree`/`LayoutBox`/`EdgeSizes`/`DomSnapshot`/`CssColor`/`CssError` derivam `PartialEq, Eq`;
  `StyledTree`/`ComputedStyle`/`Length` **não** podem derivar `Eq` (payload `f32`), então o lint não dispara neles.
- **`module_name_repetitions` (`pedantic`)**: `dom_snapshot::DomSnapshot`, `styled_tree::StyledTree`, etc. Mitigação:
  re-exportar tudo no facade de `lib.rs` (molde de `core/graphics/src/lib.rs:57-71`); o caminho público canônico
  (`css::DomSnapshot`) não repete e o lint não dispara.
- **`missing_const_for_fn` / `must_use_candidate` (`nursery`/`pedantic` = `deny`)**: cada getter puro precisa de `const`
  onde o corpo permite e de `#[must_use]`. `core/graphics/src/domain/*` é a referência exaustiva.
- **`arch-lint` não tem escopo para `css`** — sem os `[[scopes]]` `css_domain`/`css_application`/`css` e os
  `[[deny-scope-dep]]` correspondentes o crate entra sem regra de camada. `css` **pode** depender de `dom` e `graphics`
  (ao contrário de `graphics`, que nega `dom` em `arch-lint.toml:118`), então a deny-list de `css` é
  `["engine", "runtime_rhai", "runtime_rhai_bindings", "alloy_cli"]` — sem `dom`/`graphics`.
- **`just gate` não roda `no-engine`** (`justfile:85`, recon do plano `:47`) — a asserção nova de `css` não seria
  exercitada localmente. B0 adiciona `no-engine` à lista de dependências da recipe `gate`.
- **Feature de teste sob `--no-default-features`**: `css_conformance.rs` e `port_swap.rs` usam
  `UaCascade`/`BlockLayout`. Se esses fossem gateados por `builtin-adapters`, o teste `--no-default-features` não
  compilaria. Mitigação: em B0 nada é gateado pela feature (ver decisão); os dois configs de feature compilam e passam
  idênticos.
- **Ordem de `cargo tree`**: `graphics` e `dom` puxam `thiserror` → `syn`/`proc-macro2`/`quote`/`unicode-ident`. Nenhum
  puxa `engine` ou `rhai`, então `cargo tree -p css` fica limpo — mas a asserção do CI/justfile tem de ancorar o nome do
  pacote no início da linha (`--prefix none`), não casar em substring, para não pegar `thiserror`.

### Acceptance Criteria Coverage

| AC (`PRD-007`) | Descrição                                                                     | Endereçável? | Lacunas / Notas                                                                                                 |
| -------------- | ----------------------------------------------------------------------------- | ------------ | --------------------------------------------------------------------------------------------------------------- |
| `:92`          | Portas + 4 agregados definidos em `core/css`, congelam em I3                  | Yes          | Definidos em B0; o **freeze** e o `style-cascade-port-contract.md` são B4 (I3). `lib.rs` já aponta pra lá       |
| `:93`          | Adaptadores Rust built-in passam a suíte `css-conformance`                    | Yes          | `run_css_conformance(&UaCascade::new(), &BlockLayout::new())` em `tests/css_conformance.rs`                     |
| `:94`          | Mock `CascadeResolver` troca e muda estilo computado sem tocar dom/gfx        | Yes          | `tests/port_swap.rs`: `MockCascadeResolver` força `color` a `SENTINEL_COLOR`; teste só nomeia tipos `css::`     |
| `:95`          | Adaptador `.rhai` altera propriedade computada, cap `DOM_READ\|GRAPHICS_DRAW` | No           | Fora de B0 — o adaptador scriptável mora em `rhai-bindings` (Fase M). B0 só deixa a porta pronta                |
| `:96`          | Adaptador scriptável que entra em pânico cai pro built-in, página renderiza   | Partial      | B0 sem caminho scriptado; exercitado como robustez (documento vazio / mínimo não entra em pânico)               |
| `:97`          | `core/css` compila e testa com `--no-default-features` (`no-script`)          | Yes          | Job `layering` do CI + `just no-engine` + DoD: `cargo test -p css --no-default-features`                        |
| `:98`          | Determinismo: 100 runs da mesma entrada → `LayoutBoxTree` idêntico            | Yes          | `check_cascade_is_deterministic` / `check_layout_is_deterministic` em `conformance.rs` (100 runs, `assert_eq!`) |
