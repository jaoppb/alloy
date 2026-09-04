# SPDD Analysis — v0.5 B1 (`core/css`): parser CSS, seletores e especificidade

| Campo        | Valor                                                                                                          |
| ------------ | -------------------------------------------------------------------------------------------------------------- |
| Fase         | B1 do plano `~/.claude/plans/verifique-o-docs-reports-implementacao-d-fancy-dijkstra.md:402-418` (F9a)         |
| Realiza      | `PRD-007` §1 — o **parsing** fica em Rust nativo no `core/css`; só cascata e layout são portas substituíveis   |
| Recorte      | Tabela "Dentro / Fora" de `docs/reports/IMPLEMENTACAO-DETALHADA-V0-5.md:340-345`, declarada por `MANIFEST.md`  |
| Depende de   | B0 (`3ea4834`) — agregados de fronteira, as três portas, `manifest_runner` semente                             |
| Estado atual | `StyleSheetSet` existe mas nasce vazio; `UaCascade::resolve` **ignora** `sheets` (`ua_sheet.rs:32`); 0 seletor |

## Original Business Requirement

Seção **"## Fase B1"** do plano (`~/.claude/plans/verifique-o-docs-reports-implementacao-d-fancy-dijkstra.md:402-418`),
verbatim:

```text
## Fase B1 — F9a: parser CSS, seletores, especificidade (10–15 d)

- `core/css/src/infrastructure/parser/{tokenizer.rs, rules.rs, selectors.rs, media.rs, mod.rs}` —
  tokenizer CSS Syntax L3 (at-rules, blocos, funções, `url()`, strings, escapes, comentários); parser
  de regras/declarações populando `StyleSheetSet`; `@media` `min-width`/`max-width`.
- `core/css/src/domain/{selector.rs, specificity.rs}` — `Selector` VO; `Specificity(u16,u16,u16)`;
  matching contra `DomSnapshot` com ordem de documento como desempate.
- Suporte de seletor exatamente como a tabela §2.8 do relatório (tipo/universal/`.classe`/`#id`/
  `[attr]`/`[attr=v]`/listas; `>`/`+`/`~`/descendente; `:hover`/`:active`/`:focus`/`:first-child`/
  `:last-child`/`:nth-child()`). `<style>`, `style=` ligados na construção de `StyleSheetSet`.
- `core/css/tests/data/MANIFEST.md` — recorte propriedade a propriedade, seletor a seletor;
  `manifest_runner.rs` força nos dois sentidos.

**Entregável:** recorte do MANIFEST 100% verde; `<style>`/`style=` aplicados de forma observável via
`CascadeResolver`.
```

Recorte de seletores de `docs/reports/IMPLEMENTACAO-DETALHADA-V0-5.md:336-345`, verbatim:

```text
`infrastructure/parser/` — tokenizador de CSS Syntax Level 3 (o suficiente para folhas reais:
_at-rules_, blocos, funções, `url()`, strings, escapes, comentários) e o parser de regras que popula o
`StyleSheetSet` que já existe. Entram `<style>`, `style=` e `<link rel=stylesheet>` (subrecurso, via §2.11).

Seletores, com especificidade de três componentes e ordem de origem/documento como desempate:

| Dentro                                                                       | Fora, e por quê                                            |
| ---------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Tipo, universal, `.classe`, `#id`, `[attr]` e `[attr=v]`, listas             | `:has()` — exige _matching_ reverso, custo desproporcional |
| Combinadores descendente, `>`, `+`, `~`                                      | Namespaces — sem conteúdo estrangeiro até a v1.0           |
| `:hover`, `:active`, `:focus`, `:first-child`, `:last-child`, `:nth-child()` | `::before`/`::after` — geram caixa sem nó; v0.7            |
| `@media` com `min-width`/`max-width`                                         | `@supports`, `@font-face`, `@import`, `@keyframes`         |
```

`docs/reports/IMPLEMENTACAO-DETALHADA-V0-5.md:350-354`, verbatim:

```text
O recorte acima é **declarado por manifesto**, no molde do recorte html5lib da v0.3
(`IMPLEMENTACAO-DETALHADA-V0-3.md:299-309`): `core/css/tests/data/MANIFEST.md` lista propriedade a
propriedade e seletor a seletor o que a v0.5 suporta, e o runner falha se o código suportar algo não
listado ou listar algo não suportado. É o que impede o recorte de encolher em silêncio para o CI ficar
verde.
```

`PRD-007` (`docs/requirements/PRD-007-style-cascade-and-layout-engine-ports.md`), §1 e §3.1 verbatim (`PRD-007:11-16`,
`:37-38`):

```text
CSS **parsing** (tokenizer, selector syntax, rule sets) stays native Rust in `core/css`. The **cascade
resolution** and **layout** stages of the pipeline are exposed as replaceable ports, so an engine
developer can substitute a custom specificity/inheritance resolver or a custom layout algorithm — in
Rust, or as a `.rhai` / Wasm adapter driven through `RuntimeEngine` — without modifying `core/dom`,
`core/graphics`, or any consumer.

