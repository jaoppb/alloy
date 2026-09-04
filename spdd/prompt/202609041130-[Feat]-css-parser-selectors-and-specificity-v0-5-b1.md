# `core/css` — CSS Syntax L3 tokenizer, selector engine and specificity (v0.5 B1)

## Requirements

Dar ao `core/css` a responsabilidade que `PRD-007:11-13` reserva ao Rust nativo: **parsear CSS**. Entram um tokenizador
CSS Syntax Level 3 escrito à mão (at-rules, blocos `{}`/`()`/`[]`, funções, `url()`, strings com escape, comentários,
números/dimensões/percentagens, `#hash`, delimitadores), um parser de regras que popula o `StyleSheetSet` que B0 já
congelou em forma (`core/css/src/domain/stylesheet_set.rs:4-7`), o motor de seletores do recorte §2.8 do relatório
(`docs/reports/IMPLEMENTACAO-DETALHADA-V0-5.md:340-345`) com especificidade de três componentes, `@media` com
`min-width`/`max-width`, e a ligação de `<style>` e `style=` de forma **observável** através de
`CascadeResolver::resolve`. O recorte é declarado por manifesto (`core/css/tests/data/MANIFEST.md`) e provado nos dois
sentidos pelo `manifest_runner`, incluindo a **rejeição** do que está fora (`:has()`, namespaces, `::before`/`::after`,
`@supports`/`@font-face`/`@import`/`@keyframes`). Fora de B1: a cascata de três origens com `!important` (B2),
`rgb()`/`rgba()` e a tabela completa de cores nomeadas (B2), box model / IFC / Flexbox (B4), `<link rel=stylesheet>`
(C1/§2.11), e o adaptador `.rhai` (Fase M). `core/css` continua sem `engine` e sem `rhai`.

## Entities

