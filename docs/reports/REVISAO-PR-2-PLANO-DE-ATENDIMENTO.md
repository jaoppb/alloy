# Revisão da PR #2 — plano de atendimento da review e do redesenho da superfície de script

| Campo          | Valor                                                                                                             |
| -------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Status**     | ❌ Não iniciado — os 37 comentários inline estão abertos e nenhum foi resolvido                                   |
| **Cobertura**  | ~0% (0 de 40 itens: 37 comentários inline + 2 pontos globais da review + 1 pedido novo)                           |
| **Esforço**    | 22,5–37 dias-dev `[modelado]` no escopo completo; 9,75–13,25 d no mínimo que destrava o merge                     |
| **Depende de** | Nada — F0 pode começar hoje                                                                                       |
| **Bloqueia**   | O merge de `feat/v1-roadmap-implementation` em `main` (`reviewDecision: CHANGES_REQUESTED`)                       |
| **Atenção**    | ⚠️ As 8 bindings de script **não são alcançáveis por script nenhum** — provado na seção 1, e bloqueia N-01 e G-02 |

---

## 1. Estado atual — evidências

### O que a PR entrega hoje

A branch `feat/v1-roadmap-implementation` tem 10 commits sobre `main` e um diff de 125 arquivos, 9.803 inserções e 339
remoções. `cargo test --workspace` no commit `35d35d6` produz **48 testes passando** em 32 suítes `test result: ok` —
nenhuma falha, nenhum teste ignorado.

O pipeline `HtmlStream → DomTree → StyledTree → DisplayList → RenderBackend` está inteiro e é executável de ponta a
ponta pelo subcomando `alloy render` (`alloy/src/main.rs:36-56`), que lê HTML, faz cascata de CSS, gera `DisplayList` e
grava PNG via `SoftwareCpuBackend`.

### O que a review pede

A review de `jaoppb` está registada como `CHANGES_REQUESTED`, submetida em `2026-08-26T21:43:59Z` — **depois** do último
commit da branch, que é de `2026-08-26 10:41:41 -0300` (13:41 UTC). Nenhum comentário foi endereçado por código
posterior:

```bash
gh api graphql -f query='{... reviewThreads(first:60){nodes{isResolved}}}'
# → resolved=false count=37
```

Além dos 37 comentários inline, o corpo da review carrega dois pedidos globais:

1. Substituir a implementação manual dos enums de erro por `thiserror`, e criar um ADR para isso.
2. Mover para `rhai` a lógica que hoje está em Rust — Rust define entidades e portas, `rhai` define comportamento, de
   modo que o utilizador possa alterar qualquer comportamento do browser.

O segundo é a reafirmação direta do ADR-0003 (Skeleton and Muscle) e não é negociável como escopo — é negociável apenas
como cronograma.

### O pedido adicional, fora da review

Além da review, há um pedido direto: **trocar o método de acesso de todas as funções expostas a script nos runtimes,
agrupando-as obrigatoriamente por namespace / objetos relacionados, e não apenas na parte do DOM** —
`document.createElement(…)` no lugar de `dom_create_element(…)`, `renderer.pushRect(…)` no lugar de
`graphics_push_rect(…)`, e assim por diante para qualquer subsistema exposto. Nenhuma função deve ser registrada ou
exposta de forma plana no escopo global. Está registado abaixo como **N-01** e não veio de `jaoppb`.

O pedido é correto e chega na hora certa: qualquer migração de política para `rhai` (G-02) vai multiplicar o número de
bindings, e multiplicar um namespace plano e sem módulos é o momento errado para descobrir que ele não escala e polui o
ambiente de execução.

### ⚠️ Defeito encontrado: as 8 bindings não são alcançáveis por script

Antes de discutir como agrupar as funções expostas, há um facto que muda o enquadramento de N-01 e de G-02: **nenhuma
das 8 bindings é chamável a partir de um script `.rhai`**.

`RhaiContext::register_fn` (`core/runtime/rhai/src/application/context.rs:46-49`) apenas insere num
`HashMap<String, NativeFn>` próprio do contexto (`:9`). O `rhai::Engine` nunca é tocado:

```rust
// core/runtime/rhai/src/application/context.rs:46-49
fn register_fn(&mut self, name: Identifier, f: NativeFn) -> Result<(), EngineError> {
    self.functions.insert(name.as_str().to_string(), f);
    Ok(())
}
```

E `RhaiEngine::eval` (`core/runtime/rhai/src/application/engine.rs:69-81`) avalia com
`self.engine.eval_with_scope(context.scope_mut(), script)` — o `HashMap` de `functions` **nunca é consultado**. O único
leitor daquele mapa é `call_function` (`context.rs:64-76`), que é a API de Rust, não de script.

Confirmei empiricamente com um teste temporário em `core/dom/tests/`, executado e depois removido:

```text
// eval::<EngineValue>(&mut ctx, r#"dom_create_element("div")"#)
PROBE RESULT: Err(FunctionNotFound("dom_create_element (&str | ImmutableString | String)"))
```

A suíte não apanha isto porque **todos os testes chamam as bindings pelo lado de Rust**:
`core/dom/tests/script_dom_mutation.rs:27,35,43,51,60,92` e `core/graphics/tests/graphics_conformance.rs:75,88,100` usam
`ctx.call_function(...)`, nunca `engine.eval(...)`. Os 48 testes ficam verdes enquanto a funcionalidade voltada ao
utilizador não existe.

O ficheiro `scripts/hello.rhai` — o único script do repositório — tem 3 linhas e só concatena strings, portanto também
não denuncia o problema.

Isto não é causado pela review nem por N-01. Mas **N-01 é inimplementável enquanto durar**: não há como agrupar em
objetos funções que nenhum script consegue ver.

### ⚠️ Os comentários são uma amostra, não um inventário

O reviewer aponta `pub` fields em quatro locais (`css/src/domain/property.rs:5`, `css/src/domain/rule.rs:8`,
`graphics/src/domain/geometry.rs:20` e `:37`). A busca mostra que o problema é seis vezes maior:

```bash
grep -rn "pub [a-z_]*:" core/*/src/domain/*.rs
# → 30 campos públicos em 6 arquivos
```

São `declaration.rs:6-7`, `computed.rs:6-19` (14 campos), `rule.rs:7-8`, `geometry.rs:4-5,19-20,34-37`,
`hot_reload.rs:56-57` e `specificity.rs:7-9`. Mais dois tuple-structs com campo público: `Px(pub f32)` e
`Color(pub u32)` em `property.rs:5,29`. Corrigir só os quatro apontados garante uma segunda rodada de review sobre
exatamente o mesmo defeito.

O mesmo vale para o `else`, proibido pelo ADR-0010: `grep -rn "} else {"` devolve **9 ocorrências**, em
`hot_reload.rs:168`, `sandbox.rs:51,75`, `entities.rs:42,46`, `main.rs:77,112` e dois testes. Nenhuma delas foi
comentada na review.

### Duas divergências pré-existentes que estão no caminho

`CLAUDE.md` declara `core/engine` como **"Zero dependencies — pure abstraction"**. O manifesto real tem duas: `bitflags`
e `notify` (`core/engine/Cargo.toml:11-12`). O comentário sobre `notify` na camada `application` (`hot_reload.rs:175`) é
o sintoma; a violação está declarada no manifesto.

`thiserror` não existe no workspace:

```bash
grep -rn "thiserror" Cargo.toml core/*/Cargo.toml core/runtime/*/Cargo.toml alloy/Cargo.toml
# → 0 resultados
```

Existem **5 enums de erro** com `impl fmt::Display` escrito à mão: `dom/domain/error.rs:22-36`, `css/domain/error.rs`,
`engine/domain/error.rs`, `graphics/domain/error.rs` e `html/domain/token.rs`.