- `StyleSheetSet` — parsed, ordered rules with origin (`UserAgent`, `User`, `Author`). Produced by the
  native Rust parser; not replaceable in this PRD.
```

`PRD-007:76-87` (invariantes) e `PRD-001:96` (orçamento de hook), verbatim:

```text
1. **No per-node callbacks** cross the seam; the unit of exchange is the whole tree.
2. **Determinism**: the same `DomSnapshot` + `StyleSheetSet` yields a byte-identical `LayoutBoxTree`…
4. **No foreign types**: no `core/dom` or `core/graphics` internal type appears in a port signature or
   a boundary aggregate.
```

## Domain Concept Identification

### Existing Concepts (from codebase)

- **`StyleSheetSet` / `StyleRule` / `DeclarationBlock` / `Origin`** (`core/css/src/domain/stylesheet_set.rs:11-156`): o
  andaime honesto de B0 — `StyleRule` guarda `selector_text: String` (`:92`) e um `DeclarationBlock` de pares
  `(String, String)` (`:55`). O doc-comment do arquivo (`:4-7`) já declara: "B1 replaces these with parsed selector and
  declaration types **without changing the aggregate's shape**". `Origin::precedence()` (`:27`) já ordena UA(0) <
  User(1) < Author(2).
- **`DomSnapshot` / `NodeRef` / `SnapshotId` / `AttributeList`** (`core/css/src/domain/dom_snapshot.rs:171-275`): o alvo
  de _matching_. `NodeRef` já expõe `tag()` (lowercased, `:231`), `attribute(name)` (`:237`), `attributes()` (`:243`),
  `parent()` (`:257`), `children()` (`:262`), `kind()` (`:224`). Não há `previous_sibling`/`next_sibling` — irmãos se
  obtêm pelo `children()` do pai. `SnapshotId` é pré-ordem DFS (`:13-16`), então `0..len` é ordem de documento e todo
  `parent_id < child_id`.
- **`UaCascade`** (`core/css/src/infrastructure/ua_sheet.rs:31-35`): `resolve(&self, dom, _sheets)` — **ignora** o
  `StyleSheetSet` por completo hoje. Aplica regras UA por tag (`:55-82`) e herda `color`/`font-size` via
  `ComputedStyle::inheriting_from` (`computed/style.rs:51`).
- **`StyledTree::recompute_in_document_order`** (`core/css/src/domain/styled_tree.rs:87-108`): a passada única
  pai-antes-de-filho que qualquer cascata usa; recebe `FnMut(NodeRef, Option<&ComputedStyle>) -> ComputedStyle`.
- **`ComputedStyle`** (`core/css/src/domain/computed/style.rs:22-29`): exatamente 6 campos — `display`, `color`,
  `background_color`, `margin`, `padding`, `font_size` — com `with_*` (`:60-90`) no estilo _copy-with_. É o que
  `SUPPORTED_PROPERTIES` (`core/css/src/lib.rs:64`) espelha.
- **`Length`** (`core/css/src/domain/length.rs:25-38`): soma `px`/`em`/`rem`/`%`/`pt`. **Já** é o VO que o parser de
  valores tem de produzir; `resolve_to_au` (`:70`) é a única travessia para `Au`.
- **`CssColor`** (`core/css/src/domain/color.rs:15`) — `rgb`/`rgba` `const fn`, embrulha `graphics::Color`.
- **`CssError` / `CssStage` / `SourceSpan`** (`core/css/src/domain/error.rs:16-108`): `CssStage::Parse` e
  `CssStage::Selector` **já existem** e nunca foram usados (`:17-20` os documenta como "B1").
  `SourceSpan::new(line, column)` 1-based, `0` = desconhecido (`:48`). `CssError::unsupported(stage, detail)` +
  `with_span` (`:150`) são os construtores prontos.
- **`manifest_runner.rs` + `MANIFEST.md`** (`core/css/tests/manifest_runner.rs:1-154`,
  `core/css/tests/data/MANIFEST.md`): o padrão bidirecional de B0 — hoje lê **itens de lista** (`:49-56`) sob as seções
  `## Properties` e `## Selectors`, e compara com `SUPPORTED_PROPERTIES` / `SUPPORTED_SELECTORS`. `UPDATE_MANIFEST`
  (`:19`, `:98`) é o _blessing_ no molde de `UPDATE_GOLDEN` (`core/graphics/src/infrastructure/golden.rs`).