```mermaid
classDiagram
direction TB

class Token {
    <<enumeration non_exhaustive>>
    Ident(Identifier)
    AtKeyword(Identifier)
    Hash(String)
    QuotedString(String)
    BadString
    Function(Identifier)
    Url(String)
    BadUrl
    Number(f32)
    Dimension(f32, Identifier)
    Percentage(f32)
    Delimiter(char)
    Whitespace
    Colon
    Semicolon
    Comma
    OpenBrace
    CloseBrace
    OpenParenthesis
    CloseParenthesis
    OpenBracket
    CloseBracket
}

class SpannedToken {
    +token() Token
    +span() SourceSpan
}

class TokenStream {
    -Vec~SpannedToken~ tokens
    -usize position
    +peek() Option~Token~
    +peek_span() SourceSpan
    +advance()
    +is_exhausted() bool
    +skip_whitespace()
}

class Scanner {
    -Vec~char~ characters
    -usize position
    -u32 line
    -u32 column
    +peek() Option~char~
    +peek_ahead(usize) Option~char~
    +consume()
    +span() SourceSpan
}

class Identifier {
    -String text
    +new(str) Option~Identifier~
    +lowercased(str) Option~Identifier~
    +as_str() str
}

class SelectorList {
    -Vec~ComplexSelector~ selectors
    +iter() Iterator~ComplexSelector~
    +len() usize
    +is_empty() bool
}

class ComplexSelector {
    -Vec~SelectorStep~ steps
    +steps() Iterator~SelectorStep~
    +subject() Option~CompoundSelector~
    +specificity() Specificity
}

class SelectorStep {
    +combinator() Combinator
    +compound() CompoundSelector
}

class Combinator {
    <<enumeration>>
    Descendant
    Child
    NextSibling
    SubsequentSibling
}

class CompoundSelector {
    -TypeSelector type_selector
    -ClassNames classes
    -ElementIds ids
    -AttributeSelectors attributes
    -PseudoClasses pseudo_classes
    +specificity() Specificity
}

class TypeSelector {
    <<enumeration>>
    Universal
    Named(Identifier)
}

class AttributeSelector {
    +name() Identifier
    +match_kind() AttributeMatch
}

class AttributeMatch {
    <<enumeration non_exhaustive>>
    Exists
    Exact(String)
}

class PseudoClass {
    <<enumeration non_exhaustive>>
    Hover
    Active
    Focus
    FirstChild
    LastChild
    NthChild(NthFormula)
}

class NthFormula {
    -i32 step
    -i32 offset
    +matches(u32) bool
}

class Specificity {
    -u16 ids
    -u16 classes
    -u16 types
    +ZERO Specificity
    +plus(Specificity) Specificity
}

class MediaQuery {
    -Vec~MediaCondition~ conditions
    +ALWAYS MediaQuery
    +is_always() bool
    +matches(ViewportConstraints) bool
}

class MediaCondition {
    +feature() MediaFeature
    +length() Length
}

class MediaFeature {
    <<enumeration non_exhaustive>>
    MinWidth
    MaxWidth
}

class Declaration {
    +property() Identifier
    +value() DeclarationValue
    +importance() Importance
}

class DeclarationValue {
    -String text
    +as_str() str
}

class Importance {
    <<enumeration>>
    Normal
    Important
}

class DeclarationBlock {
    -Vec~Declaration~ declarations
    +push(Declaration)
    +iter() Iterator~Declaration~
    +len() usize
}

class StyleRule {
    -SelectorList selectors
    -DeclarationBlock declarations
    -MediaQuery media
    +selectors() SelectorList
    +declarations() DeclarationBlock
    +media() MediaQuery
}

class StyleSheetSet {
    -Vec~OriginRule~ rules
    -InlineStyles inline
    -ParseNotes notes
    +push_rule(Origin, StyleRule)
    +push_inline(SnapshotId, DeclarationBlock)
    +rules() Iterator
    +inline_of(SnapshotId) Option~DeclarationBlock~
    +notes() ParseNotes
    +matching_viewport(ViewportConstraints) StyleSheetSet
    +merge(StyleSheetSet)
}

class InlineStyles {
    -Vec~(SnapshotId, DeclarationBlock)~ blocks
    +get(SnapshotId) Option~DeclarationBlock~
}

class ParseNote {
    +message() str
    +span() SourceSpan
}

class ParseNotes {
    -Vec~ParseNote~ notes
    +iter() Iterator~ParseNote~
    +is_empty() bool
}

class DomSnapshot {
    +root() SnapshotId
    +node(SnapshotId) Option~NodeRef~
    +nodes_in_document_order() Iterator
}

class NodeRef {
    +kind() SnapshotNodeKind
    +tag() Option~str~
    +attribute(str) Option~str~
    +parent() Option~SnapshotId~
    +children() Iterator~SnapshotId~
}

class CascadeResolver {
    <<interface>>
    +resolve(DomSnapshot, StyleSheetSet) Result~StyledTree~
}

class UaCascade {
    +resolve(DomSnapshot, StyleSheetSet) Result~StyledTree~
}

class ComputedStyle {
    +display() Display
    +color() CssColor
    +margin() LengthEdges
    +padding() LengthEdges
    +font_size() Length
}

TokenStream o-- SpannedToken
SpannedToken o-- Token
Token ..> Identifier
Scanner ..> Token : produces via tokenize()
SelectorList o-- ComplexSelector
ComplexSelector o-- SelectorStep
SelectorStep o-- Combinator
SelectorStep o-- CompoundSelector
CompoundSelector o-- TypeSelector
CompoundSelector o-- AttributeSelector
CompoundSelector o-- PseudoClass
AttributeSelector o-- AttributeMatch
PseudoClass o-- NthFormula
ComplexSelector ..> Specificity
MediaQuery o-- MediaCondition
MediaCondition o-- MediaFeature
DeclarationBlock o-- Declaration
Declaration o-- DeclarationValue
Declaration o-- Importance
StyleRule o-- SelectorList
StyleRule o-- DeclarationBlock
StyleRule o-- MediaQuery
StyleSheetSet o-- StyleRule
StyleSheetSet o-- InlineStyles
StyleSheetSet o-- ParseNotes
ParseNotes o-- ParseNote
UaCascade ..|> CascadeResolver
UaCascade ..> StyleSheetSet : matches selectors against
UaCascade ..> ComputedStyle : applies declarations by specificity
DomSnapshot o-- NodeRef
```

## Approach

1. **O parser é infraestrutura, seu produto é agregado de fronteira (`PRD-007:11-13`, `:37-38`)**: todo módulo novo de
   parsing mora em `core/css/src/infrastructure/parser/`; nada dele aparece numa assinatura de porta. O produto é o
   `StyleSheetSet` que `application/ports.rs:33` já recebe.
2. **Tokenizador total, recuperação no parser de regras**: o tokenizador de CSS Syntax L3 é **total** por desenho —
   string sem terminador vira `Token::BadString`, `url(` sem `)` vira `Token::BadUrl`, `)` órfão vira
   `Token::CloseParenthesis`. Nenhum caminho do tokenizador devolve `Err`. Quem recupera é `rules.rs`: regra malformada
   consome até o `}` que a fecha, declaração malformada até o `;`, e cada recuperação vira uma `ParseNote` com
   `SourceSpan`. O `Result<_, CssError>` de `parse_stylesheet` fica reservado ao que torna a fonte inutilizável — hoje
   só `MAX_NESTING_DEPTH` (guarda contra entrada hostil, o alvo de _fuzz_ de §2.11).
