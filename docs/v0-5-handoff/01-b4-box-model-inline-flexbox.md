# B4 — box model, contexto de formatação inline, Flexbox

## Contexto

`core/css` tem, desde B0–B3, as portas (`CascadeResolver`/`LayoutEngine`/`TextMeasurer`), o parser CSS completo, a
cascata real de três origens e um `TextMeasurer` real (`FontBackedMeasurer`). O que falta é o motor de layout de
verdade: `BlockLayout` hoje (antes de B4) só empilhava blocos sem colapsar margem, sem `box-sizing`, sem quebra de linha
e sem Flexbox. B4 substitui isso por um motor completo, e **é o bloqueador de tudo mais** — B5, X, I2, M, I4 e P todos
pressupõem um `core/css` que compila e passa nos seus próprios testes.

Um sub-agente já escreveu boa parte desta fase e morreu por limite de sessão da conta antes de terminar. O trabalho
**não foi descartado** — está no working tree da branch `feat/v0-5`, não commitado. Este arquivo descreve exatamente o
que já está pronto e o que falta, com evidência verificada nesta sessão.

## Estado atual

### O que já está escrito, revisado e correto

Estes arquivos foram lidos linha a linha nesta sessão e seguem o estilo do resto do crate (Object Calisthenics, sem
`unwrap`/`else`, um dot por linha, `#[non_exhaustive]` nos enums):

- `core/css/src/domain/computed/flex.rs` (454 linhas) — os nove _value objects_ do Flexbox (`FlexDirection`, `FlexWrap`,
  `JustifyContent`, `AlignItems`, `AlignContent`, `AlignSelf`, `FlexFactor`, agrupados em `FlexStyle`). Completo.
- `core/css/src/domain/computed/{inline_style,intrinsic,sizing}.rs` — `TextAlign`/`WhiteSpace`, `IntrinsicSize` (o
  marcador "ainda depende de um recurso" que a Fase X vai consumir), `Sizing`/`BoxSizing`. Completos.
- `core/css/src/domain/computed/style.rs` — `ComputedStyle` ganhou `border`, `width`, `height`, `box_sizing`,
  `text_align`, `white_space`, `flex`. `inheriting_from` herda só o que a CSS realmente herda (`color`, `font-size`,
  `text-align`, `white-space`). Completo.
- `core/css/src/domain/layout_box_tree.rs` — `BoxEdges` (agrupa margin/border/padding num só parâmetro),
  `LayoutBox::new` com a assinatura nova (`node, content, edges: BoxEdges, intrinsic_size: IntrinsicSize, children`).
  Completo.
- `core/css/src/domain/styled_tree.rs` — `StyledNode` ganhou `text: Option<TextRun>` e `intrinsic_size: IntrinsicSize`,
  preenchidos em `recompute_in_document_order`. Completo.
- `core/css/src/domain/text.rs` — `TextMetrics` ganhou `baseline`/`descent`, para alinhamento por linha de base.
  Completo.
- `core/css/src/infrastructure/layout/box_model.rs` (174 linhas) — `BoxMetrics`, resolve `box-sizing`,
  margem/borda/padding contra o _containing block_. Completo.
- `core/css/src/infrastructure/layout/margin_collapse.rs` (97 linhas) — `CollapsedMargin`,
  `collapses_at_top`/`collapses_at_bottom` (CSS 2.1 §8.3.1). Completo.
- `core/css/src/infrastructure/layout/fragment.rs` (121 linhas) — `Fragment`/`Fragments`, sempre posicionados
  **relativos** à origem que o chamador vai transladar. Completo.
- `core/css/src/infrastructure/layout/context.rs` (288 linhas) — `LayoutContext`, `BlockInput`, `BlockResult`,
  `ContentFlow`. **Tem um bug real** (seção seguinte).
- `core/css/src/infrastructure/layout/inline.rs` (694 linhas) — o contexto de formatação inline completo: segmentação de
  palavras, colapso de espaço em branco, `white-space: pre`/`nowrap`, quebra suave por espaço e por hífen, `text-align`
  incluindo `justify`, alinhamento por linha de base. Completo, mas **chamado com a aridade errada** em `block.rs`
  (seção seguinte).