Nenhuma das duas divergências foi causada pela review — mas as duas passam pelo código que ela manda reescrever, e o
custo de corrigi-las junto é próximo de zero.

---

## 2. Os 40 pontos, mapeados

### 2.1 Inventário completo

Esforço em dias-dev, `[modelado]` em toda a coluna.

| #        | Local                                                                     | Pedido                                                                                     | Tema                 | Esforço   |
| -------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | -------------------- | --------- |
| **C-01** | `.github/workflows/ci.yml:18`                                             | `actions/checkout` → v7                                                                    | CI                   | 0,05 d    |
| **C-02** | `ci.yml:21`                                                               | `pnpm/action-setup` → v6                                                                   | CI                   | 0,05 d    |
| **C-03** | `ci.yml:26`                                                               | `actions/setup-node` → v7                                                                  | CI                   | 0,05 d    |
| **C-04** | `ci.yml:28`                                                               | `node-version` 22 → 24                                                                     | CI                   | 0,05 d    |
| **C-05** | `ci.yml:53`                                                               | `actions/checkout` → v7                                                                    | CI                   | —         |
| **C-06** | `ci.yml:76`                                                               | `actions/checkout` → v7                                                                    | CI                   | —         |
| **C-07** | `lefthook.yml:18`                                                         | remover `npx`                                                                              | Tooling              | 0,05 d    |
| **C-08** | `lefthook.yml:23`                                                         | remover `npx`                                                                              | Tooling              | —         |
| **C-09** | `spdd/prompt/.gitkeep:1`                                                  | apagar o arquivo                                                                           | Tooling              | 0,05 d    |
| **C-10** | `alloy/src/main.rs:131`                                                   | `process::exit` → `Result`                                                                 | Erros                | 0,5 d     |
| **C-11** | `main.rs:169`                                                             | idem em `execute_script`                                                                   | Erros                | —         |
| **C-12** | `css/application/cascade.rs:22`                                           | `match` em vez de early return                                                             | Estilo               | 0,1 d     |
| **C-13** | `cascade.rs:43`                                                           | lista de tags inline é frágil → função de domínio                                          | Camadas              | 0,3 d     |
| **C-14** | `css/domain/property.rs:5`                                                | sem campo `pub`                                                                            | Encapsulamento       | 0,3 d     |
| **C-15** | `property.rs:98`                                                          | parsing de cor → `infrastructure` (preparar `rebeccapurple`)                               | Portas               | 0,5 d     |
| **C-16** | `property.rs:122`                                                         | `parse_hex` junto da porta acima                                                           | Portas               | —         |
| **C-17** | `property.rs:128`                                                         | `PropertyName` → enum W3C CSS3                                                             | Enums W3C            | 1–1,5 d   |
| **C-18** | `property.rs:155`                                                         | mapear `Keyword` e variantes como enums                                                    | Enums W3C            | 1–1,5 d   |
| **C-19** | `css/domain/rule.rs:8`                                                    | sem campo `pub`; garantir `selectors.len() > 0`                                            | Encapsulamento       | 0,3 d     |
| **C-20** | `css/domain/selector.rs:17`                                               | `Selector` completo CSS3: pseudo, `>`, etc.                                                | Enums W3C            | 1,5–2 d   |
| **C-21** | `css/application/parser.rs:1`                                             | parsing de CSS pertence ao `domain`                                                        | Camadas              | 0,3 d     |
| **C-22** | `dom/domain/attribute.rs:6`                                               | `AttributeName` → enum W3C                                                                 | Enums W3C            | 0,5–1 d   |
| **C-23** | `dom/domain/error.rs:1`                                                   | usar `thiserror` + criar ADR                                                               | Erros                | 0,5–1 d   |
| **C-24** | `dom/application/service.rs:1`                                            | serialização pertence ao `domain`                                                          | Camadas              | 0,2 d     |
| **C-25** | `dom/domain/tag_name.rs:7`                                                | `TagName` → enum W3C                                                                       | Enums W3C            | 1–1,5 d   |
| **C-26** | `dom/domain/tree.rs:235`                                                  | `validate_exists` denuncia mau modelo; usar `?` e métodos tudo-ou-nada                     | Arena                | 0,5 d     |
| **C-27** | `tree.rs:13`                                                              | arena com `generation`, `Slot<T>` e ADR                                                    | Arena                | 1–1,5 d   |
| **C-28** | `dom/infrastructure/bridge.rs:1`                                          | usar o helper de capability; mais extensibilidade                                          | Sandbox              | 0,5 d     |
| **C-29** | `engine/application/hot_reload.rs:175`                                    | `notify` acoplado ao `application` → porta + adaptador                                     | Portas               | 0,5–1 d   |
| **C-30** | `graphics/application/cpu_backend.rs:12`                                  | criar VO `Rect`                                                                            | VOs                  | 0,25 d    |
| **C-31** | `cpu_backend.rs:28`                                                       | criar VO `Position`                                                                        | VOs                  | 0,25 d    |
| **C-32** | `graphics/application/layout.rs:1`                                        | layout pertence ao `domain`                                                                | Camadas              | 0,3 d     |
| **C-33** | `graphics/domain/geometry.rs:20`                                          | sem campo `pub`; impor `f32 >= 0`                                                          | Encapsulamento       | 0,3 d     |
| **C-34** | `geometry.rs:37`                                                          | `Rect` como `origin: Point` + `size: Size`                                                 | VOs                  | 0,5 d     |
| **C-35** | `html/domain/entities.rs:41`                                              | enum de entidades + `TryFrom`                                                              | Enums W3C            | 0,5 d     |
| **C-36** | `html/domain/tree_builder.rs:36`                                          | implementar o DOCTYPE, não ignorar                                                         | Conformidade         | 0,3 d     |
| **C-37** | `tree_builder.rs:130`                                                     | `is_void(&self) -> bool` no enum                                                           | Enums W3C            | 0,25 d    |
| **G-01** | review body                                                               | `thiserror` em todos os enums de erro + ADR                                                | Erros                | (C-23)    |
| **G-02** | review body                                                               | mover lógica de Rust para `rhai`                                                           | Skeleton/Muscle      | 8–15 d    |
| **N-01** | `dom/infrastructure/bridge.rs` · `graphics/infrastructure/rhai_bridge.rs` | agrupar todas as funções expostas nos runtimes em namespaces/objetos (DOM, renderer, etc.) | Superfície de script | 2,5–3,5 d |
| **D-01** | `runtime/rhai/src/application/context.rs:46-49`                           | bindings não chegam ao `rhai::Engine` — pré-requisito de N-01 e G-02                       | Defeito              | 1–2 d     |

As linhas com esforço `—` são duplicatas do mesmo edit da linha anterior e não somam custo. **N-01** não veio da review;
**D-01** não veio de ninguém — é o defeito que a seção 1 prova e que N-01 destapa.

### 2.2 Os seis pedidos de versão de CI estão corretos

Verifiquei cada um contra a release mais recente publicada:

```bash
gh api repos/actions/checkout/releases/latest -q .tag_name        # → v7.0.1
gh api repos/actions/setup-node/releases/latest -q .tag_name      # → v7.0.0
gh api repos/pnpm/action-setup/releases/latest -q .tag_name       # → v6.0.10
```

O workflow usa `actions/checkout@v4` em três jobs (`ci.yml:18,53,76`), `pnpm/action-setup@v4` (`:21`) e
`actions/setup-node@v4` (`:26`). Os quatro estão **duas ou três majors atrás**. O pedido de `node-version: 24` (`:28`)
alinha com o LTS ativo; o ambiente local já roda `v26.7.0`.

`Swatinem/rust-cache@v2` (`:40,61`) e `EmbarkStudios/cargo-deny-action@v2` (`:77`) já estão na major corrente (`v2.9.2`
e `v2.1.1`) e não foram comentados — corretamente.

