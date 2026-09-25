# SPDD Analysis — v0.5 B4 (`core/css`): box model, contexto inline e Flexbox

| Campo        | Valor                                                                                                                |
| ------------ | -------------------------------------------------------------------------------------------------------------------- |
| Fase         | B4 do plano `~/.claude/plans/verifique-o-docs-reports-implementacao-d-fancy-dijkstra.md:483-508` (F9c)               |
| Realiza      | `PRD-007` §3.3 / §3.5 — o `LayoutEngine` embutido deixa de ser placeholder e vira o motor real de fluxo + Flexbox    |
| Recorte      | `MANIFEST.md` ganha as propriedades de caixa, de texto inline e de flex; tudo fora continua **recusado com nota**    |
| Depende de   | B0 (`3ea4834`) portas + agregados, B1 (`775045d`) parser, B2 (`0ed7f9c`) cascata real, B3 (`e07971d`) `TextMeasurer` |
| Fecha        | **Freeze I3** — congela os agregados de `core/css` e publica o contract record da porta (ADR-0011, 7 itens)          |
| Estado atual | `BlockLayout` empilha caixas sem colapso, sem `width`/`height`, sem borda, sem linha de texto, sem flex              |

## Original Business Requirement

Seção **"## Fase B4"** do plano (`~/.claude/plans/verifique-o-docs-reports-implementacao-d-fancy-dijkstra.md:483-508`),
verbatim:

```text
## Fase B4 — F9c: box model, contexto inline, Flexbox (16–24 d)

- `core/css/src/infrastructure/layout/{block.rs (reescrito), inline.rs, flex.rs, margin_collapse.rs,
  mod.rs}` — substitui `BlockLayout`.
  1. **Box model** — `margin`/`border`/`padding`, `box-sizing`, colapso de margem vertical (irmãos +
     pai/primeiro-último filho). **Passo 1**, com asserção de retângulo própria.
  2. **Contexto de formatação inline** — caixas de linha, `white-space` (`normal`/`pre`/`nowrap`),
     colapso de espaço, quebra suave (espaço + pós-hífen, UAX #14 simplificado), `text-align`
     (`left`/`right`/`center`/`justify`), inlines aninhados, alinhamento por baseline. Consome
     `TextMeasurer`.
  3. **Flexbox** — `flex-direction`, `flex-wrap`, `justify-content`, `align-items`, `align-content`,
     `align-self`, `flex-grow`/`shrink`/`basis`.
  4. `flex-wrap` — **a alavanca de alívio** (relatório §1.3, §2.9): se a fase estourar, cai para flex
     de linha única + adia o wrap para v0.7 por linha registrada no `MANIFEST.md`.
- Toda geometria `Au(i32)`.
- **Marcador de "intrinsic size pendente"** no `LayoutBoxTree` (para a Fase X) entra **aqui**, antes
  do freeze.
- **Commit de churn de golden #3** — `*.png` isolado (páginas com CSS de autor).
- **Freeze I3:** no fim da B4, congelar os agregados de `core/css` + `css::PORT_SCHEMA_VERSION`;
  registrar em `docs/architecture/style-cascade-port-contract.md`. Mudança posterior exige nota de
  migração em `PRD-007`.

**Entregável:** asserções de retângulo para colapso de margem, `box-sizing`, quebra de linha e cada
propriedade de Flexbox; `MockLayoutEngine` ainda troca; determinismo de 100 runs no `LayoutBoxTree`.
```

Correções ao spec (topo do mesmo plano, `:33-51`), verbatim na parte que se aplica:

```text
- **`CssError` / `NetworkError` / `WindowError` / `HtmlError` usam `#[derive(thiserror::Error)]` +
  `#[error("…")]`** — o `Display` à mão é carve-out **só** de `core/engine` (ADR-0015).
- Conformance suite fica em `src/application/conformance.rs` (molde `graphics`), `pub fn run_*_suite`,
  header `#![allow(clippy::panic, clippy::expect_used)]`. `PORT_SCHEMA_VERSION` fica direto em `lib.rs`.