- `core/css/src/infrastructure/cascade/values.rs` e `flex_values.rs` — todas as propriedades de box model, inline e
  Flexbox já são aceitas por `apply_declaration`/`reset_to_initial`/`inherit_property`. Completo.
- `core/css/src/infrastructure/parser/values.rs` — `parse_box_sizing`, `parse_sizing`, `parse_text_align`,
  `parse_white_space`, `parse_flex_direction`, `parse_flex_wrap`, `parse_justify_content`, `parse_align_items`,
  `parse_align_content`, `parse_align_self`, `parse_flex_factor` — todos escritos. Completo.
- `spdd/analysis/202609050900-[Analysis]-css-box-model-inline-context-and-flexbox-v0-5-b4.md` e o par em `spdd/prompt/`
  — o canvas SPDD da fase já foi produzido. Não precisa refazer.

### O que falta para compilar

`cargo build -p css --all-targets` reporta **4 erros**, em três categorias:

1. **Módulo ausente.** `core/css/src/infrastructure/layout/block.rs:38` importa
   `crate::infrastructure::layout::{flex, inline}`, mas `infrastructure/layout/flex.rs` **não existe**:

    ```bash
    ls core/css/src/infrastructure/layout/flex.rs
    # → No such file or directory
    ```

    `block.rs:361-363` já chama o que se espera dele:

    ```rust
    if display_of(node) == Display::Flex {
        return flex::layout(context, node, content_width, font_size, input);
    }
    ```

    A assinatura a implementar é exatamente essa:

    ```rust
    pub(crate) fn layout(
        context: &LayoutContext<'_>,
        node: &StyledNode,
        content_width: Au,
        font_size: Au,
        input: BlockInput,
    ) -> Result<ContentFlow, CssError>
    ```

    Ver "Passos" abaixo para o algoritmo.

2. **Argumento faltando.** `block.rs:511` chama
   `inline::layout(context, items, flowing.content_width, flowing.font_size)` com 4 argumentos, mas `inline.rs:95-101`
   pede 5 — falta `align: TextAlign` no final. Correção:
    - Adicionar um campo `align: TextAlign` à struct `Flowing` (`block.rs:436-454`).
    - Em `layout_content` (`block.rs:354-369`), ler `node.style().text_align()` e passar para `stack_segments`.
    - Encadear esse valor até `Flowing::new` e até a chamada em `absorb_inline` (`block.rs:505-521`).

3. **`const fn` inválida (`E0493`).** `context.rs:253` (`ContentFlow::with_margins`) e `context.rs:265`
   (`ContentFlow::with_flow`) são `const fn` que consomem `self` por valor. `ContentFlow` carrega
   `fragments: Fragments`, que embrulha um `Vec<Fragment>` — um tipo com destrutor não pode ser parâmetro de uma função
   `const` no Rust estável. Correção: remover a palavra `const` das duas assinaturas (nenhum chamador as invoca em
   contexto `const` — confirmado por leitura de todos os usos em `block.rs` e `inline.rs`).

Depois desses quatro, rode `cargo build -p css --all-targets` de novo — pode haver mais erros que só aparecem depois que
os primeiros são corrigidos (o compilador para cedo quando há erro de import).

## Passos

1. **Corrigir os três problemas de compilação acima**, na ordem: `const fn` (mais rápido), depois o argumento de
   `inline::layout`, depois escrever `flex.rs`.