O `npx` em `lefthook.yml:18,23` é redundante duas vezes: `npx pnpm prettier` invoca `npx` para achar `pnpm` para achar
`prettier`. A forma correta é `pnpm prettier --write {staged_files}`.

### 2.3 `thiserror` — decisão de ADR, não de refactor

O pedido G-01 exige um ADR novo. Há duas opções reais:

| Opção                           | O que é                                    | Custo                       | Veredito                                             |
| ------------------------------- | ------------------------------------------ | --------------------------- | ---------------------------------------------------- |
| `thiserror` em todos os crates  | Derive macro; `Display` e `source` gerados | +1 dep de build em 5 crates | **Recomendada** — é literalmente o que a review pede |
| `thiserror` só fora do `engine` | Preserva o "zero dependencies" do `engine` | Inconsistência entre crates | Rejeitada: `engine` já tem `bitflags` e `notify`     |

A segunda opção defenderia uma pureza que o `Cargo.toml` já não tem. O ADR-0011 deve, no mesmo documento, **corrigir a
linha do `CLAUDE.md`** que chama `core/engine` de "zero dependencies", porque ela é falsa desde o commit `0fd86c4`.

`thiserror` é `proc-macro` e entra em `[dependencies]` de 5 crates. `deny.toml` precisa aceitar a licença dual
`MIT OR Apache-2.0` — já é o padrão do workspace, então não há trabalho extra ali.

### 2.4 Enums W3C — o bloco mais caro depois de G-02

Cinco comentários (C-17, C-18, C-20, C-22, C-25) pedem a mesma transformação: trocar `struct X(String)` por um `enum`
fechado derivado da spec do W3C. O impacto não é uniforme:

`TagName` (`dom/domain/tag_name.rs:7`) é hoje `String` normalizada com validação de não-vazio (`:14-22`). HTML5 tem ~110
elementos. Um enum fechado **quebra** `TagName::new()` como construtor infalível e obriga uma variante `Custom(String)`
para web components — sem ela, o parser rejeita `<my-widget>`, que é HTML válido.

`AttributeName` (`dom/domain/attribute.rs:6`) tem o mesmo problema, agravado: `data-*` e `aria-*` são famílias abertas
por definição. O enum precisa de `Data(String)` e `Aria(String)`.

O comentário C-22 diz "use W3C CSS3 as the standard to map `AttributeName`" — **CSS3 não define nomes de atributo
HTML**. A referência correta é o HTML Living Standard. É um lapso do texto do comentário, não do pedido; o pedido em si
é válido.

`Selector` (`css/domain/selector.rs:6-17`) hoje tem 5 variantes: `Universal`, `Tag`, `Class`, `Id`, `Descendant`. Faltam
`>`, `+`, `~`, seletores de atributo, pseudo-classes e pseudo-elementos. Isto não é um refactor — é **implementar o
Selectors Level 3**, incluindo o parser correspondente em `css/application/parser.rs` e a especificidade de cada família
em `specificity.rs`.

`is_void_element` (`html/domain/tree_builder.rs:112-130`) só vira `is_void(&self)` depois que `TagName` for enum. C-37
**depende** de C-25 e não pode ser feito antes.

### 2.5 Vazamento de camada — quatro comentários, um padrão

C-13, C-21, C-24 e C-32 apontam o mesmo erro: regra de domínio escrita em `application/`.

O caso mais claro é `cascade.rs:40-46`, apontado por C-13:

```rust
// core/css/src/application/cascade.rs:40-46
if tag_name.as_str() == "span"
    || tag_name.as_str() == "a"
    || tag_name.as_str() == "b"
    || tag_name.as_str() == "i"
{
    computed.display = DisplayType::Inline;
}
```

Quatro comparações de string cruas decidindo o modelo de formatação default do HTML. É frágil como o reviewer diz — e é
também exatamente o tipo de política que G-02 quer em `rhai`, não em Rust. Resolver C-13 movendo para
`TagName::default_display()` no domínio é correto **como passo intermediário**, mas o destino final é um script.

`css/application/parser.rs:1-191` e `graphics/application/layout.rs:1-104` são serviços puros, sem I/O e sem porta:
`parse_stylesheet` (`:17`) recebe `&str` e devolve `Result<StyleSheet, CssError>`; `LayoutEngine::layout` (`:13-18`)
recebe três aggregates e devolve `DisplayList`. Nenhum dos dois toca o sistema operativo. Estão na camada errada e a
mudança é uma movimentação de arquivo mais ajuste de `mod.rs` e reexport em `lib.rs` — barata.

`DomService::serialize_to_html` (`dom/application/service.rs:48-97`) é o mesmo caso: serialização é conhecimento do
modelo, não orquestração.

### 2.6 Portas e adaptadores — `notify` e o parser de cor

C-29 é a violação de dependência mais séria da PR. `hot_reload.rs:3` importa
`notify::{Event, RecursiveMode, Result as NotifyResult, Watcher}` e `:114` guarda um
`Option<notify::RecommendedWatcher>` **como campo de um struct de `application`**. A regra do ADR-0010 é que
`application` define portas e `infrastructure` as implementa.

A correção é uma trait `FileWatchPort` em `engine/application/ports.rs` — arquivo que já existe — e um
`NotifyFileWatcher` em `engine/infrastructure/`. O `debounce` (`hot_reload.rs:156-161`) é regra de negócio e **fica** na
aplicação; só o mecanismo de observação atravessa a porta.

C-15 e C-16 pedem o mesmo para cor. `Color::parse` (`property.rs:78-98`) tem uma tabela de 8 cores nomeadas embutida no
domínio (`:81-91`) e `parse_hex` (`:100-123`) logo abaixo. O reviewer cita `rebeccapurple`: a lista CSS Color 4 tem 148
nomes. Uma tabela de 148 entradas não pertence a um value object — pertence a um adaptador atrás de uma porta
`ColorResolver`.

### 2.7 Arena geracional — ADR e efeito dominó

C-27 pede duas coisas concretas sobre `DomTree` (`dom/domain/tree.rs:10-13`): `nodes: Vec<Option<DomNode>>` vira
`Vec<Slot<DomNode>>`, e `NodeId` passa a carregar `index: u32` + `generation: u32` para eliminar o problema ABA — hoje,
remover um nó e alocar outro **reaproveita o índice em silêncio** e qualquer `NodeId` antigo passa a apontar para o nó
novo.

`allocate_node` (`:222-228`) hoje só faz `push` e nunca reusa slot, então o bug ABA ainda não é alcançável — mas a
assinatura pública `NodeId::new(index)` já promete o contrário, e a primeira implementação de `remove_node` o
materializa.

C-26 é o corolário: `validate_exists` (`:230-235`) é chamado **7 vezes** em `tree.rs:107,108,141,142,143,182,183`,
sempre em pares ou trios antes de uma mutação. Isso é uma pré-condição repetida à mão porque o modelo não sabe resolver
múltiplos IDs de uma vez. Com `Slot<T>` e geração, o padrão certo é um
`resolve_all(&[NodeId]) -> Result<[&DomNode], DomError>` com `?`.

`NodeId` atravessa quase tudo: `css/domain/selector.rs:36`, `dom/infrastructure/bridge.rs`,
`graphics/application/layout.rs`. Mudar a sua representação interna é seguro **se** `NodeId::index()` continuar
existindo — as bindings de script em `bridge.rs:43` já dependem dele para marshalar `EngineValue::Int`.

### 2.8 G-02 — mover lógica para `rhai` é o pedido que redefine o cronograma

A superfície exposta a script hoje é de **8 funções**: 5 em `dom/infrastructure/bridge.rs` (`dom_create_element`,
`dom_create_text`, `dom_append_child`, `dom_get_text`, `dom_set_text`) e 3 em `graphics/infrastructure/rhai_bridge.rs`
(`graphics_push_rect`, `graphics_get_len`, `graphics_serialize_json`). Não há nenhuma binding de CSS, de HTML ou de
hot-reload.