- **Disciplina de travessia não-recursiva** (`core/css/src/application/snapshot.rs:22-31`,
  `core/dom/src/application/serialize.rs`): work-stack explícita, nunca auto-chamada. É o molde para o _matching_
  right-to-left com backtracking.
- **VOs validados com `const fn is_forbidden`** (`core/dom/src/domain/tag_name.rs`,
  `core/dom/src/domain/attributes.rs`): o molde de newtype validado + lowercase que os identificadores CSS seguem.
- **Portão de clippy do workspace** (`Cargo.toml:67-82`): `string_slice`, `indexing_slicing`, `arithmetic_side_effects`,
  `as_conversions`, `unwrap_used`, `expect_used`, `panic` todos `deny`, mais `pedantic`+`nursery`. Um tokenizador
  escrito à mão vive inteiramente dentro desse portão.

### New Concepts Required

- **`Token` / `SpannedToken` / `TokenStream`** — o vocabulário do tokenizador CSS Syntax L3: `Ident`, `AtKeyword`,
  `Hash`, `String`, `BadString`, `Function`, `Url`, `BadUrl`, `Number`, `Dimension`, `Percentage`, `Delim`,
  `Whitespace`, `Colon`, `Semicolon`, `Comma`, e os seis delimitadores de bloco. Cada token carrega o `SourceSpan` do
  seu primeiro caractere, para o erro tipado ter localização (`ADR-0011:93-95`).
- **`Scanner`** — o cursor de caracteres com linha/coluna. Existe porque o portão de clippy proíbe `s[i..j]`
  (`string_slice`) e `chars[i]` (`indexing_slicing`): a leitura é `Vec<char>` + `position` + `.get()` +
  `saturating_add`.
- **`Identifier`** — newtype validado de identificador CSS (nome de tipo, classe, id, atributo, propriedade). Um único
  VO reusado em cinco posições, no molde de `dom::TagName`.
- **`Selector`** — a família de VOs do recorte §2.8: `SelectorList` (coleção de primeira classe de `ComplexSelector`),
  `ComplexSelector` (sequência de `SelectorStep { combinator, compound }`), `CompoundSelector` (tipo/universal +
  coleções de classes, ids, atributos e pseudo-classes),
  `Combinator { Descendant, Child, NextSibling, SubsequentSibling }`, `TypeSelector { Universal, Named(Identifier) }`,
  `AttributeSelector` + `AttributeMatch { Exists, Exact }`,
  `PseudoClass { Hover, Active, Focus, FirstChild, LastChild, NthChild(NthFormula) }`, `NthFormula { a, b }` com
  `matches(index_one_based)`.
- **`Specificity`** — tripla `(ids, classes, types)` em `u16` com `Ord` derivado na ordem dos campos, exatamente o
  desempate da cascata. `*` contribui `0`.
- **`MediaQuery` / `MediaCondition` / `MediaFeature`** — `min-width`/`max-width` em `Length`;
  `matches(&ViewportConstraints) -> bool`; `MediaQuery::ALWAYS` (conjunto vazio de condições) para a regra fora de
  `@media`.
- **`Declaration` / `DeclarationValue` / `Importance`** — a declaração deixa de ser um par de `String` cru e vira VO:
  propriedade (`Identifier`), valor (`DeclarationValue`), e `Importance { Normal, Important }` — porque descartar
  `!important` em silêncio seria mudança semântica muda (B2 consome a flag).
- **`ParseNote` / `ParseNotes`** — o registro tipado de tudo que o parser **recuperou**: at-rule desconhecida,
  declaração descartada, seletor fora do recorte, `BadString`/`BadUrl`. É o que impede o "silent drop of the rest of the
  sheet".
- **`parse_stylesheet(source, origin)` / `parse_inline_style(source)`** — as duas entradas públicas do parser.
- **`matches(&ComplexSelector, NodeRef, &DomSnapshot) -> bool`** — _matching_ right-to-left, não-recursivo, com
  backtracking por work-stack.
- **`collect_style_sheets(&DomSnapshot) -> Result<StyleSheetSet, CssError>`** — colhe `<style>` e `style=` do próprio
  snapshot (não do `dom::DomTree`), preservando `snapshot.rs` como o único arquivo que nomeia `core/dom`.