2. **Escrever `core/css/src/infrastructure/layout/flex.rs`** — o algoritmo de Flexbox (CSS Flexbox L1 §9, simplificado):
    - Reunir os filhos em fluxo do container (mesma filtragem de `display: none` que `block.rs` já faz).
    - Resolver o _flex basis_ de cada item (`flex-basis` se não for `auto`; senão o tamanho de conteúdo, reaproveitando
      `layout_box`/`box_model::resolve` já existentes).
    - Distribuir o espaço livre por `flex-grow` (positivo) ou `flex-shrink` (negativo) — fatores em `FlexFactor` já
      existem em `domain/computed/flex.rs`; a conversão fração→`Au` deve ser feita com numerador/denominador inteiro,
      nunca ponto flutuante direto na geometria (ADR-0016).
    - Posicionar os itens na linha principal segundo `justify-content`.
    - Posicionar/esticar os itens no eixo transversal segundo `align-items`, com `align-self` sobrepondo por item.
    - **Válvula de alívio explícita** (já pré-aprovada no plano): se o algoritmo completo de quebra multi-linha
      (`flex-wrap: wrap`/`wrap-reverse`, CSS Flexbox L1 §9.4–9.7) se mostrar grande demais para esta fase, é aceitável
      entregar só flex de linha única. Nesse caso, documente a lacuna como uma linha em
      `core/css/tests/data/MANIFEST.md` explicando o corte e apontando para v0.7 — **não** simplifique silenciosamente
      sem essa linha.
    - Retornar um `ContentFlow` — os `Fragment`s de cada item, posicionados relativos à origem do container, exatamente
      como `block.rs`/`inline.rs` já fazem.
    - Assinatura final: a mesma listada em "Estado atual" acima, item 1.

3. **Registrar as 19 propriedades novas.** `core/css/src/lib.rs:73-88` (`SUPPORTED_PROPERTIES`) ainda tem só as 14 de
   B0/B1, mas `values.rs`/`flex_values.rs` já tratam 33 nomes. As 19 que faltam registrar:
    - Shorthands/singulares: `border-width`, `width`, `height`, `box-sizing`, `text-align`, `white-space` (6).
    - Longhands de borda: `border-top-width`, `border-right-width`, `border-bottom-width`, `border-left-width` (4).
    - Flexbox: `flex-direction`, `flex-wrap`, `justify-content`, `align-items`, `align-content`, `align-self`,
      `flex-grow`, `flex-shrink`, `flex-basis` (9).

    Mude o tipo do array para `[&str; 33]` e documente o motivo do salto no comentário da constante.

4. **Atualizar `core/css/tests/data/MANIFEST.md`** — acrescente as 19 linhas na tabela `## Properties` (coluna `since` =
   `B4`), e revise a linha "Declared out": remova `width`, `height`, `box-sizing`, `flex-direction` dela (agora
   suportados), mantendo `float`, `position`, `border` (a _shorthand_ cheia, não `border-width`), `z-index`.

5. **Atualizar `core/css/tests/manifest_runner.rs`**:
    - `PROPERTY_PROBES` (`manifest_runner.rs:76-91`) cresce de 14 para 33 entradas — uma por propriedade nova, com um
      valor que muda visivelmente o `ComputedStyle` em relação ao _default_ (mesmo padrão das 14 já existentes).
    - `REFUSED_PROPERTIES` (`manifest_runner.rs:112-119`) perde `"width"` e `"flex-direction"` (ficam
      `["float", "position", "border", "z-index"]`).
    - Rode `cargo test -p css --test manifest_runner` a cada propriedade adicionada — ele falha nas duas direções se
      manifesto, registro e parser divergirem.

6. **Escrever os testes de retângulo** que faltam (nenhum arquivo existe ainda — `ls core/css/tests/` não lista nada
   além dos arquivos de B0–B3). Sugestão de organização: um arquivo por sub-tema, no padrão dos testes já existentes
   (`#![allow(clippy::unwrap_used, clippy::expect_used)]`, DOM construído à mão via `dom::DomTree`):
    - `core/css/tests/box_model.rs` — colapso de margem (≥3 casos: irmãos adjacentes, pai/primeiro-filho,
      pai/último-filho) e `box-sizing` (`content-box` vs `border-box` no mesmo `width` declarado produzindo caixas de
      tamanho diferente).
    - `core/css/tests/inline_formatting.rs` — pelo menos um caso de overflow forçando quebra de linha, um de
      `text-align: justify`, um de `white-space: pre`.
    - `core/css/tests/flexbox.rs` — pelo menos uma prova por propriedade: `flex-direction` (row vs column muda o eixo em
      que os itens se espalham), `flex-wrap`, `justify-content` (as seis variantes distribuem o espaço livre de formas
      diferentes), `align-items`, `align-content`, `align-self`, `flex-grow`/`flex-shrink` (dois itens com fatores
      diferentes ocupam larguras proporcionais).