O único script do repositório é `scripts/hello.rhai`, com 3 linhas, e não chama nenhuma dessas 8 funções — só concatena
strings. E não poderia chamar: como a seção 1 prova, **nenhuma delas chega ao `rhai::Engine`**. A superfície real
disponível a um autor de script hoje é de **zero funções**.

A política que hoje está cravada em Rust e que G-02 quer mover:

| Política                     | Onde está hoje                                                       | Alvo                            |
| ---------------------------- | -------------------------------------------------------------------- | ------------------------------- |
| Tags que renderizam `inline` | `css/application/cascade.rs:40-46`                                   | script de default stylesheet    |
| Elementos void do HTML5      | `html/domain/tree_builder.rs:112-130`                                | tabela de conformidade + script |
| Tabela de cores nomeadas     | `css/domain/property.rs:81-91`                                       | adaptador `ColorResolver`       |
| Tabela de entidades HTML     | `html/domain/entities.rs:29-41`                                      | adaptador de conformidade       |
| Margens e viewport do layout | `graphics/application/layout.rs:20-35` (`10.0`, `viewport_w - 20.0`) | script de layout policy         |
| Tratamento do DOCTYPE        | `tree_builder.rs:33-37` (ignorado)                                   | script de modo de renderização  |

Nada disso é alcançável com 8 bindings. G-02 exige, antes de qualquer migração, **a superfície de bindings que hoje não
existe**: CSS, layout e política de conformidade precisam de portas expostas ao contexto de script, cada uma com a sua
`Capability` correspondente.

É por isso que G-02 custa 8–15 d `[modelado]` e não cabe nesta PR — e por isso D-01 e N-01 são pré-requisitos dele, não
trabalho paralelo: migrar política para scripts que não conseguem chamar o host é impossível, e migrá-la para um
namespace plano de 40+ funções é trabalho que se refaz.

### 2.9 N-01 — agrupar toda a superfície de script em namespaces / objetos (regra universal dos runtimes)

A superfície atual exposta a script é **completamente plana e prefixada à mão em todos os subsistemas**. Os 8 nomes
registados no workspace hoje são:

- **DOM** (`dom/infrastructure/bridge.rs:21,52,81,124,158`): `dom_create_element`, `dom_create_text`,
  `dom_append_child`, `dom_get_text`, `dom_set_text`
- **Graphics** (`graphics/infrastructure/rhai_bridge.rs:43,65,75`): `graphics_push_rect`, `graphics_get_len`,
  `graphics_serialize_json`

O prefixo manual (`dom_*`, `graphics_*`) é o único mecanismo de agrupamento que existe — é mera convenção de string, sem
nenhum suporte estrutural do modelo de domínio, da porta ou do runtime.

#### A regra mandatória: agrupamento por namespace em todas as funções dos runtimes

O pedido N-01 não se restringe à API do DOM: **toda e qualquer função registrada nos runtimes deve obrigatoriamente
pertencer a um namespace ou objeto agrupador (`HostObject`), abolindo completamente o registro de funções planas no
escopo global raiz**.

A exigência é universal para todo e qualquer subsistema do Alloy pelos seguintes motivos arquiteturais:

1. **Poluição e colisão de escopo global**: Em qualquer runtime de scripts, funções nativas soltas na raiz contaminam o
   ambiente de execução do utilizador e geram colisões inevitáveis entre subsistemas independentes (ex.: dois módulos
   precisarem expor `clear`, `reset` ou `serialize`).
2. **Não há exceções entre subsistemas**: O agrupamento não pode ser um tratamento especial dado apenas ao DOM enquanto
   Graphics, CSS ou Layout continuam com funções globais prefixadas (`graphics_push_rect`, etc.). Essa assimetria
   destruiria a coerência da API do browser.
3. **Governança homogênea de permissões (`Capability`)**: Quando as funções são soltas, a segurança depende de decorar
   cada binding individual com checagens repetitivas (`guarded_native_fn`). Agrupando por namespace/objeto, a
   `Capability` passa a ser governada no contêiner ou em métodos dentro de um escopo semântico delimitado.
4. **Neutralidade de backends de runtime (ADR-0002)**: No ecossistema web normativo e em runtimes JavaScript (motores
   futuros como QuickJS, V8 ou Boa), as APIs do host vivem estritamente sob objetos e namespaces padronizados
   (`document`, `console`, `CSS`, etc.). Manter funções soltas no `rhai` criaria uma interface anômala que exigiria
   shims frágeis em cada troca ou adição de backend.
5. **Escalabilidade para G-02**: Ao migrar dezenas de políticas de CSS, Layout e conformidade HTML de Rust para script,
   despejar mais de 40 funções no escopo global tornaria a superfície de script ingovernável.

#### A causa arquitetural está na porta `core/engine`

O problema nasce na própria trait `ExecutionContext` (`core/engine/src/application/ports.rs:16-53`), que hoje expõe
apenas o registo plano e desestruturado:

```rust
// core/engine/src/application/ports.rs:24 — API ATUAL (DEFEITUOSA)
fn register_fn(&mut self, name: Identifier, f: NativeFn) -> Result<(), EngineError>;
```

Um `Identifier` (`engine/src/domain/identifier.rs:16-24`) valida apenas não-vazio e faz `trim` — aceita qualquer string,
inclusive `"document.createElement"`, mas isso produziria um **nome plano contendo um ponto**, não um objeto chamável. E
`set_variable` (`ports.rs:30`) recebe `EngineValue`, cujo `Object` é `HashMap<String, EngineValue>`
(`engine/src/domain/value.rs:15`) — um mapa de **dados passivos**, sem variante capaz de guardar ou despachar uma
`NativeFn`. Não há como colocar `document` ou `renderer` no escopo com métodos chamáveis através da porta atual.

Conclusão: **N-01 é uma reformulação da porta em `core/engine` e do contrato de todos os runtimes**, e não um ajuste
pontual em bridges individuais.

#### Mecanismos de agrupamento no runtime (`rhai` 1.26.0)

Verifiquei os três mecanismos de agrupamento do `rhai` 1.26.0 com um programa descartável, executado e removido. Os três
funcionam:

| Opção                      | Sintaxe no script                                        | Prova             | Veredito                                                   |
| -------------------------- | -------------------------------------------------------- | ----------------- | ---------------------------------------------------------- |
| Tipo customizado + métodos | `document.createElement("div")` · `renderer.pushRect(…)` | `Ok(3)` · `Ok(1)` | **Recomendada** — padrão idiomático de objetos e host APIs |
| Módulo estático            | `dom::create_element("div")`                             | `Ok(3)`           | Rejeitada: `::` é sintaxe de módulo, não de objeto         |
| Object map com closures    | `api.f("div")`                                           | `Ok(3)`           | Rejeitada: o mapa é dado do script, não superfície do host |

A opção recomendada usa `Engine::register_type_with_name::<T>(name)` (`rhai-1.26.0/src/api/register.rs:220`),
`register_fn` com `&mut T` no primeiro parâmetro para método de instância, e `register_get` / `register_set` (`:305`)
para propriedades.

Essa abordagem é a **única que sobrevive diretamente à troca de backend**:

- Em `rhai`, é exposta via tipos customizados e submódulos com métodos.
- Em um runtime JS, traduz-se diretamente para objetos globais (`globalThis.document`, `globalThis.renderer`),
  preservando a sintaxe idêntica para o autor do script.

#### A porta neutra: `HostObject` universal e abolição de funções globais

Para garantir formalmente que **nenhuma função seja registrada fora de um namespace**, a porta `ExecutionContext` deve
**substituir o `register_fn` solto** por um conceito neutro de **objeto/namespace de host**:

```rust
// core/engine/src/application/ports.rs — proposta para a porta neutra
fn register_host_object(&mut self, object: HostObject) -> Result<(), EngineError>;
```

O struct `HostObject` fica em `core/engine/src/domain/` e consolida:

1. `namespace`: `Identifier` do namespace/objeto (ex.: `"document"`, `"renderer"`, `"css"`).
2. `capability`: a `Capability` que governa o acesso ao namespace ou objeto como um todo.
3. `methods`: coleção de pares `(Identifier, NativeFn)` pertencentes àquele namespace.
4. `properties`: coleção de propriedades com getters/setters tipados.

A `Capability` sobe da função isolada para o objeto/namespace, o que resolve C-28 estruturalmente: em vez de repetir
`guarded_native_fn` em cada registo, o guard passa a ser propriedade declarativa do contêiner.

#### Inventário completo de namespaces para todos os subsistemas

O agrupamento em namespaces abrange toda a superfície de script, presente e futura:

| Namespace / Objeto            | Subsistema           | Métodos e propriedades principais                                                 | Capability exigida        |
| ----------------------------- | -------------------- | --------------------------------------------------------------------------------- | ------------------------- |
| `document`                    | DOM (Host Root)      | `createElement`, `createTextNode`, `createComment`, `getElementById`, `body`      | `DOM_MUTATE` / `DOM_READ` |
| `Node` (métodos de instância) | DOM (Entidade)       | `appendChild`, `removeChild`, `textContent` (get/set), `parentNode`, `childNodes` | `DOM_READ` + `DOM_MUTATE` |
| `renderer`                    | Graphics Engine      | `pushRect`, `commandCount`, `toJSON`, `clear`                                     | `GRAPHICS_DRAW`           |
| `css`                         | CSS Engine (G-02)    | `parseStylesheet`, `matchSelector`, `computeStyles`                               | `CSS_STYLE`               |
| `layout`                      | Layout Engine (G-02) | `computeLayout`, `getViewport`, `setViewport`                                     | `LAYOUT_COMPUTE`          |
| `html`                        | HTML Engine (G-02)   | `parseFragment`, `isVoidElement`, `lookupEntity`                                  | `HTML_PARSE`              |
| `console`                     | Diagnóstico / Logs   | `log`, `warn`, `error`                                                            | `SYSTEM_LOG`              |

#### Convenções obrigatórias da superfície de script

1. **camelCase uniforme em todos os namespaces**: Todos os métodos e propriedades expostos aos scripts devem usar
   rigorosamente camelCase (`createElement`, `pushRect`, `matchSelector`, `computeLayout`), alinhando o Alloy aos
   padrões do DOM e da web. Essa regra deve ser formalizada no ADR-0012.
2. **Eliminação de inteiros primitivos expostos**: `NodeId` hoje cruza a fronteira como `EngineValue::Int`
   (`bridge.rs:43,72`). Com instâncias de `Node` encapsuladas como tipos de host, o script manipula objetos opacos,
   eliminando completamente a aritmética de inteiros sobre ponteiros de nós no lado do script.

### 2.10 O que não fazer

**Não fechar `TagName` e `AttributeName` sem variante aberta.** Um enum sem `Custom(String)` / `Data(String)` quebra web
components e `data-*`, que são HTML válido. O parser passaria a rejeitar documentos que hoje aceita, e `html_parsing.rs`
denunciaria isso na hora.

**Não fazer G-02 dentro da PR #2.** A PR já tem 125 arquivos. Somar a superfície de bindings de CSS e layout torna a
review impossível de fazer com atenção — o que produziria uma terceira rodada pior que a segunda.

**Não corrigir só os 4 `pub` fields apontados.** São 30. Corrigir 4 garante que a próxima review aponte os outros 26.

**Não agrupar as bindings antes de corrigir D-01.** Reorganizar em `document`/`renderer` uma superfície que nenhum
script alcança produz uma API bonita e morta, e o teste que a cobriria continuaria a passar pelo lado de Rust — sem
provar nada.

**Não resolver N-01 só nos dois bridges.** O agrupamento tem de nascer na porta `ExecutionContext`. Fazer o `rhai`
registar tipos customizados por fora, sem passar pela trait, viola o ADR-0002 e prende a superfície de script ao `rhai`
para sempre.

**Não permitir funções soltas ou planas no escopo global de nenhum runtime.** O agrupamento por namespace não é uma
concessão restrita ao DOM (`document`): manter `graphics_push_rect` ou qualquer função futura de CSS/Layout diretamente
na raiz do engine quebra a coerência arquitetural e polui o ambiente de execução. Toda função exposta a script deve
viver estritamente sob o seu respectivo namespace (`document`, `renderer`, `css`, etc.).

**Não manter `register_fn` plano global na porta `ExecutionContext`.** Permitir que a trait de abstração continue
aceitando funções sem namespace associado deixaria brechas para que subsistemas futuros ou testes voltem a criar
bindings planas por conveniência, quebrando a integridade estrutural.

**Não mover o `debounce` para o adaptador junto com o `notify`.** O intervalo de debounce é política de hot-reload
(ADR-0005) e pertence à aplicação. Movê-lo para dentro do adaptador `notify` troca uma violação de camada por outra.

---

## 3. Plano de implementação

| Fase    | Conteúdo                                                                                                                                       | Itens fechados                     | Esforço `[modelado]` |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- | -------------------- |
| **F0**  | Versões de action, `node 24`, remover `npx`, apagar `.gitkeep`                                                                                 | C-01…C-09                          | 0,25 d               |
| **F1**  | `thiserror` nos 5 enums de erro + **ADR-0011** + correção do `CLAUDE.md`                                                                       | C-23, G-01                         | 0,5–1 d              |
| **F2**  | `process::exit` → `Result` nas 7 ocorrências de `main.rs`                                                                                      | C-10, C-11                         | 0,5 d                |
| **F3**  | Encapsular os **30** campos `pub` de domínio + `Px`/`Color` + invariantes (`Size >= 0`, `selectors.len() > 0`)                                 | C-14, C-19, C-33                   | 1–1,5 d              |
| **F4**  | `Rect { origin: Point, size: Size }`, VO `Position`, ajuste dos 13 `Rect::new`                                                                 | C-30, C-31, C-34                   | 0,5–1 d              |
| **F5**  | Mover `parser.rs`, `layout.rs` e `serialize_to_html` para `domain/`; `TagName::default_display()`                                              | C-13, C-21, C-24, C-32             | 1–1,5 d              |
| **F6**  | Porta `FileWatchPort` + adaptador `NotifyFileWatcher`; porta `ColorResolver` + adaptador de cores                                              | C-15, C-16, C-29                   | 1–2 d                |
| **F7**  | **Ligar `register_fn` ao `rhai::Engine`** + teste que chama binding por `eval`, não por `call_function`                                        | D-01                               | 1–2 d                |
| **F8**  | `HostObject` universal no domínio de `engine`, `register_host_object` na porta (abole `register_fn` plano), tradução no backend + **ADR-0012** | N-01 (porta)                       | 1–1,5 d              |
| **F9**  | Reagrupar todas as funções sob namespaces (`document`/`Node`, `renderer`), camelCase, zero funções planas; `scripts/` de exemplo real          | N-01, C-28                         | 1,5–2 d              |
| **F10** | `Slot<T>`, `NodeId` geracional, `resolve` com `?` + **ADR-0013**                                                                               | C-26, C-27                         | 1,5–2 d              |
| **F11** | Enums W3C: `TagName`, `AttributeName`, `PropertyName`, `PropertyValue`, `HtmlEntity`, `is_void`                                                | C-17, C-18, C-22, C-25, C-35, C-37 | 3–4,5 d              |
| **F12** | `Selector` Selectors Level 3 completo + parser + especificidade                                                                                | C-20                               | 1,5–2 d              |
| **F13** | DOCTYPE implementado (quirks × standards mode)                                                                                                 | C-36                               | 0,3 d                |
| **F14** | Objetos de CSS/layout/conformidade expostos + migração da política para `rhai`                                                                 | G-02                               | 8–15 d               |