```

Riscos de sequenciamento que tocam a B4 (`:803-834`), verbatim:

```text
1. **Churn de golden é 3×, não 1×.** `boxes.png` (F4a) quebra em B2 (cascata real), B3 (glifos reais),
   B4 (layout real). Cada regen é commit isolado só de `*.png`; o teste de 100 runs é o invariante que
   tem de valer em todos; nunca abençoar golden no mesmo commit de mudança de lógica.
6. **`TextMeasurer` tem de existir antes da F9c.** Definido como porta de `css` na B0 com adaptador
   sintético, F9c nunca importa tipo de fonte e roda contra métricas sintéticas mesmo se B3 escorregar.
7. **Freeze dos agregados de `css` (I3) agora cai dentro da v0.5.** O marcador "intrinsic size
   pendente" da Fase X entra na B4, antes do freeze — senão exige nota de migração em PRD-007.
```

`PRD-007` §3.1, §3.3, §3.5, §4 e §5, verbatim nas partes que a B4 realiza:

```text
- `LayoutBoxTree` — boxes with resolved geometry, ready for `DisplayList` generation.

pub trait LayoutEngine: Send + Sync {
    fn layout(&self, styled: &StyledTree, constraints: &ViewportConstraints)
        -> Result<LayoutBoxTree, CssError>;
}

The built-in Rust cascade resolver and the built-in flow-plus-Flexbox layout engine are themselves
adapters behind these ports — the contract is dogfooded, not bypassed for the default path.

2. **Determinism**: the same `DomSnapshot` + `StyleSheetSet` yields a byte-identical `LayoutBoxTree`,
   verified by golden images on `SoftwareCpuBackend` and by rectangle-assertion tests (`roadmap §5`).
4. **No foreign types**: no `core/dom` or `core/graphics` internal type appears in a port signature or
   a boundary aggregate.

- [ ] `CascadeResolver`, `LayoutEngine`, and the four boundary aggregates defined in `core/css`, frozen
      at integration point `I3`.