- **Aplicação por especificidade na cascata** — `UaCascade` passa a consultar o `StyleSheetSet`: ordena as regras
  casadas por `(Origin::precedence, Specificity, índice de documento)` e aplica as declarações em ordem crescente, com o
  bloco inline por último.
- **`MANIFEST.md` em tabela** — duas seções (`## Properties`, `## Selectors`) com linhas `| token | since | notes |`, e
  um `manifest_runner` que confere três coisas: manifesto ⇄ registros, registros ⇄ **o que o parser aceita de fato**
  (sonda por entrada), e a rejeição de uma forma não-listada.

### Key Business Rules

- **Parsing é Rust nativo, não porta** (`PRD-007:11-13`): nada do parser aparece numa assinatura de porta; o produto do
  parser é o `StyleSheetSet`, que já é agregado de fronteira. Governa a localização de todo módulo novo
  (`infrastructure/parser/`).
- **O recorte é declarado, não descoberto** (`relatório §2.8:350-354`): o que está fora tem de ser **rejeitado ou
  marcado como não-suportado**, nunca ignorado em silêncio. Governa `ParseNotes`, o `manifest_runner` e a decisão de
  `:has()` / `::before` / `@supports` / namespaces.
- **Recuperação, não abandono** (CSS Syntax L3 §5.4.1, plano B1): uma regra malformada consome até o `}` que a fecha,
  uma declaração malformada até o `;`, e o resto da folha continua sendo parseado. Governa `rules.rs`.
- **Especificidade + origem + ordem de documento** (`relatório §2.8:334`, `PRD-007:38`): o desempate é
  `(origem, especificidade, ordem)`, nessa ordem. Governa `Specificity: Ord` e o laço de aplicação da cascata.
- **`resolve` é puro e determinístico** (`PRD-007:52`): parsear dentro de `resolve` seria determinístico, mas
  `PRD-007:37-38` diz que o `StyleSheetSet` é **produzido pelo parser**, antes da porta. Governa `collect_style_sheets`
  ser `application/`, não `infrastructure/cascade/`.
- **Nenhum callback por nó cruza a fronteira** (`PRD-007:78`, `PRD-001:96`): o `<10μs` por hook proíbe FFI por nó no
  caminho quente — o _matching_ roda inteiramente em Rust, dentro de uma única chamada a `resolve`.
- **Nenhum tipo estrangeiro na fronteira** (`PRD-007:83-84`): `Token`, `Selector` e `MediaQuery` são tipos de `css`;
  `collect_style_sheets` lê o `DomSnapshot`, não o `dom::DomTree`.
- **Object Calisthenics integral** (`CLAUDE.md`, `ADR-0010:127-137`) e o portão de clippy (`Cargo.toml:67-82`): um
  tokenizador escrito à mão é exatamente o lugar onde `s[i..j]`, `chars[i]`, `i + 1` e `c as u32` seriam naturais — e os
  quatro são `deny`.

## Strategic Approach

### Solution Direction

`core/css` ganha uma quarta responsabilidade — **parsear** — inteiramente dentro de `infrastructure/parser/`, cujo único
produto é o `StyleSheetSet` que B0 já congelou em forma. O fluxo cresce de
`DomTree → snapshot → CascadeResolver → LayoutEngine` para
`DomTree → snapshot → collect_style_sheets → StyleSheetSet → CascadeResolver → LayoutEngine`: o passo novo colhe
`<style>` e `style=` **do próprio `DomSnapshot`**, parseia com `parse_stylesheet` / `parse_inline_style`, e entrega ao
resolvedor um conjunto de regras já ordenado por origem. `UaCascade` deixa de ignorar `sheets`: para cada nó, casa os
seletores contra o `DomSnapshot`, ordena os casamentos por `(origem, especificidade, ordem de documento)` e aplica as
declarações sobre a base UA, com o bloco `style=` por último. O recorte §2.8 vira um contrato mecânico: o `MANIFEST.md`
declara linha a linha o que entra, e o `manifest_runner` sonda o parser com um exemplo por linha, falhando se o código e
o manifesto divergirem **em qualquer sentido** — e falhando também se uma forma declarada fora (`:has()`, `::before`,
`@supports`) for aceita.

### Key Design Decisions

- **Tokenizador total + notas tipadas, em vez de tokenizador falível**: trade-off — um tokenizador que devolve `Result`
  aborta a folha inteira num `"` sem par. → Recomendado: o tokenizador é **total** (CSS Syntax L3 §4 é total por
  desenho: string sem terminador vira `BadString`, `url(` sem `)` vira `BadUrl`), e o `rules.rs` converte cada
  construção malformada numa `ParseNote` com `SourceSpan`, recuperando no próximo `}`/`;`. O `Result` de
  `parse_stylesheet` fica reservado ao que torna a fonte inutilizável — hoje só o guarda de profundidade de aninhamento
  (`MAX_NESTING_DEPTH`), que é também a defesa contra entrada hostil que o alvo de _fuzz_ de §2.11 vai exercitar. Assim
  "malformado → `CssError { Parse, span }`" e "nunca descarta o resto da folha" convivem.