**Mínimo que destrava o merge** (F0–F10, sem enums W3C nem G-02): ≈ **9,75–13,25 d**. **Escopo da review + N-01, sem
G-02** (F0–F13): ≈ **14,5–22 d**. **Escopo completo:** F0–F14 ≈ **22,5–37 d**.

Ordem importa em cinco lugares, e três deles são novos:

1. **F7 antes de F8 e F9.** Agrupar uma superfície inalcançável é trabalho não verificável. F7 é o que torna F9 testável
   por `eval`, que é o único teste que prova alguma coisa aqui.
2. **F8 antes de F9.** O agrupamento nasce na porta; mexer nos bridges primeiro obriga a refazê-los quando a porta
   mudar.
3. **F9 antes de F14.** Migrar política para `rhai` sobre um namespace plano é trabalho que se refaz objeto a objeto
   depois.
4. F3 antes de F4, porque encapsular `Point`/`Size` primeiro evita reescrever `Rect` duas vezes.
5. F11 antes de F12 e F13, porque `Selector::Tag` carrega `TagName` e `is_void` (C-37) só existe depois de `TagName` ser
   enum.

F8 e F9 tocam `core/engine`, `core/runtime/rhai`, `core/dom` e `core/graphics` ao mesmo tempo — são a fase de maior
alcance do plano depois de F14, e a que mais se beneficia de vir numa PR própria.

**Recomendação:** três PRs. A #2 fecha com F0–F6 (a review "mecânica", ≈ 4,75–7,75 d). Uma segunda leva F7–F10 — o
redesenho da superfície de script e a arena — que é coeso e revisável. Uma terceira leva F11–F13. F14 é épico separado,
com escopo próprio. Empilhar tudo na PR #2, que já tem 125 arquivos, garante uma review superficial exatamente na parte
que mais precisa de atenção.

## 4. Armadilhas

| Armadilha                                                                                                                                                            | Mitigação                                                                                                                                           |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `TagName::new()` hoje é `Result` (`tag_name.rs:14`); virar enum tende a produzir `TryFrom` infalível para tags conhecidas e `Custom` para o resto                    | Manter a assinatura `Result` e deixar `Custom(String)` absorver o desconhecido — só string vazia continua erro                                      |
| `Px(pub f32)` e `Color(pub u32)` são construídos por padrão de tupla em `parser.rs:159` e testes                                                                     | `grep -rn "Px(\|Color("` antes de fechar o campo; trocar por `Px::new`/`Color::rgba`                                                                |
| 13 chamadas de `Rect::new(x, y, w, h)` quebram com `Rect { origin, size }`                                                                                           | Manter `Rect::new(x,y,w,h)` como construtor de conveniência que monta `Point`/`Size` internamente                                                   |
| `NodeId::index()` é usado para marshalar `EngineValue::Int` em `bridge.rs:43`                                                                                        | Preservar `index()` na API pública; a `generation` fica fora do valor de script                                                                     |
| `Size` com invariante `>= 0` rejeita `f32::NAN`, que passa em `>= 0.0` como `false`                                                                                  | Validar com `is_finite() && v >= 0.0`, não só `>= 0.0`                                                                                              |
| Mover `parser.rs` para `domain/` deixa `application/mod.rs` vazio em `css` e `graphics`                                                                              | Apagar o `mod.rs` e o `pub mod application` do `lib.rs`, não deixar módulo vazio                                                                    |
| `deny.toml` pode barrar `thiserror` por licença ou por duplicata de versão                                                                                           | Rodar `cargo deny check` local antes do push; `thiserror` é `MIT OR Apache-2.0`                                                                     |
| `notify` sai de `application` mas continua em `core/engine/Cargo.toml`                                                                                               | Só sai do manifesto se o adaptador for para outro crate; se ficar em `engine/infrastructure/`, a dep permanece e o `CLAUDE.md` precisa dizer isso   |
| `NodeId` como objeto `Node` de host deixa de ser `EngineValue::Int` e quebra qualquer script que faça aritmética com ele                                             | Não há script assim hoje (`scripts/hello.rhai` tem 3 linhas); fazer a troca agora, antes de existir base instalada                                  |
| `register_type_with_name` exige `T: Variant + Clone + 'static`; um `HostObject` que capture `Arc<Mutex<DomTree>>` precisa ser `Clone` barato                         | Guardar só o `Arc` clonado dentro do tipo de host — é o padrão que `ScriptDisplayListContainer` já usa (`rhai_bridge.rs:11-14`)                     |
| A feature `sync` do `rhai` (`Cargo.toml:29`) exige `Send + Sync` nos tipos registados; `NativeFn` já o exige (`ports.rs:9-13`), o tipo de host também passa a exigir | Verificar em F8, não em F9 — o erro aparece na porta, não no bridge                                                                                 |
| camelCase na superfície de script dispara `clippy::non_snake_case` se o nome virar identificador de Rust                                                             | Os nomes são `Identifier` (dado, não símbolo Rust); só o ADR precisa registar a escolha para não ser revertida                                      |
| Corrigir D-01 pode expor as 8 funções a scripts **sem** o guard de capability, se o registo no `rhai::Engine` contornar `guarded_native_fn`                          | F7 e F9 são inseparáveis na review: o teste de `PermissionDenied` tem de passar a rodar por `eval`, não só por `call_function`                      |
| As 9 ocorrências de `} else {` violam ADR-0010 e não estão na review                                                                                                 | Corrigir dentro das fases que já tocam esses arquivos (F2 pega `main.rs:77,112`; F6 pega `hot_reload.rs:168`)                                       |
| Tratar o agrupamento em namespaces como específico do DOM e deixar `graphics_*` ou funções futuras soltas no escopo global                                           | A porta `ExecutionContext` deve abolir o registro plano, forçando `HostObject` com namespace para todas as funções de todos os runtimes sem exceção |

---

## 5. Verificação

Nenhum item abaixo foi executado. Todos nascem desmarcados.

**Automatizável em CI (`cargo test --workspace`):**

- [ ] `TagName` aceita `<my-widget>` e devolve a variante aberta — o teste falha se o enum for fechado sem `Custom`, que
      é a regressão mais provável de F11.
- [ ] `AttributeName` aceita `data-id` e `aria-label` sem perder o sufixo.
- [ ] `Rule::new` rejeita `selectors` vazio com erro tipado (invariante que C-19 pede).
- [ ] `Size::new(-1.0, 0.0)` e `Size::new(f32::NAN, 0.0)` são rejeitados — os dois, não só o negativo.
- [ ] `NodeId` de um nó removido não resolve para o nó alocado depois dele no mesmo slot (ABA, C-27).
- [ ] `Selector::specificity` de `div > p:first-child` devolve `(0,1,2)` conforme Selectors L3.
- [ ] `DOCTYPE` ausente coloca o documento em quirks mode e `<!DOCTYPE html>` em standards mode — o teste falha se
      `process_token` voltar a devolver `Ok(())` sem efeito.
- [ ] Um `ColorResolver` de teste que não conhece `rebeccapurple` faz o parse falhar, provando que a resolução saiu do
      domínio e passou pela porta.
- [ ] `register_dom_bindings` sem `DOM_MUTATE` devolve `PermissionDenied` nas 5 funções — já coberto por
      `script_dom_mutation.rs`, precisa continuar verde depois de F9 — e passar a rodar também por `eval`.
- [ ] Um `FileWatchPort` falso dispara reload sem tocar o sistema de arquivos (prova que a porta de C-29 é real e não um
      wrapper fino de `notify`).