- [ ] Determinism test: 100 repeated runs of the same input produce the identical `LayoutBoxTree`.
```

## Domain Concept Identification

### Existing Concepts (from codebase)

- **`LayoutBoxTree` / `LayoutBox` / `EdgeSizes`** (`core/css/src/domain/layout_box_tree.rs:74-121`): `LayoutBox` carrega
  hoje `node` + `content: Rect` + `margin` + `padding` + `children`. Falta **borda** (o box model do passo 1 é
  `margin`/`border`/`padding`) e falta o marcador de intrinsic size do passo 4. `LayoutBoxTreeBuilder` (`:157-180`) é
  `pub(crate)` — só um `LayoutEngine` deste crate produz a árvore.
- **`StyledTree` / `StyledNode`** (`core/css/src/domain/styled_tree.rs:15-45`): carrega `node`, `parent`, `children`,
  `style`. **Não carrega texto.** Como `LayoutEngine::layout` recebe apenas `&StyledTree` (`PRD-007:56-60`), um contexto
  de formatação inline é hoje literalmente impossível de escrever: o conteúdo textual não atravessa a fronteira.
  `recompute_in_document_order` (`:87-108`) é a passada única pai-antes-de-filho que os três resolvers usam.
- **`ComputedStyle`** (`core/css/src/domain/computed/style.rs:22-29`): exatamente 6 campos — `display`, `color`,
  `background_color`, `margin`, `padding`, `font_size`. Todas as propriedades que o layout de B4 precisa ler (`width`,
  `height`, `border-width`, `box-sizing`, `text-align`, `white-space`, as nove de flex) **não existem**.
- **`Display`** (`core/css/src/domain/computed/display.rs:15-27`): `None` / `Block` / `Inline` / `Flex`. `Flex` já foi
  declarado em B0 "para o agregado nascer inteiro; B4 dá a ele um contexto de formatação" — é exatamente esta fase.
- **`Length` + `Length::resolve_to_au`** (`core/css/src/domain/length.rs:70`): a única travessia autor → `Au`. Não tem
  variante `auto`: `width: auto` precisa de um VO novo por cima de `Length`.
- **`TextMeasurer` / `TextRun` / `ComputedText` / `TextMetrics`** (`core/css/src/application/ports.rs:53-58`,
  `core/css/src/domain/text.rs`): a porta que o IFC consome. `TextMetrics` tem `width` + `height` e **nenhum canal de
  baseline** — "alinhamento por baseline simples" precisa de um.
- **`MonospaceMetrics`** (`core/css/src/infrastructure/text_metrics.rs`) e **`FontBackedMeasurer`**
  (`core/css/src/infrastructure/font_backed_measurer.rs`): os dois adaptadores da porta. O primeiro é sintético e
  determinístico (fracções inteiras `3/5` e `6/5`), o segundo delega a `graphics::FontProvider`. B4 consome a **porta**,
  nunca `ttf-parser` nem `graphics::infrastructure::font`.
- **`BlockLayout`** (`core/css/src/infrastructure/layout/block.rs`): o placeholder de B0. Empilha caixas de nível bloco
  numa lista plana com `cursor_y` global, ignora aninhamento real, dá a toda caixa uma altura de uma linha, e o próprio
  doc-comment diz "B4 replaces this". É `Copy` e unit struct — vai deixar de ser as duas coisas ao passar a segurar um
  `TextMeasurer`.
- **`MockLayoutEngine`** (`core/css/src/infrastructure/mock.rs:47-84`): caixa `1×1 Au` por nó. Tem de continuar trocando
  sem nenhuma mudança no resto (critério de DoD e `PRD-007:94-95`).
- **`SUPPORTED_PROPERTIES` + `MANIFEST.md` + `manifest_runner.rs`**: o portão bidirecional de B1. Hoje 14 propriedades;
  `width`, `border` e `flex-direction` estão na lista `REFUSED_PROPERTIES` do runner
  (`core/css/tests/manifest_runner.rs:135-142`) — B4 as move de "recusada" para "suportada" e o runner falha
  ruidosamente se manifesto, registry e parser divergirem em qualquer direção.
- **`CssStage::Layout` / `CssStage::Measure`** (`core/css/src/domain/error.rs:24-30`): os estágios já existem; `Layout`
  já é usado pelo placeholder. Nenhum estágio novo é necessário.
- **`graphics::Au` / `Point` / `Size` / `Rect`** (`ADR-0016`): `Size::new` devolve `Option` (recusa lado negativo), `Au`
  tem `saturating_add`/`saturating_sub`/`larger`/`smaller`/`checked_mul` — a aritmética de layout é inteira e
  **saturante**, porque `arithmetic_side_effects` é `deny` no workspace.

### New Concepts Required

- **`Sizing`** — o valor computado de `width` / `height` / `flex-basis`: `Auto` ou uma `Length`. Sem primitivo cru; é o
  VO que falta entre `Length` e "a caixa decide sozinha".
- **`BoxSizing`** — `ContentBox` / `BorderBox`. Decide se a `width` declarada mede o conteúdo ou a borda.
- **`TextAlign`** — `Left` / `Right` / `Center` / `Justify`, aplicado por **caixa de linha**.
- **`WhiteSpace`** — `Normal` / `Pre` / `NoWrap`: o par (colapsa espaço?, permite quebra suave?) que governa a
  segmentação do texto.
- **`FlexStyle`** — agregado de valor com as nove propriedades de flex (`FlexDirection`, `FlexWrap`, `JustifyContent`,
  `AlignItems`, `AlignContent`, `AlignSelf`, `FlexFactor` para `grow`/`shrink`, `Sizing` para `basis`). Agrupar evita
  transformar `ComputedStyle` num registro de 25 campos e mantém "entidades pequenas".
- **`IntrinsicSize`** — o marcador do passo 4: `Resolved` ou `Pending`. Diz que a caixa depende de um recurso ainda não
  carregado (`<img>` sem dimensões declaradas). Nasce no `StyledNode` (é onde a tag ainda é conhecida) e é copiado para
  o `LayoutBox`, que é quem a Fase X vai ler.
- **`CollapsedMargin`** — o par `(maior positivo, menor negativo)` que é a álgebra do colapso de margem vertical (CSS
  2.1 §8.3.1). Um VO com `adjoin` associativo e comutativo e um `resolve() -> Au`.
- **`BoxMetrics`** — as arestas resolvidas (`margin`/`border`/`padding`) mais as restrições de tamanho já convertidas
  para `Au`, o produto de "resolver o box model de um nó" antes de saber onde ele fica.
- **`Fragment`** — uma caixa posicionada **relativamente** à origem da sub-árvore que a produziu. É o que permite cada
  contexto de formatação devolver um resultado auto-contido que o pai translada, em vez de um `cursor_y` global.
- **`LineBox`** / **`InlineItem`** — a caixa de linha do IFC e o item (uma palavra medida, ou uma caixa inline aninhada)
  que ela empilha.
- **`FlexLine`** / **`FlexItem`** / **`MainAxis`** — a linha de flex e o item, mais a abstração de eixo que faz
  `row`/`column` (e as variantes `-reverse`) percorrerem o **mesmo** código.

### Key Business Rules

- **Colapso de margem vertical** (CSS 2.1 §8.3.1) governa `CollapsedMargin`, `BoxMetrics` e a ordem de empilhamento: (a)
  a margem inferior de um irmão colapsa com a superior do próximo; (b) a margem superior do pai colapsa com a do
  primeiro filho em fluxo **se** o pai não tiver borda nem padding no topo; (c) a inferior do pai colapsa com a do
  último filho **se** o pai não tiver borda nem padding embaixo **e** tiver `height: auto`.
- **`box-sizing`** (CSS Box Sizing L3 §5) governa `BoxMetrics`: em `border-box` a largura de conteúdo é
  `width − border.horizontal() − padding.horizontal()`, nunca negativa.
- **Colapso de espaço em branco** (CSS Text L3 §4.1.1) governa `WhiteSpace`: em `normal` e `nowrap` cada corrida de
  espaço vira um único espaço; em `pre` nada é colapsado e `\n` é uma quebra forçada.
- **Quebra suave** governa `LineBox`: só em `white-space: normal`; a oportunidade de quebra é o espaço entre palavras;
  uma palavra sozinha maior que a linha **transborda** em vez de ser partida.
- **`text-align: justify`** governa a distribuição de folga: o espaço extra vai para os intervalos entre palavras, e a
  **última linha** de um bloco nunca é justificada (CSS Text L3 §7.3).
- **Determinismo** (`PRD-007:79-80`, `:100`) governa tudo: nenhum `HashMap` iterado, nenhuma ordenação instável, nenhum
  `f32` na posição final — só `Au`. A mesma `StyledTree` e o mesmo `ViewportConstraints` produzem a mesma
  `LayoutBoxTree` byte a byte, 100 vezes.
- **Sem tipo estrangeiro** (`PRD-007:83-84`) governa a fronteira: o marcador de intrinsic size, o texto que atravessa
  para o `StyledTree` e as métricas de linha são todos tipos de `css` (ou as unidades compartilhadas de `graphics`
  autorizadas por `ADR-0016`).
- **O recorte é declarado** (`§2.8:350-354`) governa o parser: toda propriedade nova tem linha no `MANIFEST.md`, entrada
  em `SUPPORTED_PROPERTIES`, probe no `manifest_runner` e aceitação real no parser — ou o CI fica vermelho nos dois
  sentidos.

## Strategic Approach

### Solution Direction

Substituir o `BlockLayout` plano por um motor **recursivo por contexto de formatação**, com resultados relativos que o
pai translada:

1. `infrastructure/layout/box_model.rs` resolve as arestas e as restrições de tamanho de um nó (`BoxMetrics`).
2. `infrastructure/layout/margin_collapse.rs` isola a álgebra `CollapsedMargin` — é a única parte do colapso que precisa
   ser provada isoladamente e é o "passo 1" que o spec manda escrever primeiro.
3. `infrastructure/layout/block.rs` implementa o `LayoutEngine` e o contexto de formatação de bloco; escolhe, por caixa,
   entre BFC (filhos de nível bloco), IFC (filhos inline/texto) e flex (`display: flex`).
4. `infrastructure/layout/inline.rs` e `flex.rs` são os outros dois contextos, com a mesma assinatura de resultado
   (`Fragment`s relativos + altura de conteúdo), o que mantém `block.rs` pequeno e faz o despacho ser um `match` de três
   braços.

O fluxo de dados não muda em nada na fronteira: continua `&StyledTree` + `&ViewportConstraints` → `LayoutBoxTree`. O que
muda é o **conteúdo** dos agregados, e é por isso que esta é a fase que fecha o freeze I3.

### Key Design Decisions

- **Texto no `StyledTree`**: sem ele o IFC é impossível (o motor só recebe `&StyledTree`). Trade-off: engorda um
  agregado de fronteira **versus** passar `&DomSnapshot` também para `layout` — o que quebraria a assinatura congelada
  de `PRD-007:56-60` e daria ao motor de layout acesso de leitura ao DOM inteiro. → **Guardar `Option<TextRun>` no
  `StyledNode`**, preenchido por `recompute_in_document_order` a partir do `NodeRef`, sem tocar na assinatura do closure
  que os três resolvers passam.
- **Onde nasce o marcador de intrinsic size**: o `LayoutEngine` não vê tags, logo não consegue decidir sozinho que uma
  caixa é `<img>`. Trade-off: (a) passar o snapshot para o layout — rejeitado, mesma razão acima; (b) pôr o marcador no
  `ComputedStyle` — rejeitado, não é uma propriedade CSS; (c) pôr no `StyledNode`, decidido pelo **construtor da
  árvore** (que vê o `NodeRef`) e não pelo resolver. → **(c)**: `IntrinsicSize::Pending` para as tags substituídas,
  copiado para o `LayoutBox` quando a caixa realmente não tem tamanho declarado. Nenhum dos três resolvers muda uma
  linha.
- **Agrupar as nove propriedades de flex num `FlexStyle`** em vez de nove campos soltos em `ComputedStyle`: mantém o
  agregado com 13 campos em vez de 21 e dá um lugar natural para `FlexStyle::initial()`. Custo: um salto a mais
  (`style.flex().direction()`), o mesmo comprimento de cadeia que `node.style().display()` já tem hoje.
- **`border` entra como `border-width` (atalho 1–4 componentes) + os quatro longhands**, não como o atalho completo
  `border: 1px solid red`: o layout só consome a **largura**; `border-style` e `border-color` são pintura, não
  geometria, e entrar com eles agora seria escrever cascata para algo que nenhum consumidor lê. `border` continua
  **recusado com nota** e o `MANIFEST.md` diz por quê.
- **Resultados relativos + translação**, em vez de duas passadas (medir de baixo para cima, posicionar de cima para
  baixo): metade do código para a mesma resposta, à custa de `O(n·profundidade)` translações. Para uma árvore de página
  real é irrelevante e o determinismo é trivialmente preservado (soma saturante de inteiros).
- **Recursão com teto explícito** em vez de work-stack: layout é naturalmente recursivo e uma stack explícita
  triplicaria o tamanho de cada contexto. O teto (`MAX_LAYOUT_DEPTH`) devolve `CssError::unsupported(Layout, …)` — a
  mesma disciplina de `MAX_NESTING_DEPTH` no parser (`rules.rs:31`), que já trata profundidade patológica como entrada
  hostil.
- **Flexbox multi-linha entra inteiro**, sem puxar a alavanca de alívio: o algoritmo §9.3-9.7 numa forma reduzida (base
  size → coleta em linhas → resolução de flexíveis → cross size por linha → `align-content` → `justify-content` →
  `align-items`/`align-self`) cabe num arquivo, porque a abstração de eixo faz `row` e `column` compartilharem tudo. O
  que **fica fora** e vai ao `MANIFEST.md`: `flex-basis: content`, `order`, `min-width`/`max-width` como restrições de
  flex, e o segundo passe de resolução com congelamento iterativo de itens que violam min/max.

### Alternatives Considered

- **Manter `BlockLayout` como está e escrever um segundo `LayoutEngine`**: rejeitado — o spec diz "substitui
  `BlockLayout`", e dois motores embutidos dobram a superfície do freeze I3 sem nenhum consumidor pedindo.
- **Passar `&DomSnapshot` para `LayoutEngine::layout`**: rejeitado — `PRD-007:56-60` fixa a assinatura, e o freeze I3
  desta mesma fase a tornaria imutável de qualquer jeito; melhor engordar o agregado (aditivo, versionado) do que a
  porta.
- **`f32` para posições intermediárias, arredondando só no fim**: rejeitado por `ADR-0016` — a promessa de golden byte a
  byte em três SOs depende de aritmética inteira em **todo** o caminho, não só na saída.
- **Alavanca de alívio (`flex-wrap` fora)**: mantida como plano B explícito, não usada. Se tivesse sido usada, a linha
  no `MANIFEST.md` teria de dizer o corte e apontar a v0.7 — o spec proíbe cortar em silêncio.
- **`TextMetrics` sem baseline, alinhando pelo topo**: rejeitado — "alinhamento por baseline simples" está no spec, e
  alinhar pelo topo faz texto de tamanhos diferentes na mesma linha ficar visivelmente errado. `TextMetrics` ganha um
  `baseline` com um construtor que preserva o comportamento anterior por default.

## Risk & Gap Analysis

### Requirement Ambiguities

- **"quebra suave (espaço + pós-hífen, UAX #14 simplificado)"**: o spec do plano cita pós-hífen; a versão curta do brief
  da fase pede só "por espaço". → Implementar a oportunidade de quebra **depois de um hífen** além do espaço é barato e
  está no texto original, então entra; UAX #14 completo (classes de quebra, CJK) fica declarado fora.
- **"alinhamento por baseline simples"**: não diz o que é "simples". → Baseline = `TextMetrics::baseline()`, com todos
  os itens da linha alinhados nessa distância a partir do topo da linha; a linha é alta o bastante para o maior
  ascendente mais o maior descendente. Sem `vertical-align`.
- **"marcador de intrinsic size pendente (um campo/variante)"**: não diz em qual agregado nem quem o produz. → Decidido
  acima (`StyledNode` produz, `LayoutBox` expõe); o contract record registra a decisão para a Fase X.
- **`border` no passo 1**: "margin/border/padding" não diz se é o atalho completo. → Só a largura entra, e o
  `MANIFEST.md` declara o resto fora.

### Edge Cases

- **Margem negativa**: `CollapsedMargin` tem de somar `max(positivos) + min(negativos)`, não `max` de tudo — um
  `margin-bottom: -10px` seguido de `margin-top: 4px` colapsa para `-6px`.
- **Caixa que colapsa através** (altura 0, sem borda, sem padding, `height: auto`): as duas margens tornam-se
  adjacentes. Tratada; a interação exata quando essa caixa é o **primeiro** filho fica declarada como simplificação no
  `MANIFEST.md`.
- **`width` maior que o contêiner**: transborda, não é clampado — clampar seria inventar `max-width`.
- **`box-sizing: border-box` com `width` menor que `border + padding`**: largura de conteúdo satura em zero, nunca
  negativa (`Size::new` recusaria).
- **Palavra única maior que a linha**: transborda a linha em vez de ser partida (não há hifenização).
- **`white-space: pre` com `\n` final**: gera uma última linha vazia com a altura da fonte, não uma linha ausente.
- **Contêiner flex vazio**: altura de conteúdo zero, `justify-content` sem efeito, nenhuma caixa filha — não pode
  dividir por zero ao distribuir folga (`space-between` com 1 item, `space-around` com 0 itens).
- **`flex-shrink` com soma de fatores zero**: nada encolhe; os itens transbordam. Divisão por zero evitada por guarda.
- **Documento vazio** (só o nó `Document`): já coberto por `check_an_empty_document_is_handled` na suíte de
  conformidade; tem de continuar passando.
- **Nó `display: none` com filhos**: a sub-árvore inteira não gera caixa — a poda de `BlockLayout` já faz isso e a
  recursão preserva a regra naturalmente (nem se desce).
- **Profundidade patológica**: teto de recursão devolve erro tipado em vez de estourar a pilha.

### Technical Risks

- **`arithmetic_side_effects = deny`**: toda soma de `Au` tem de ser `saturating_*`/`checked_*`, incluindo a divisão de
  folga em `justify-content` e `text-align: justify`. → Concentrar a aritmética de distribuição em funções pequenas com
  `checked_div` + guarda de zero.
- **`as_conversions = deny`**: converter `usize` (contagem de palavras/itens) para `i32` (fator de `Au`) precisa de
  `i32::try_from` com erro tipado. → Já é o padrão de `MonospaceMetrics`.
- **Explosão de `SUPPORTED_PROPERTIES`** (14 → ~33): cada propriedade nova exige quatro edições coordenadas (registry,
  manifesto, probe, parser+cascata). → Rodar `cargo test -p css --test manifest_runner` a cada bloco de propriedades,
  não no fim.
- **Probe do `manifest_runner` exige que a propriedade **mude** o `ComputedStyle` do parágrafo-fixture**: uma
  propriedade cujo valor de probe coincida com o inicial passa despercebida como "ignorada". → Escolher valores de probe
  distintos do `initial()` para cada uma das novas.
- **`BlockLayout` deixa de ser `Copy`** ao segurar um `Arc<dyn TextMeasurer>`: `css_conformance.rs` e `pipeline.rs`
  passam `&BlockLayout::new()`, o que continua compilando; mas `#[derive(Clone, Copy, Default)]` some. → Verificado que
  nenhum crate fora de `core/css` nomeia `BlockLayout` hoje.