- **`Scanner` sobre `Vec<char>` com `.get()`, não `&str` com _slicing_**: trade-off — uma alocação de `4·n` bytes por
  folha. → Recomendado: `string_slice` e `indexing_slicing` são `deny` (`Cargo.toml:72,78`) e um tokenizador precisa de
  _lookahead_ de até 3 caracteres (`\`+hex, `/*`, `-->`, `+.5`). `Peekable<Chars>` dá 1 só. `Vec<char>` + `position` +
  `.get()` + `saturating_add` é a única forma que passa o portão sem `#[allow]`, e mantém linha/coluna exatas para o
  `SourceSpan`.
- **`MediaQuery` em `domain/`, parseado em `infrastructure/parser/media.rs`**: trade-off — o plano lista o VO sob
  `infrastructure/parser/media.rs`. → Recomendado: `StyleRule` (domínio) carrega a condição de mídia; se o VO morasse em
  `infrastructure/`, `domain/` dependeria de `infrastructure/` — inversão direta de `ADR-0010:54-74`. O VO vai para
  `domain/media.rs`; o **parser** dele fica exatamente onde o plano pede. Desvio de arquivo, não de escopo.
- **Filtragem de `@media` pelo produtor, não pelo resolvedor**: trade-off —
  `CascadeResolver::resolve(&DomSnapshot, &StyleSheetSet)` não recebe `ViewportConstraints`, então a cascata não tem
  como avaliar `min-width`. → Recomendado: `StyleSheetSet::matching_viewport(&ViewportConstraints) -> StyleSheetSet` é
  uma consulta do **produtor**, chamada entre `collect_style_sheets` e `resolve`; ela mantém as regras cuja `MediaQuery`
  casa (reescrevendo-as para `ALWAYS`) e descarta as demais. A cascata, defensivamente, **pula** qualquer regra com
  condição não-vazia — o padrão seguro é não aplicar uma regra de mídia não avaliada. Alternativa rejeitada: mudar a
  assinatura da porta, que `PRD-007:56-60` fixa e o I3 congela.
- **`style=` como bloco inline no `StyleSheetSet`, chaveado por `SnapshotId`**: trade-off — acopla o agregado de folhas
  ao snapshot daquele documento. → Recomendado: a alternativa é a cascata parsear `node.attribute("style")` por conta
  própria dentro de `resolve`, o que obrigaria **todo** resolvedor substituto a reimplementar o parser inline — e
  `PRD-007:37-38` diz que quem produz regras parseadas é o parser, não a porta. O acoplamento é ao mesmo documento que o
  `DomSnapshot` já descreve.
- **`Declaration` como VO (propriedade + valor + `Importance`) no lugar do par `(String, String)`**: trade-off — muda a
  forma pública de `DeclarationBlock`, que B0 declarou estável, e obriga `PORT_SCHEMA_VERSION` a subir. → Recomendado: o
  próprio doc-comment de B0 (`stylesheet_set.rs:4-7`) previu essa troca para B1; `!important` aparece em folhas reais e
  descartá-lo em silêncio contradiz a regra do recorte declarado. `PORT_SCHEMA_VERSION` vai de `1` para `2` — que é
  exatamente para isso que ele existe (`ADR-0011` item 3), e o congelamento só acontece em I3.
- **`:hover` / `:active` / `:focus` parseiam, contam especificidade e nunca casam**: trade-off — um seletor "suportado"
  que sempre devolve `false` parece suporte falso. → Recomendado: o `DomSnapshot` (`PRD-007:35-36`) projeta elementos,
  atributos e forma da árvore — **não** estado de interação, que nem existe no motor até a Fase M (janela + eventos).
  Rejeitar a regra inteira apagaria as declarações irmãs numa lista (`a:hover, a { color }`); casar sempre pintaria a
  página errada. Parsear + nunca casar é a única leitura correta hoje, e está declarada no `MANIFEST.md` na coluna
  `notes`.
- **Propriedades: as 6 de B0 mais as 8 longhands de `margin-*`/`padding-*`**: trade-off — a tentação é abrir `width`,
  `height`, `border`, `font-family` agora. → Recomendado: B1 é parser, não cascata (B2) nem box model (B4). As oito
  longhands são grátis — mapeiam nos campos de `LengthEdges` que `ComputedStyle` já tem — e tornam a tabela do
  `MANIFEST.md` uma declaração real em vez de uma cópia da de B0. `rgb()`/`rgba()` e a tabela completa de cores nomeadas
  são explicitamente de B2 (`plano:420-434`).
- **Sonda por entrada do manifesto no `manifest_runner`**: trade-off — cada linha nova do manifesto exige uma sonda
  escrita à mão, e o runner **entra em pânico** se faltar. → Recomendado: é o comportamento desejado. O manifesto sem
  sonda é uma declaração sem prova; a falha ruidosa é o que impede o recorte de encolher em silêncio
  (`relatório §2.8:354`). O _blessing_ `UPDATE_MANIFEST` continua existindo para a **tabela legível**, mas a checagem
  código ⇄ manifesto e a checagem parser ⇄ manifesto não têm caminho de _bless_.

### Alternatives Considered

- **Usar `cssparser` / `selectors` (as crates do Servo)**: rejeitado. `PRD-007:11-13` põe o parsing como Rust nativo
  **deste** crate, e `ADR-0018` governa `unsafe` por superfície de ameaça — CSS de autor é entrada hostil. A pilha do
  Servo também traria `smallvec`/`phf` e um grafo de deps que `deny.toml` teria de auditar. Escrever ~900 linhas sob o
  portão de clippy é o custo aceito e declarado.
- **`Selector` como uma única struct plana com `Vec<Component>`**: rejeitado — colapsa combinador e componente no mesmo
  nível, e a especificidade e o _matching_ right-to-left passam a precisar de varredura com estado. A hierarquia
  `SelectorList → ComplexSelector → SelectorStep → CompoundSelector → componente` é a que o CSS Selectors L4 §3 descreve
  e a que torna `specificity()` uma soma trivial.
- **_Matching_ left-to-right**: rejeitado. Para `div p`, a partir da esquerda é preciso descer toda a subárvore; a
  partir da direita o candidato já é o nó em questão e o trabalho é subir por `parent()`. É a razão pela qual todo motor
  casa da direita para a esquerda, e é o que mantém o custo proporcional à profundidade, não ao tamanho da árvore.
- **_Matching_ recursivo**: rejeitado por `ADR-0010` / precedente do repo (`snapshot.rs:22-31`, `dom::serialize_html`) —
  profundidade de árvore vinda de entrada hostil não pode virar estouro de pilha.
- **Guardar o texto original do seletor em `StyleRule`**: rejeitado — `SelectorList` ganha `Display`, então o texto se
  reconstrói para diagnóstico sem carregar uma `String` por regra nem deixar duas fontes de verdade.
- **Um `SUPPORTED_AT_RULES` e uma terceira seção no `MANIFEST.md`**: rejeitado — o plano fixa duas seções.
  `@media (min-width)` e `@media (max-width)` entram na tabela de seletores, cuja frase de abertura diz que ela cobre
  "as formas de seletor e as condições de at-rule que as gateiam".

## Risk & Gap Analysis

### Requirement Ambiguities

- **"`Selector` VO" (plano `:409`) versus a hierarquia que o recorte exige**: não existe um tipo único que seja ao mesmo
  tempo lista, sequência e composto. Resolução: o módulo `domain/selector/` é o "`Selector` VO" do plano; os tipos
  exportados são `SelectorList`, `ComplexSelector`, `CompoundSelector`, `Combinator`, `TypeSelector`,
  `AttributeSelector`, `AttributeMatch`, `PseudoClass`, `NthFormula`. **Não** há um tipo chamado `Selector`; o ponto de
  entrada do facade é `SelectorList`. Desvio registrado.
- **"`<link rel=stylesheet>`" (relatório `:338`)**: exige um subrecurso de rede (§2.11 / Fase C1). Fora de B1 — o parser
  está pronto para a folha que chegar, mas quem a busca é outra fase. Registrado, não implementado.
- **"ordem de documento como desempate" (plano `:410`)**: ambíguo entre ordem do **nó** e ordem da **regra**. Resolução:
  é a ordem da regra na folha — é o que a cascata CSS define, e `StyleSheetSet` já preserva a ordem de inserção
  (`stylesheet_set.rs:140`).
- **`!important` "entra integralmente" no relatório `§2.8:347`, mas o plano o lista sob B2 (`:423`)**: resolução — B1
  **parseia e preserva** a flag (`Importance`); B2 a **honra** na ordenação da cascata. Sem isso, B1 teria de descartar
  `!important` em silêncio, que é exatamente o que o recorte declarado proíbe.
- **O plano não diz o que `parse_stylesheet` faz com uma at-rule desconhecida**: resolução — pula até o fim do bloco (ou
  até o `;` de uma at-rule sem bloco) e registra uma `ParseNote`. Nunca falha a folha; nunca engole em silêncio.

### Edge Cases

- **`<style>` com filho `Text` vazio, ou `<style>` sem filho**: folha vazia, `StyleSheetSet` sem regras, sem erro.
- **`style=""`**: bloco inline vazio — não pode virar uma entrada inline que sobrescreve nada com nada.
- **`:nth-child(2n+1)`, `odd`, `even`, `-n+3`, `0n+2`, `3`**: a fórmula `an+b` com `a` negativo e `b` negativo, e as
  palavras-chave. `a = 0` degenera para "índice exatamente `b`". `matches` tem de resolver
  `(index - b) % a == 0 && (index - b) / a >= 0` **sem** `arithmetic_side_effects` — tudo `checked_*`.
- **`:nth-child` contando irmãos**: conta apenas irmãos **elemento**, 1-based. Texto e comentário entre elementos não
  podem deslocar o índice.
- **Combinador `+` / `~` atravessando nós de texto**: `<p>a</p> texto <span>` — `p + span` casa, porque o irmão
  imediatamente anterior _elemento_ é `p`.
- **Comentário no meio de um seletor** (`div/* x */p`): o tokenizador remove comentários, mas isso **não** pode fundir
  `div` e `p` num só identificador — o comentário separa como espaço em branco separa.
- **`\` como escape no fim da fonte**, `"string` sem terminador, `url(` sem `)`: as três terminações abruptas do
  tokenizador. Nenhuma pode entrar em laço infinito nem estourar índice.
- **Seletor vazio numa lista** (`p, , span`): a lista inteira é inválida por CSS Selectors L4 §3.1 — regra descartada
  com nota. `p,` no fim: idem.
- **Regra com seletor fora do recorte numa lista** (`a:hover, a:has(b)`): a lista inteira é descartada com nota — é o
  comportamento do CSS para seletor não reconhecido, e é o que impede aplicar metade de uma regra.
- **`@media` aninhado em `@media`**: fora do recorte; nota + salto do bloco.
- **Declaração sem `:` ou sem valor** (`color`, `color:`): descartada com nota, recuperando no `;`.
- **Propriedade desconhecida** (`float: left`): só a declaração cai, a regra sobrevive com as irmãs.
- **Nó raiz sem pai em `:first-child`**: o `Document` não tem pai; `:first-child` não casa (não há lista de irmãos) e
  não pode estourar.
- **Documento sem `<style>` e sem `style=`**: `collect_style_sheets` devolve um conjunto vazio e `UaCascade` produz
  exatamente o `StyledTree` que produzia em B0 — o teste de conformidade e as goldens de B0 não podem mudar.

### Technical Risks

- **O portão de clippy contra o vocabulário natural de um tokenizador**: `string_slice`, `indexing_slicing`,
  `arithmetic_side_effects`, `as_conversions` (`Cargo.toml:72-82`) proíbem `s[i..j]`, `chars[i]`, `i + 1` e `c as u32` —
  as quatro construções que aparecem em qualquer tokenizador de livro. Mitigação: `Scanner` sobre `Vec<char>` com
  `.get()`; `saturating_add`/`checked_*` para o cursor e para o `an+b`; `u32::from`/`i32::try_from` para estreitar;
  nenhum `#[allow]` em código de lib (só o header comentado de `conformance.rs` e os `tests/`).
- **`f32` na conversão de número CSS**: `"1.5"` → `f32` sem `as`. Mitigação: `str::parse::<f32>()`, que devolve `Result`
  — e a montagem do literal é feita acumulando `char`s numa `String`, não por aritmética de dígitos.
- **Entidades < ~100 linhas com um tokenizador de ~15 formas de token**: mitigação: `parser/` é dividido em `token.rs`
  (vocabulário), `scanner.rs` (cursor), `tokenizer.rs` (o laço + leitores focados), `selectors.rs`, `rules.rs`,
  `media.rs`, `mod.rs`; cada leitor (`read_ident`, `read_number`, `read_string`, `read_url`, `read_escape`) é uma função
  própria com um nível de indentação.
- **Backtracking exponencial no _matching_**: `a b c d e` contra uma árvore profunda. Mitigação: o recorte não tem
  `:has()` (o caso realmente patológico); a work-stack tem um teto de passos derivado da profundidade da árvore, e o
  custo real de uma folha de classe `example.com` é irrelevante. Registrado como risco de F9c, não de B1.
- **`PORT_SCHEMA_VERSION` 1 → 2 e a Fase EE**: a Fase EE (`plano:596`) também planeja mexer no
  `engine::PORT_SCHEMA_VERSION` (2 → 3). São constantes de crates diferentes; não colidem. O de `css` congela em I3.
- **`UaCascade` passa a depender do parser**: `infrastructure/cascade` → `infrastructure/parser` é dependência
  intra-camada, legítima em `ADR-0010`. Mas a cascata **não** pode chamar `parse_stylesheet` — só o parser de _valores_
  de declaração. Mitigação: `infrastructure/cascade/values.rs` é o único ponto de contato, e ele consome
  `DeclarationValue`, não texto de folha.
- **`cargo test -p css --no-default-features`**: os testes novos (`parser.rs`, `selectors.rs`, `authored_style.rs`) não
  podem depender de nada gateado por `builtin-adapters` — como em B0, a feature continua não gateando nada.
- **`manifest_runner` e `arch-lint`**: o arquivo já está em `[analyzer].exclude` (`arch-lint.toml:20`); a versão nova,
  que entra em pânico com mensagem, continua coberta pela mesma exclusão.
- **Determinismo (`PRD-007:100`)**: a ordenação de regras casadas tem de ser **estável e total** —
  `(precedência de origem, especificidade, índice da regra)` não tem empates possíveis, porque o índice é único.
  Ordenação instável ou chave parcial faria os 100 runs divergirem.

### Acceptance Criteria Coverage

| AC / DoD                 | Descrição                                                                | Endereçável? | Lacunas / Notas                                                                                                                               |
| ------------------------ | ------------------------------------------------------------------------ | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `PRD-007:11-13`          | Parsing fica em Rust nativo no `core/css`                                | Yes          | `infrastructure/parser/`; nenhum tipo do parser numa assinatura de porta                                                                      |
| `PRD-007:37-38`          | `StyleSheetSet` = regras parseadas e ordenadas, produzidas pelo parser   | Yes          | `parse_stylesheet(source, origin)` + `collect_style_sheets(&DomSnapshot)`                                                                     |
| plano `:417`             | Recorte do `MANIFEST.md` 100% verde                                      | Yes          | Tabela `\| token \| since \| notes \|` + sonda por entrada no `manifest_runner`                                                               |
| plano `:417`             | `<style>` / `style=` aplicados de forma observável via `CascadeResolver` | Yes          | `tests/authored_style.rs`: `<style>` + `style=` → `resolve` → `color`/`margin` computados; `#id` > `.class` > tipo                            |
| relatório `§2.8:340-345` | Recorte de seletor exatamente como a tabela                              | Yes          | `tests/selectors.rs` cobre cada forma; o runner prova a **rejeição** de `:has()`, `::before`, `@supports`, namespace                          |
| relatório `§2.8:334`     | Especificidade de 3 componentes, origem/documento como desempate         | Yes          | `Specificity: Ord`; chave de ordenação `(origem, especificidade, índice)`                                                                     |
| plano `:406`             | `@media` `min-width` / `max-width`                                       | Yes          | `MediaQuery::matches(&ViewportConstraints)` + `StyleSheetSet::matching_viewport` (a porta não recebe viewport — decisão registrada)           |
| `PRD-007:98` / `:100`    | Determinismo, 100 runs                                                   | Yes          | `run_css_conformance` continua verde; a chave de ordenação da cascata é total                                                                 |
| `PRD-007:83-84`          | Sem tipo estrangeiro                                                     | Yes          | `collect_style_sheets` lê o `DomSnapshot`; `snapshot.rs` segue o único arquivo que nomeia `core/dom`                                          |
| DoD do brief             | `manifest_runner` falha nos dois sentidos                                | Yes          | Verificado à mão removendo uma entrada de `SUPPORTED_PROPERTIES` sem tocar o `MANIFEST.md`                                                    |
| DoD do brief             | Job CI `css-conformance` blocante + recipe `just css-conformance`        | Yes          | `needs: rust-quality`, ubuntu-24.04, bloco de cache padrão                                                                                    |
| `PRD-007:95` / `:96`     | Adaptador `.rhai` altera propriedade; pânico cai pro built-in            | No           | Fora de B1 — o adaptador scriptável mora em `rhai-bindings` (Fase M)                                                                          |
| relatório `§2.8:347`     | Três origens + `!important` + herança integralmente                      | Partial      | B1 **parseia e preserva** `Origin` e `Importance` e aplica Author sobre UA; a cascata de três origens com `!important` é B2 (`plano:420-434`) |