- [ ] `engine.eval(&mut ctx, r#"document.createElement("div")"#)` devolve `Ok` — o teste falha enquanto D-01 existir, e
      é o único que distingue "binding registada" de "binding alcançável". Hoje devolve `Err(FunctionNotFound(...))`.
- [ ] `document.createElement("div")` a partir de um contexto **sem** `DOM_MUTATE` devolve `PermissionDenied` — guarda
      contra a armadilha de F7 expor as funções por fora do `guarded_native_fn`.
- [ ] `dom_create_element("div")` e `graphics_push_rect(...)` — os nomes planos antigos — **falham** com
      `FunctionNotFound` depois de F9. O teste guarda a eliminação total de qualquer função plana solta no escopo
      global; sem ele, funções soltas e agrupadas poderiam coexistir em silêncio.
- [ ] `renderer.pushRect(...)` incrementa `renderer.commandCount`, provando que o objeto carrega estado entre chamadas e
      não é um wrapper sem identidade.
- [ ] A trait `ExecutionContext` não expõe método para registro de funções desprovidas de namespace contêiner, forçando
      conformidade em tempo de compilação para qualquer runtime (`rhai`, `mock`, etc.).
- [ ] Um `MockContext` (`engine/src/infrastructure/mock.rs`) regista um `HostObject` e resolve os seus métodos sem
      envolver `rhai` — prova que o agrupamento vive na porta e não no backend, que é o requisito do ADR-0002.

**Automatizável em CI (`pnpm check` + `cargo deny`):**

- [ ] `cargo clippy -D warnings` limpo após cada fase — o hook de pre-commit já o exige.
- [ ] `cargo deny check` aceita `thiserror` sem exceção manual em `deny.toml`.

**Só verificável no GitHub (não automatizável aqui):**

- [ ] Os 3 jobs do workflow passam com `actions/checkout@v7`, `setup-node@v7` e Node 24 — a matriz inclui `macos-latest`
      e `windows-latest`, que não foram exercitados neste ambiente.
- [ ] Os 37 threads da PR #2 aparecem como `isResolved: true`.

**Manual, sem cobertura possível hoje:**

- [ ] Editar `scripts/hello.rhai` com o watcher ativo dispara recompilação e swap atômico do AST.
- [ ] Um script de exemplo escrito só com a superfície agrupada monta uma árvore e desenha, sem chamar nenhuma função
      plana — é a prova de uso que `scripts/hello.rhai` hoje não dá.

---

## 6. Riscos

1. **A superfície de script não existe na prática, e três itens do plano dependem dela.** As 8 bindings não chegam ao
   `rhai::Engine` (D-01) e nenhuma toca CSS, layout ou conformidade HTML. N-01 e G-02 assentam sobre isso. Tratar G-02
   como "mover código" subestima o trabalho por uma ordem de grandeza e é o risco de cronograma dominante do conjunto.

2. **O defeito D-01 sobreviveu a 48 testes verdes, e a mesma cegueira pode repetir-se em F9.** Toda a suíte exercita as
   bindings por `call_function`, do lado de Rust. Enquanto o teste não passar por `eval`, qualquer redesenho da
   superfície — incluindo o agrupamento em objetos — é verificado contra um caminho que os utilizadores não usam.

3. **F11 e F12 podem quebrar o pipeline inteiro de uma vez.** `TagName` aparece em `dom`, `css`, `html`, `graphics` e
   `alloy`. Se o enum for introduzido sem variante aberta, o `TreeBuilder` passa a rejeitar documentos válidos e o
   subcomando `render` deixa de produzir PNG — falha visível só em teste de integração, não em unit test.

4. **A review é uma amostra e a próxima rodada será maior.** 4 de 30 campos `pub` foram apontados, 0 de 9 `else`, 0 de 5
   enums de erro fora de `dom`. Fechar apenas o literal dos comentários produz uma segunda `CHANGES_REQUESTED` sobre o
   mesmo material.

5. **Três ADRs novos entram no caminho crítico.** ADR-0011 (`thiserror`), ADR-0012 (objetos de host na superfície de
   script) e ADR-0013 (arena geracional) precisam ser escritos, revistos e indexados em `docs/adr/README.md` antes de
   F1, F8 e F10. Se a revisão do ADR-0012 demorar, F8 para — e F8 bloqueia F9, que por sua vez bloqueia F14.

6. **N-01 é uma mudança de porta universal para os runtimes e alcança todos os subsistemas.** O agrupamento por
   namespace não é cosmética exclusiva do DOM: tratar apenas `document` e deixar `graphics_*` ou futuras funções de
   CSS/Layout como funções globais soltas criaria uma assimetria fatal na arquitetura. `HostObject` nasce em
   `core/engine`, é traduzido em `core/runtime/rhai` e consumido em `core/dom`, `core/graphics` e em todas as políticas
   migradas para script em G-02. Errar a abstração da porta obriga a refazer todos os bridges. É o item do plano com
   maior custo de correção tardia.

7. **`CLAUDE.md` mente sobre `core/engine` e a mentira é operacional.** Enquanto a linha "Zero dependencies — pure
   abstraction" continuar lá, qualquer agente ou contribuidor novo vai tomar decisões de arquitectura com base num
   contrato falso.

---

## 7. Arquivos tocados