7. **Teste de determinismo** — 100 execuções idênticas do `LayoutBoxTree` completo contra uma página com colapso de
   margem + inline + Flexbox, no molde de `core/graphics/tests/text_rendering.rs` (tem um teste de determinismo de 100
   _runs_ que pode ser espelhado).

8. **Revisar `core/css/tests/pipeline.rs`** contra o motor novo — **verificado, ainda não corrigido**:
   `a_heading_is_laid_out_above_the_following_paragraph` (`pipeline.rs:89-91`) indexa `ordered.get(2)`/`ordered.get(3)`
   supondo que a ordem de documento das caixas é `[html, body, h1, p]` sem nada entre `h1` e `p`. Com o motor novo, o
   texto de `<h1>` e de `<p>` cada um gera seu **próprio** fragmento via `inline.rs` (a corrida de itens inline inclui o
   próprio nó de texto — `push_own_text` em `block.rs`), então a ordem real passa a ser
   `[html, body, h1, "Alloy", p, "First pixel"]` — os índices 2 e 3 do teste atual apontam para `h1` e para o texto de
   `h1`, não para `h1` e `p`. Corrija o teste para localizar as caixas por nó (`LayoutBoxTree::box_of`) em vez de por
   índice fixo.

9. **Bump de versão.** `ComputedStyle`, `StyledNode` e `LayoutBox` ganharam campos novos nesta fase —
   `css::PORT_SCHEMA_VERSION` (`core/css/src/lib.rs:63`) precisa subir de 2 para 3. Documente o motivo no comentário da
   constante, no molde de como `graphics::PORT_SCHEMA_VERSION` documentou 1→2 em B3.

10. **Congelamento I3.** Depois de tudo acima verde, escreva `docs/architecture/style-cascade-port-contract.md` cobrindo
    os 7 itens do ADR-0011 Replaceable Port Contract para `CascadeResolver`/`LayoutEngine`/`TextMeasurer` — use
    `docs/architecture/runtime-engine-port-contract.md` como molde de formato e nível de detalhe. A partir daqui,
    qualquer mudança de forma nos agregados de `core/css` exige nota de migração em `PRD-007`.

## Crates de referência

- `core/graphics` — porta completa: `Cargo.toml` `[features]`, `lib.rs` com `PORT_SCHEMA_VERSION` documentado, `domain/`
  com _newtypes_, `tests/text_rendering.rs` para o padrão de teste de determinismo.
- O próprio `core/css` já escrito em B0–B3 — siga o estilo que já está lá (não invente um novo padrão).

## Definition of Done

- [ ] `cargo build -p css --all-targets` limpo.
- [ ] `cargo clippy -p css --all-targets --all-features -- -D warnings` limpo.
- [ ] `cargo test -p css` — **todos** verdes, incluindo `manifest_runner` e `pipeline`.
- [ ] `cargo test -p css --no-default-features` verde (feature `no-script`).
- [ ] `cargo fmt --all -- --check` limpo.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` limpo (nada quebrado fora de `css`).
- [ ] `cargo test --workspace` — todas as suítes verdes.
- [ ] `just no-engine` verde.
- [ ] Testes de retângulo cobrindo: colapso de margem (≥3 casos), `box-sizing`, quebra de linha, uma prova por
      propriedade de Flexbox.
- [ ] `MockLayoutEngine` ainda troca sem tocar no resto (já deveria continuar funcionando — `infrastructure/mock.rs` foi
      corrigido nesta sessão para a nova assinatura de `LayoutBox::new`).
- [ ] Teste de determinismo (100 _runs_) verde.
- [ ] `docs/architecture/style-cascade-port-contract.md` escrito.
- [ ] `css::PORT_SCHEMA_VERSION == 3`.

## Convenção de commit

Um commit só, na branch `feat/v0-5`, sem _push_:

```text
feat(css): box model, inline formatting context, flexbox (v0.5 B4)

<corpo descrevendo o que mudou>

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
```

Depois do commit, atualize a tabela de estado em `docs/v0-5-handoff/README.md` (linha da fase B4) e delete este arquivo
**não** — mantenha como registro histórico do que foi feito, só marque o status na tabela do `README.md`.