3. **`Scanner` sobre `Vec<char>` (`Cargo.toml:72,78`)**: `string_slice` e `indexing_slicing` são `deny`, e o tokenizador
   precisa de _lookahead_ de até 3 caracteres (`\`+hex, `/*`, `-->`, `+.5`) — mais do que `Peekable` dá. O `Scanner`
   guarda `Vec<char>` + `position` + `line`/`column`, lê com `.get()` e avança com `saturating_add`. CQS:
   `peek()`/`peek_ahead()` consultam, `consume()` comanda e devolve `()`.
4. **Hierarquia de seletor do CSS Selectors L4 §3**: `SelectorList` → `ComplexSelector` →
   `SelectorStep { Combinator, CompoundSelector }` → o composto (`TypeSelector`, `ClassNames`, `ElementIds`,
   `AttributeSelectors`, `PseudoClasses`). O combinador do **primeiro** passo é sempre `Descendant` e não tem
   significado (documentado) — o casamento termina ao consumir esse passo.
5. **_Matching_ right-to-left, não-recursivo (`snapshot.rs:22-31` como molde)**: a work-stack guarda
   `(índice do passo, SnapshotId candidato)`. O último passo é o sujeito; cada passo casado empurra os candidatos do
   passo anterior conforme o combinador — `Descendant` empurra todos os ancestrais, `Child` o pai, `NextSibling` o
   irmão-elemento imediatamente anterior, `SubsequentSibling` todos os irmãos-elemento anteriores. Chegar ao índice `0`
   com casamento é `true`; esvaziar a pilha é `false`.
6. **Especificidade é soma, comparação é `Ord` derivado**: `Specificity { ids, classes, types }` em `u16`, `derive(Ord)`
   na ordem dos campos — que é exatamente a ordem lexicográfica que o CSS define. `#id` soma `(1,0,0)`; `.classe`,
   `[attr]`, `[attr=v]` e cada pseudo-classe somam `(0,1,0)`; um tipo soma `(0,0,1)`; `*` soma `(0,0,0)`. Toda soma é
   `saturating_add`.
7. **`@media` filtrado pelo produtor, não pelo resolvedor**: `CascadeResolver::resolve` (`ports.rs:33`) não recebe
   `ViewportConstraints` e `PRD-007:56-60` fixa a assinatura (congela em I3). `StyleRule` carrega uma `MediaQuery`
   (`ALWAYS` quando fora de `@media`); `StyleSheetSet::matching_viewport(&ViewportConstraints)` é a consulta do produtor
   que mantém as regras cuja mídia casa (reescritas para `ALWAYS`) e descarta as demais. A cascata **pula** toda regra
   com condição não-vazia — o padrão seguro para uma regra de mídia não avaliada.
8. **`<style>` e `style=` colhidos do `DomSnapshot`, não do `DomTree`**:
   `application/collect_sheets.rs::collect_style_sheets(&DomSnapshot) -> Result<StyleSheetSet, CssError>` percorre a
   projeção em ordem de documento; o texto dos filhos `Text` de um `<style>` vai para
   `parse_stylesheet(_, Origin::Author)`, e cada `style=` vai para `parse_inline_style` → `push_inline(id, block)`.
   `application/snapshot.rs` segue sendo o **único** arquivo que nomeia `core/dom` (`PRD-007:83-84`).
9. **A cascata passa a consultar o `StyleSheetSet` (entregável do plano `:417`)**: `UaCascade::resolve` mantém a base UA
   por tag (`ua_sheet.rs:55-82`) e a herança de `color`/`font-size`, e sobre ela aplica as regras casadas ordenadas por
   `(Origin::precedence, Specificity, índice da regra)` — chave **total**, sem empate possível, que é o que mantém os
   100 runs de `PRD-007:100` idênticos. O bloco `style=` do nó é aplicado por último.
10. **Valores: só o que as 14 propriedades declaradas exigem**: `infrastructure/cascade/values.rs` converte
    `DeclarationValue` em `Display` (palavras-chave), `CssColor` (`#rgb`, `#rrggbb`, `transparent` e as cores nomeadas
    básicas), `Length` (`px`/`em`/`rem`/`%`/`pt` e o `0` sem unidade) e `LengthEdges` (1–4 componentes). `rgb()`/
    `rgba()` e a tabela completa de nomes são de B2 (`plano:420-434`) e caem como declaração descartada com nota.
11. **O recorte é mecânico nos três sentidos**: `MANIFEST.md` vira duas tabelas `| token | since | notes |`;
    `manifest_runner.rs` compara (a) manifesto ⇄ `SUPPORTED_PROPERTIES`/ `SUPPORTED_SELECTORS`, (b) manifesto ⇄ **o que
    o parser aceita de fato** (uma sonda por entrada; entrada sem sonda entra em pânico com a mensagem que diz o que
    fazer), e (c) uma bateria de formas declaradas **fora** que o parser tem de rejeitar. `UPDATE_MANIFEST` continua
    reescrevendo só a tabela legível; as três checagens não têm caminho de _bless_.
12. **`PORT_SCHEMA_VERSION` 1 → 2**: `DeclarationBlock` passa de pares `(String, String)` a uma coleção de
    `Declaration`, `StyleRule` troca `selector_text: String` por `SelectorList` + `MediaQuery`, e `StyleSheetSet` ganha
    blocos inline e notas — mudanças que um produtor ou um resolvedor substituto nota. `ADR-0011` item 3 manda bumpar; o
    congelamento é em I3 (fim de B4).

## Structure

### Inheritance / trait relationships

1. `UaCascade` implementa `CascadeResolver` (`application/ports.rs:31`) — assinatura **inalterada**.
2. `Token`, `AttributeMatch`, `PseudoClass`, `MediaFeature` são `#[non_exhaustive]`; `Combinator`, `TypeSelector`,
   `Importance` não (o recorte de combinadores e de importância é fechado por definição do CSS).
3. `Specificity` deriva `PartialOrd, Ord` na ordem `(ids, classes, types)`; `Eq`, `Hash`, `Copy`, `Default`.
4. `Identifier`, `DeclarationValue`, `ParseNote` derivam `Clone, Debug, PartialEq, Eq, Hash` e têm `Display`.
5. `MediaQuery`, `SelectorList`, `ComplexSelector`, `CompoundSelector` derivam `Clone, Debug, PartialEq, Eq`
   (`MediaCondition` guarda `Length`, que tem `f32` — então `MediaCondition`/`MediaQuery` derivam só `PartialEq`, sem
   `Eq`; `StyleRule` e `StyleSheetSet` seguem essa restrição e perdem `Eq`).
6. `SelectorList`, `ComplexSelector`, `CompoundSelector` e `Specificity` implementam `fmt::Display`, para diagnóstico e
   para reconstruir o texto do seletor sem guardar uma `String` por regra.
7. Nenhuma trait nova é introduzida — `PRD-007` fixa três portas e B1 não acrescenta uma quarta.

### Dependencies

1. `core/css/Cargo.toml` **não muda**: `dom` (path), `graphics` (path), `thiserror` (workspace). Nenhuma dependência
   nova — o parser é escrito à mão (`PRD-007:11-13`, `ADR-0018`).
2. `domain/` → só `domain/` + os VOs de `graphics` (`Au`, `Px`, `Color`, `Rect`). `application/` → `domain/` (+ `dom` só
   em `snapshot.rs`). `infrastructure/parser/` → `domain/`. `infrastructure/cascade/` → `domain/` +
   `application/matching.rs` + `infrastructure/parser` (só o parser de valores).
3. `arch-lint.toml` **não muda**: `infrastructure/parser/**` já cai no escopo `css` (`arch-lint.toml:86-88`) e
   `application/matching.rs` no escopo `css_application` (`:82-84`). Nenhum diretório de topo novo.
4. `.github/workflows/ci.yml` ganha o job blocante `css-conformance` (`needs: rust-quality`, ubuntu-24.04, bloco de
   cache padrão) rodando `cargo test -p css --test manifest_runner`, com espaço comentado para `-p html` em B5. O
   `justfile` ganha a recipe `css-conformance` e a inclui em `gate`.

### Layered responsibilities (`ADR-0010:54-74`)

1. `domain/` — `identifier.rs` (`Identifier`), `selector/{mod,component,compound,complex}.rs` (a família de seletor),
   `specificity.rs` (`Specificity`), `media.rs` (`MediaQuery`/`MediaCondition`/`MediaFeature`), `declaration.rs`
   (`Declaration`/`DeclarationValue`/`Importance`/`DeclarationBlock`), `parse_notes.rs` (`ParseNote`/`ParseNotes`),
   `stylesheet_set.rs` (`Origin`/`StyleRule`/`InlineStyles`/`StyleSheetSet`, reescrito). Zero I/O, zero `else`.
2. `application/` — `matching.rs` (`matches`), `collect_sheets.rs` (`collect_style_sheets`), `snapshot.rs` e `ports.rs`
   **intocados**, `conformance.rs` intocado.
3. `infrastructure/parser/` — `token.rs` (`Token`/`SpannedToken`/`TokenStream`), `scanner.rs` (`Scanner`),
   `tokenizer.rs` (`tokenize` + leitores focados), `selectors.rs` (gramática do recorte §2.8), `media.rs` (prelúdio de
   `@media`), `rules.rs` (regras, declarações, recuperação, at-rules), `mod.rs` (`parse_stylesheet`,
   `parse_inline_style`).
4. `infrastructure/cascade/` — `mod.rs` (re-export), `author_rules.rs` (ordenação por especificidade + aplicação),
   `values.rs` (`DeclarationValue` → `Display`/`CssColor`/`Length`/`LengthEdges`). `ua_sheet.rs` mantém a base UA por
   tag e passa a chamar `author_rules`.
5. `tests/` — `parser.rs`, `selectors.rs`, `authored_style.rs` (novos); `manifest_runner.rs`, `value_objects.rs`
   (reescritos); `css_conformance.rs`, `port_swap.rs`, `pipeline.rs` (inalterados, têm de continuar verdes).

## Operations

### 1. `domain/identifier.rs`

1. `pub struct Identifier(String)` com `new(text: &str) -> Option<Self>` (rejeita vazio e qualquer caractere fora de
   `[a-zA-Z0-9_-]`, não-ASCII, ou escapado) e `lowercased(text: &str) -> Option<Self>` (o mesmo, em minúsculas — para
   nome de tag, de atributo, de propriedade e de at-rule).
2. `as_str()`, `Display`, `#[must_use]`, `const fn` onde o corpo permite. Molde: `core/dom/src/domain/tag_name.rs`.

### 2. `domain/selector/`

1. `component.rs` — `TypeSelector { Universal, Named(Identifier) }`; `AttributeMatch { Exists, Exact(String) }`
   (`#[non_exhaustive]`); `AttributeSelector { name: Identifier, match_kind: AttributeMatch }`;
   `NthFormula { step: i32, offset: i32 }` com `matches(index_one_based: u32) -> bool` usando
   `checked_sub`/`checked_rem`/ `checked_div` (nunca `%` cru — `arithmetic_side_effects`); `PseudoClass`
   (`#[non_exhaustive]`) com `specificity_contribution()`.
2. `compound.rs` — `ClassNames`, `ElementIds`, `AttributeSelectors`, `PseudoClasses` (quatro coleções de primeira
   classe, sem `Vec` público) e `CompoundSelector` com `specificity()` e `Display`.
3. `complex.rs` — `Combinator`; `SelectorStep { combinator, compound }`; `ComplexSelector { steps: Vec<SelectorStep> }`
   com `steps()`, `specificity()` (soma `saturating_add` sobre os passos) e `Display`; `SelectorList` com `iter()`,
   `len()`, `is_empty()`, `Display` (junta com `", "`).
4. `mod.rs` — `pub mod` + `pub use` dos nove tipos.

### 3. `domain/specificity.rs`

1. `pub struct Specificity { ids: u16, classes: u16, types: u16 }`,
   `derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)` — a ordem dos campos **é** a ordem de
   comparação.
2. `ZERO`, `new(ids, classes, types)`, `id()`, `class()`, `type_name()` (construtores de uma unidade), `plus(Self)` com
   `saturating_add`, e os três getters. `Display` como `"(1,0,2)"`.

### 4. `domain/media.rs`

1. `MediaFeature { MinWidth, MaxWidth }` (`#[non_exhaustive]`), `MediaCondition { feature, length: Length }` com
   `matches(&ViewportConstraints) -> bool` (resolve a `Length` contra a largura do viewport com o `font-size` inicial de
   `16px`), e `MediaQuery { conditions: Vec<MediaCondition> }` (coleção de primeira classe) com `ALWAYS`, `is_always()`,
   `push(MediaCondition)` e `matches(&ViewportConstraints)` = `all()` das condições.

### 5. `domain/declaration.rs` e `domain/parse_notes.rs`

1. `Importance { Normal, Important }`; `DeclarationValue(String)` (normaliza espaço em branco na construção);
   `Declaration { property: Identifier, value: DeclarationValue, importance: Importance }` com getters `#[must_use]`.
2. `DeclarationBlock` — coleção de primeira classe de `Declaration`: `new()`, `push(Declaration)`,
   `declare(property, value)` (conveniência que constrói `Importance::Normal`, mantida para os testes de B0), `iter()`,
   `len()`, `is_empty()`.
3. `ParseNote { message: String, span: SourceSpan }` + `ParseNotes` (coleção de primeira classe) com `push`, `iter`,
   `len`, `is_empty`, `extend(ParseNotes)`.

### 6. `domain/stylesheet_set.rs` (reescrito)

1. `Origin` e `precedence()` **inalterados**.
2. `StyleRule::new(selectors: SelectorList, declarations: DeclarationBlock)`, `with_media(MediaQuery)`, e os getters
   `selectors()`, `declarations()`, `media()`. `selector_text: String` sai — `SelectorList: Display` o reconstrói.
3. `InlineStyles` — coleção de primeira classe de `(SnapshotId, DeclarationBlock)` com `get(SnapshotId)`.
4. `StyleSheetSet` ganha `push_inline`, `inline_of`, `notes()`, `push_note`, `merge(Self)` e
   `matching_viewport(&ViewportConstraints) -> Self`.

### 7. `infrastructure/parser/token.rs` e `scanner.rs`

1. `Token` (`#[non_exhaustive]`) com as 22 formas do diagrama; `SpannedToken { token, span }`; `TokenStream` — coleção
   de primeira classe com cursor: `peek()`, `peek_span()`, `advance()` (comando), `skip_whitespace()` (comando),
   `is_exhausted()`.
2. `Scanner` — `Vec<char>` + `position` + `line` + `column`; `peek()`, `peek_ahead(offset)`, `consume()` (atualiza
   linha/coluna ao ver `\n`), `span()`. Zero `unwrap`, zero índice cru, zero `as`.

### 8. `infrastructure/parser/tokenizer.rs`

1. `pub fn tokenize(source: &str) -> TokenStream` — total, nunca falha.
2. Leitores focados, um nível de indentação cada: `read_whitespace`, `read_comment` (`/* */`, sem terminador = fim),
   `read_identifier_like` (ident, função, `url(`), `read_string` (`'`/`"`, escapes, `BadString` no fim de linha/fonte),
   `read_url` (conteúdo não-citado, `BadUrl` sem `)`), `read_numeric` (número, dimensão, percentagem, sinal, expoente),
   `read_hash`, `read_escape` (`\`+1–6 hex+espaço opcional, ou `\`+qualquer), `read_delimiter`.
3. `<!--` e `-->` são consumidos como comentário (folhas antigas dentro de `<style>`), documentado.

### 9. `infrastructure/parser/selectors.rs`

1. `pub(crate) fn parse_selector_list(tokens: &mut TokenStream) -> Result<SelectorList, CssError>` — consome até `{` ou
   `,` no nível de topo; devolve `CssError { stage: Selector, span }` para qualquer forma **fora** do recorte §2.8: `::`
   (pseudo-elemento), `|` (namespace), `:has(`, qualquer pseudo-classe desconhecida, qualquer matcher de atributo que
   não seja `]` ou `=`.
2. Helpers: `parse_complex`, `parse_compound`, `parse_attribute`, `parse_pseudo_class`, `parse_nth` (`an+b`, `odd`,
   `even`, `n`, `-n+3`, inteiro puro).
3. Uma lista com **qualquer** seletor inválido é inválida por inteiro (CSS Selectors L4 §3.1) — a regra cai com nota.

### 10. `infrastructure/parser/media.rs` e `rules.rs`

1. `media.rs` — `pub(crate) fn parse_media_prelude(tokens: &mut TokenStream) -> Result<MediaQuery, CssError>`:
   `(min-width: <length>)` / `(max-width: <length>)` unidos por `and`; qualquer outra feature ou operador é
   `CssError { stage: Parse }`.
2. `rules.rs` — `parse_rules(tokens, origin, sheets)`: laço de topo que despacha entre at-rule (`@media` → recursão de
   um nível sobre o bloco; qualquer outra → salta o bloco/`;` + nota) e regra qualificada (seletor + `{` bloco de
   declarações `}`). `parse_declaration_block` lê `ident : valor [!important] ;` com recuperação no `;` e no `}`, e
   descarta a declaração cuja propriedade não está em `SUPPORTED_PROPERTIES` (com nota).
3. Guarda `MAX_NESTING_DEPTH = 32`: excedê-lo é o **único** `Err` que sobe até `parse_stylesheet`.

### 11. `infrastructure/parser/mod.rs`

1. `pub fn parse_stylesheet(source: &str, origin: Origin) -> Result<StyleSheetSet, CssError>`.
2. `pub fn parse_inline_style(source: &str) -> Result<DeclarationBlock, CssError>` — o corpo de um `style=`, sem seletor
   e sem chaves.
3. `pub use` de `tokenize`, `Token`, `SpannedToken`, `TokenStream` (o `tests/parser.rs` precisa deles).

### 12. `application/matching.rs` e `application/collect_sheets.rs`

1. `pub fn matches(selector: &ComplexSelector, node: NodeRef<'_>, snapshot: &DomSnapshot) -> bool` — work-stack,
   right-to-left; `NodeRef` é `Copy`, então vai por valor (evita `trivially_copy_pass_by_ref`).
2. Helpers privados: `compound_matches` (exige `SnapshotNodeKind::Element`), `class_list_contains` (divide o atributo
   `class` por espaço em branco), `attribute_matches`, `pseudo_class_matches`, `element_siblings`,
   `element_index_one_based`, `push_candidates`.
3. `pub fn collect_style_sheets(snapshot: &DomSnapshot) -> Result<StyleSheetSet, CssError>` — `<style>` → texto dos
   filhos `Text` → `parse_stylesheet(_, Origin::Author)` → `merge`; `style=` → `parse_inline_style` → `push_inline`.

### 13. `infrastructure/cascade/`

1. `values.rs` —
   `pub(crate) fn apply_declaration(style: ComputedStyle, declaration: &Declaration) -> Option<ComputedStyle>` mais os
   conversores `parse_display`, `parse_color`, `parse_length`, `parse_length_edges`. `None` = valor fora do recorte, e a
   declaração é ignorada.
2. `author_rules.rs` — `pub(crate) fn apply_author_rules(base, node, snapshot, sheets) -> ComputedStyle`: colhe
   `(precedência, especificidade, índice)` de cada regra casada (pulando as de mídia não avaliada), ordena, aplica, e
   por fim aplica `sheets.inline_of(node.id())`.
3. `ua_sheet.rs` — `UaCascade::resolve` passa `sheets` e o `DomSnapshot` para `apply_author_rules` dentro do fecho de
   `recompute_in_document_order`.

### 14. `lib.rs`, `MANIFEST.md`, `manifest_runner.rs`, CI

1. `lib.rs` — `PORT_SCHEMA_VERSION = 2`; `SUPPORTED_PROPERTIES` com 14 entradas (as 6 de B0 +
   `margin-top/right/bottom/left`
    - `padding-top/right/bottom/left`); `SUPPORTED_SELECTORS` com as 18 formas do recorte §2.8; `pub use` de
      `parse_stylesheet`, `parse_inline_style`, `tokenize`, `Token`, `TokenStream`, `collect_style_sheets`, `matches`,
      `SelectorList`, `ComplexSelector`, `CompoundSelector`, `Combinator`, `TypeSelector`, `AttributeSelector`,
      `AttributeMatch`, `PseudoClass`, `NthFormula`, `Specificity`, `MediaQuery`, `MediaCondition`, `MediaFeature`,
      `Identifier`, `Declaration`, `DeclarationValue`, `Importance`, `ParseNote`, `ParseNotes`, `InlineStyles`.
2. `tests/data/MANIFEST.md` — duas tabelas `| token | since | notes |`.
3. `tests/manifest_runner.rs` — parser de tabela markdown; as três checagens; `UPDATE_MANIFEST` só para a tabela.
4. `.github/workflows/ci.yml` — job `css-conformance`; `justfile` — recipe `css-conformance`, incluída em `gate`.

### 15. Testes

1. `tests/parser.rs` — comentários, escapes (`\41` e a barra invertida seguida de espaço), strings com aspas simples e
   duplas, `url()` citada e não citada, `BadString`/`BadUrl`, números/dimensões/percentagens, `@media`, recuperação de
   regra e de declaração, at-rule desconhecida, `!important`, span de linha/coluna.
2. `tests/selectors.rs` — parse + `Specificity` + `matches` contra um `DomSnapshot` construído à mão, para **cada**
   forma do recorte, incluindo `:nth-child(2n+1)`, `:nth-child(odd)`, `-n+3`, e os quatro combinadores; e a rejeição de
   `:has()`, `::before`, namespace.
3. `tests/authored_style.rs` — `dom::DomTree` com `<style>` e `style=` → `snapshot` → `collect_style_sheets` →
   `CascadeResolver::resolve` → asserção de `color` e `margin`; e `#id` > `.classe` > tipo por especificidade.

## Norms

1. **Object Calisthenics mecanicamente checado (`CLAUDE.md`, `ADR-0010:127-137`)**: sem primitivo cru no domínio
   (`Identifier`, `DeclarationValue`, `Specificity`, `NthFormula`); coleções de primeira classe (`TokenStream`,
   `SelectorList`, `ClassNames`, `ElementIds`, `AttributeSelectors`, `PseudoClasses`, `DeclarationBlock`,
   `InlineStyles`, `ParseNotes`, `MediaQuery`); sem `else` (early return / `match` / `if let`; `let … else` também
   conta); **um nível de indentação por função** — é o que obriga o tokenizador a virar dez leitores focados em vez de
   um `match` gigante com laços aninhados; um dot por linha; nomes sem abreviação (`peek_ahead`, não `peek_n`;
   `index_one_based`, não `idx`); entidades < ~100 linhas; sem campo público mutável.
2. **Clippy `pedantic` + `nursery` = `deny` (`Cargo.toml:67-82`)**: nada de `unwrap`/`expect`/`panic`/`todo`/
   `unimplemented`/`unreachable` em código de lib. **`string_slice`, `indexing_slicing`, `arithmetic_side_effects` e
   `as_conversions` são `deny`** — um tokenizador escrito à mão usa `char_indices()`/iteradores, `.get(..)`,
   `checked_*`/`saturating_*`, e `u32::from`/`TryFrom`, nunca `s[i..j]`, `chars[i]`, `i + 1` ou `c as u32`.
   `#[must_use]` em todo getter/construtor puro; `const fn` onde o corpo permite; `Self` no lugar do nome do tipo.
3. **`#[allow(clippy::...)]` só em dois lugares**: o header já existente de `application/conformance.rs` e os arquivos
   de `tests/` (`#![allow(clippy::unwrap_used, clippy::expect_used)]` escopado, com o `//!` que nomeia a regra
   guardada). Nunca em `domain/`, `application/{ports,snapshot,matching,collect_sheets}.rs` ou `infrastructure/`.
4. **Erros tipados, `#[non_exhaustive]` (`ADR-0011` item 4, `ADR-0015`)**: `CssError` continua com `thiserror`; B1 usa
   `CssStage::Parse` e `CssStage::Selector`, que B0 declarou e nunca exercitou (`error.rs:17-20`), sempre com
   `SourceSpan` via `with_span`. Nenhuma variante nova é necessária.
5. **Command–Query Separation**: `Scanner::consume()` e `TokenStream::advance()` comandam e devolvem `()`;
   `peek()`/`peek_ahead()`/`span()` consultam e não mutam. Sem parâmetro booleano — `Importance`, `Combinator`,
   `AttributeMatch`, `MediaFeature`, `TypeSelector` e `Origin` são os enums nomeados que os substituem.
6. **Comentários explicam o _porquê_, citando `ADR`/`PRD`/§** — nunca o _quê_; sem código comentado. Toda decisão
   contra-intuitiva (tokenizador total, `@media` filtrado pelo produtor, `:hover` que nunca casa) leva a citação da
   linha que a justifica.
7. **`tracing`, nunca `log` (`ADR-0014`)** — B1 não precisa de diagnóstico em runtime; as `ParseNotes` são o canal.
   `#![forbid(unsafe_code)]` intocado.
8. **Testes em `tests/`, um arquivo por tema, `//!` nomeando a regra que guarda**; nunca `#[cfg(test)] mod tests` em
   `src/`.
9. **Formatação**: `cargo fmt --all` + `pnpm format:md` (tabs, largura 4, 120 colunas, `proseWrap: always`);
   `pnpm lint:md` limpo.

## Safeguards

1. **`PRD-007:11-13` (parsing é Rust nativo, não porta)**: nenhum tipo de `infrastructure/parser/` aparece em
   `application/ports.rs`; a assinatura das três portas é byte-idêntica à de B0. `cargo tree -p css` continua sem
   dependência nova.
2. **Entregável do plano `:417` (`<style>` / `style=` observáveis)**: `tests/authored_style.rs` monta um `DomTree` com
   `<style>p { color: #0000ff; margin: 4px }</style>` e um `<p style="color: #ff0000">`, roda
   `snapshot → collect_style_sheets → UaCascade::resolve`, e asserta que o `color` computado é o inline e a `margin` é a
   da folha — falha se qualquer elo da corrente parar de aplicar.
3. **Especificidade como desempate (`relatório §2.8:334`)**: o mesmo teste asserta `#id` > `.classe` > tipo com as três
   regras declarando a mesma propriedade na mesma folha, em ordem de documento **desfavorável** ao vencedor — de forma
   que só a especificidade possa decidir.
4. **Recorte §2.8 nos dois sentidos**: `cargo test -p css --test manifest_runner` compara manifesto ⇄ registros ⇄
   parser. Verificado à mão uma vez: apagar uma entrada de `SUPPORTED_PROPERTIES` sem tocar o `MANIFEST.md` **falha** o
   runner (e o inverso também), e a alteração é revertida.
5. **O que está fora é rejeitado, não ignorado**: o runner alimenta `:has(p)`, `p::before`, `svg|rect`,
   `@supports (display: flex) { … }`, `@font-face { … }`, `@import url(x)` e `@keyframes k { … }`, e asserta que cada um
   vira regra descartada **com `ParseNote`** — nunca uma regra aplicada nem uma folha abortada.
6. **`PRD-007:98,100` (determinismo, 100 runs)**: `run_css_conformance(&UaCascade::new(), &BlockLayout::new())` e a
   versão com mocks continuam verdes em `tests/css_conformance.rs`; a chave de ordenação
   `(precedência, especificidade, índice)` é total, então não há empate a ser desempatado por ordem de `HashMap`.
7. **`PRD-007:83-84` (sem tipo estrangeiro)**: `collect_style_sheets` recebe `&DomSnapshot`; `application/snapshot.rs`
   segue o único arquivo de `core/css` que nomeia `dom::DomTree` / `dom::NodeId` — verificável por
   `grep -rn "dom::" core/css/src`.
8. **Robustez contra entrada hostil**: `tests/parser.rs` alimenta string sem terminador, `url(` sem `)`, `\` no fim da
   fonte, `{` sem par, `}` órfão, `@media` sem bloco e aninhamento profundo; nenhum caso entra em laço infinito, estoura
   índice ou entra em pânico — o aninhamento além de `MAX_NESTING_DEPTH` devolve `CssError { stage: Parse, span }`.
9. **`ADR-0002` / `PRD-001:99`**: `just no-engine` verde — `cargo tree -p css` sem `engine`, `rhai`, `rhai-runtime`,
   `rhai-bindings`.
10. **DoD completo**: `just gate` verde (`fmt-check` + `lint` + `check` + `test` + `deny` + `coverage` + `arch` +
    `no-engine`), `just no-engine` verde, `cargo test -p css` **e** `cargo test -p css --no-default-features` verdes,
    `cargo test -p css --test manifest_runner` verde, `cargo fmt --all` + `pnpm format:md` aplicados, `pnpm lint:md`
    limpo. Dois commits: o par de canvas SPDD, depois
    `feat(css): CSS Syntax L3 tokenizer, selector engine and specificity (v0.5 B1)`, ambos com os trailers do repo. Sem
    push, sem PR, sem tocar `main`, `core/network`, `core/window`, `core/html` ou qualquer arquivo de outra fase.