| Arquivo                                                                                                                                                  | Mudança                                                                                                               |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `.github/workflows/ci.yml`                                                                                                                               | 3× `checkout@v7`, `setup-node@v7`, `action-setup@v6`, `node-version: 24`                                              |
| `lefthook.yml`                                                                                                                                           | remover `npx` nas linhas 18 e 23                                                                                      |
| `spdd/prompt/.gitkeep`                                                                                                                                   | **apagado**                                                                                                           |
| `Cargo.toml`                                                                                                                                             | `thiserror` em `[workspace.dependencies]`                                                                             |
| `core/{dom,css,engine,graphics,html}/Cargo.toml`                                                                                                         | `thiserror = { workspace = true }`                                                                                    |
| `core/dom/src/domain/error.rs`                                                                                                                           | `#[derive(Error)]`, remover `impl fmt::Display`                                                                       |
| `core/css/src/domain/error.rs` · `core/engine/src/domain/error.rs` · `core/graphics/src/domain/error.rs` · `core/html/src/domain/token.rs`               | idem                                                                                                                  |
| `alloy/src/main.rs`                                                                                                                                      | `main() -> Result<…>`; 7 `process::exit` removidos; 2 `else` eliminados                                               |
| `core/css/src/domain/property.rs`                                                                                                                        | `Px`/`Color` sem campo `pub`; `parse`/`parse_hex` saem do domínio; `PropertyName` e `PropertyValue` viram enums       |
| `core/css/src/domain/rule.rs`                                                                                                                            | campos privados; construtor valida `selectors.len() > 0`                                                              |
| `core/css/src/domain/selector.rs`                                                                                                                        | Selectors L3 completo                                                                                                 |
| `core/css/src/domain/declaration.rs` · `computed.rs` · `specificity.rs`                                                                                  | 19 campos `pub` encapsulados                                                                                          |
| `core/css/src/application/parser.rs`                                                                                                                     | **movido** para `core/css/src/domain/parser.rs`                                                                       |
| `core/css/src/application/cascade.rs`                                                                                                                    | lista de tags inline → `TagName::default_display()`; `match` no lugar do early return                                 |
| `core/css/src/infrastructure/color_resolver.rs`                                                                                                          | **novo** — 148 cores CSS Color 4 atrás de porta                                                                       |
| `core/dom/src/domain/tree.rs`                                                                                                                            | `Slot<T>`, geração, `resolve` com `?`                                                                                 |
| `core/dom/src/domain/node_id.rs`                                                                                                                         | `NodeId { index, generation }`, `index()` preservado                                                                  |
| `core/dom/src/domain/tag_name.rs` · `attribute.rs`                                                                                                       | enums com variante aberta                                                                                             |
| `core/dom/src/application/service.rs`                                                                                                                    | `serialize_to_html` **movido** para `domain/`                                                                         |
| `core/dom/src/infrastructure/bridge.rs`                                                                                                                  | reescrito: objeto `document` + tipo `Node`, camelCase, capability no objeto                                           |
| `core/engine/src/application/ports.rs`                                                                                                                   | **novo** `FileWatchPort`; remoção do `register_fn` plano e introdução de `register_host_object` em `ExecutionContext` |
| `core/engine/src/domain/host_object.rs`                                                                                                                  | **novo** — `HostObject`: namespace/nome, `Capability`, métodos e propriedades                                         |
| `core/engine/src/infrastructure/mock.rs`                                                                                                                 | `MockContext` resolve `HostObject` sem `rhai` (prova de neutralidade da porta)                                        |
| `core/runtime/rhai/src/application/context.rs`                                                                                                           | **corrige D-01**: registo chega ao `rhai::Engine`; traduz `HostObject` em tipo customizado para todos os subsistemas  |
| `core/runtime/rhai/src/application/engine.rs`                                                                                                            | `eval` passa a enxergar as funções e objetos registados no contexto                                                   |
| `core/engine/src/application/hot_reload.rs`                                                                                                              | `notify` removido; debounce permanece; 1 `else` eliminado                                                             |
| `core/engine/src/infrastructure/notify_watcher.rs`                                                                                                       | **novo** adaptador                                                                                                    |
| `core/graphics/src/domain/geometry.rs`                                                                                                                   | campos privados; `Rect { origin, size }`; invariante `is_finite() && >= 0`                                            |
| `core/graphics/src/application/layout.rs`                                                                                                                | **movido** para `core/graphics/src/domain/layout.rs`                                                                  |
| `core/graphics/src/application/cpu_backend.rs`                                                                                                           | `put_pixel` recebe `Position`; `Rect` VO                                                                              |
| `core/graphics/src/infrastructure/rhai_bridge.rs`                                                                                                        | reescrito: objeto `renderer` (`pushRect`, `commandCount`, `toJSON`), eliminando prefixo plano `graphics_*`            |
| `core/html/src/domain/entities.rs`                                                                                                                       | enum `HtmlEntity` + `TryFrom`; 2 `else` eliminados                                                                    |
| `core/html/src/domain/tree_builder.rs`                                                                                                                   | DOCTYPE implementado; `is_void` no enum                                                                               |
| `docs/adr/0011-typed-errors-with-thiserror.md`                                                                                                           | **novo**                                                                                                              |
| `docs/adr/0012-host-objects-for-the-script-surface.md`                                                                                                   | **novo** — agrupamento universal por namespace para todas as funções dos runtimes, camelCase e neutralidade           |
| `docs/adr/0013-generational-arena-for-dom-nodes.md`                                                                                                      | **novo**                                                                                                              |
| `docs/adr/README.md`                                                                                                                                     | 3 linhas no índice                                                                                                    |
| `CLAUDE.md`                                                                                                                                              | corrigir "zero dependencies" de `core/engine`; registar as novas portas                                               |
| `core/dom/tests/dom_invariants.rs` · `core/css/tests/css_cascade.rs` · `core/html/tests/html_parsing.rs` · `core/graphics/tests/graphics_conformance.rs` | atualizar construções que usam campos hoje públicos                                                                   |
| `core/dom/tests/script_dom_mutation.rs` · `core/graphics/tests/graphics_conformance.rs`                                                                  | passar a exercitar por `eval`, não só por `call_function`                                                             |
| `scripts/hello.rhai`                                                                                                                                     | **substituído** por um exemplo que usa `document` e `renderer` de verdade                                             |

---

## 8. O que não foi verificado

**Nenhuma linha do código de produção foi escrita ou alterada.** Duas provas descartáveis foram executadas e removidas:
um teste em `core/dom/tests/` que confirmou `FunctionNotFound` ao chamar `dom_create_element` por `eval`, e um binário
fora do workspace que exercitou os três mecanismos de agrupamento do `rhai`. `git status` mostra apenas este relatório
como não rastreado.

**Os números de esforço são todos `[modelado]`.** Nenhuma fase foi cronometrada. As faixas vêm da contagem de arquivos e
call-sites afetados, não de medição. As faixas de F7, F8 e F9 são as menos ancoradas do plano: não há trabalho anterior
comparável no repositório para as calibrar.

**O CI não foi executado com as versões novas.** Verifiquei que `v7`/`v6`/`v7` são as releases correntes via
`gh api releases/latest`, mas não abri um workflow com elas. A matriz cobre `macos-latest` e `windows-latest`, e este
ambiente é Linux — `cargo test --workspace` foi rodado **só em Linux**, com 48 testes passando em 32 suítes.

**Não confirmei a contagem exata de nomes de cor do CSS Color 4.** Escrevi 148 de memória; o número correto precisa ser
lido da spec antes de implementar o `ColorResolver`.

**Não há defeito de correção nos 37 comentários.** Verifiquei cada um contra o arquivo e a linha citados e todos apontam
para código real com o problema descrito. A única imprecisão é textual, em C-22, que cita "W3C CSS3" para nomes de
atributo HTML — a spec correta é o HTML Living Standard. A hipótese de que algum comentário estivesse desatualizado por
commits posteriores está **refutada**: a review é de 21:43 UTC e o último commit é de 13:41 UTC.

**A hipótese de que o `rhai` não suportasse agrupamento em objetos está refutada** e fica registada para não voltar a
ser levantada. Os três mecanismos foram executados em `rhai` 1.26.0 e os três devolveram `Ok`: tipo customizado com
método (`document.createElement("div")` → `Ok(3)`) e propriedade (`document.nodeCount` → `Ok(1)`), módulo estático
(`dom::create_element("div")` → `Ok(3)`) e mapa de closures (`api.f("div")` → `Ok(3)`). A escolha entre eles é de
design, não de disponibilidade.

**Não desenhei a assinatura final de `HostObject`.** A forma proposta em 2.9 é uma direção, não uma API fechada. Três
pontos ficam por resolver e só se resolvem escrevendo o código: como um método de instância de `Node` recebe o `NodeId`
do receptor; como namespaces estáticos e métodos de instância coexistem sob a mesma porta neutra para cobrir tanto
objetos estáticos (`document`, `renderer`, `css`) quanto instâncias com estado (`Node`); se `HostObject` precisa de
propriedades mutáveis (`register_set`) ou só de getters; e se `Capability` no objeto substitui ou complementa o guard
por método. O ADR-0012 é o lugar de fechar isso, e é por isso que F8 vem antes de F9.

**Não medi o custo de marshaling da superfície completa de G-02.** Registar dezenas de objetos com tipos compostos pode
esbarrar em limites de `EngineValue` (`core/engine/src/domain/value.rs:8-16`), cujo `Object` é
`HashMap<String, EngineValue>` e não comporta funções. Isso é conhecido; o que não inspecionei é o custo de conversão em
`dynamic_to_engine_value` (`runtime/rhai/src/domain/marshaling.rs:30-71`), que faz `clone_cast` recursivo em arrays e
mapas. É premissa não verificada do custo de 8–15 d de F14.

**Não verifiquei se corrigir D-01 quebra algum teste existente.** Ligar `register_fn` ao `rhai::Engine` muda o
comportamento de `eval` para todo contexto que tenha bindings registadas. Os 48 testes passam hoje com o caminho
desligado; nenhum foi executado com ele ligado.

---

> Nenhum item deste relatório foi implementado. Toda a análise vem da leitura do código na branch
> `feat/v1-roadmap-implementation` (commit `35d35d6`), dos 37 threads não resolvidos da PR #2 e de duas provas
> descartáveis já removidas; a validação de cada fase está listada na seção 5 como pendente, e a execução do workflow
> com as versões novas de action não foi feita em nenhuma plataforma.