- **Regressão do golden `boxes.png`**: `core/css` não tem golden próprio (só `core/graphics` tem, e ele não consome
  `css`). → Verificar antes de qualquer regen; se nada consome, **não** existe churn de golden nesta fase e nenhum
  commit de `*.png` é criado.
- **Ordem de `boxes_in_document_order`**: o consumidor de I2 vai depender dela. → Emitir fragmentos em pré-ordem (pai
  antes dos filhos, filhos em ordem de documento) e afirmar isso num teste.

### Acceptance Criteria Coverage

| AC# | Descrição                                                                   | Endereçável? | Lacunas / notas                                                                         |
| --- | --------------------------------------------------------------------------- | ------------ | --------------------------------------------------------------------------------------- |
| 1   | Box model completo (`margin`/`border`/`padding`, `box-sizing`) com colapso  | Sim          | `border` entra como largura apenas; `border-style`/`border-color` declarados fora       |
| 2   | Asserções de retângulo: 3+ casos de colapso, `content-box` vs `border-box`  | Sim          | irmãos, pai/primeiro-filho, pai/último-filho, mais margem negativa                      |
| 3   | IFC: caixas de linha, `white-space`, colapso de espaço, quebra suave        | Sim          | UAX #14 completo fora; quebra em espaço e pós-hífen dentro                              |
| 4   | `text-align` `left`/`right`/`center`/`justify`, inlines aninhados, baseline | Sim          | última linha nunca justificada; sem `vertical-align`                                    |
| 5   | Flexbox: as nove propriedades, uma asserção por propriedade                 | Sim          | multi-linha entra; `order`, `flex-basis: content` e min/max iterativo declarados fora   |
| 6   | Marcador de intrinsic size pendente no `LayoutBoxTree`, antes do freeze     | Sim          | nasce no `StyledNode`; a Fase X substitui o tamanho, não o marcador                     |
| 7   | Propriedades novas em `SUPPORTED_PROPERTIES` + `MANIFEST.md` + parser       | Sim          | `manifest_runner` prova nos dois sentidos; `width`/`border`/`flex-*` saem de "recusada" |
| 8   | Freeze I3 + `docs/architecture/style-cascade-port-contract.md` (7 itens)    | Sim          | molde `runtime-engine-port-contract.md`                                                 |
| 9   | Bump de `css::PORT_SCHEMA_VERSION` com motivo no doc-comment                | Sim          | 2 → 3; `StyledNode` ganhou texto + marcador, `LayoutBox` ganhou borda + marcador        |
| 10  | Determinismo de 100 runs do `LayoutBoxTree` completo                        | Sim          | página com colapso + inline + flex, molde `core/graphics/tests/text_rendering.rs`       |
| 11  | `MockLayoutEngine` ainda troca sem tocar no resto                           | Sim          | `css_conformance.rs` já prova; nenhuma mudança nele                                     |
| 12  | `--no-default-features` verde                                               | Sim          | nenhum adaptador novo é gated por feature                                               |
| 13  | Commit de churn de golden isolado                                           | N/A          | `core/css` não tem golden e `core/graphics` não consome `css` — nada a rebençoar        |
