# Implementação da v0.3 — plano detalhado de F4 + F5 + I2

| Campo               | Valor                                                                                                                                    |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| **Status**          | ❌ Não iniciado — plano. `core/graphics`, `core/html` e `core/css` ainda são o stub `add()`                                              |
| **Cobertura**       | Fecha 2 dos 18 critérios (**C-14**, **C-17**) + **C-18** antecipado do v0.9; abre os 3 ports do `ADR-0011` que faltavam ter código       |
| **Esforço**         | 63–99 dias-dev `[modelado]`. `ROADMAP-IMPLEMENTACAO-V1.md:217` orça 40–62 — a diferença é escopo escolhido, e está aberta em §1.3        |
| **Depende de**      | v0.1 + v0.2 inteiras (`feat/v0-2-implementation`). I2 exige F4 **e** F5 (`ROADMAP-IMPLEMENTACAO-V1.md:279`)                              |
| **Atenção**         | ⚠️ O v0.3 escreve **três** ports novos de uma vez (`RenderBackend`, `CascadeResolver`/`LayoutEngine`, `TokenSink`/`TreeSink`) — §2.13    |
| **Fecha requisito** | C-14, C-17, C-18 · `PRD-005` (retrofit ao `ADR-0011`) · `PRD-008` integral · `PRD-007` parcial (adaptadores UA, congelamento fica em I3) |

> ⚠️ **Base de referência das citações.** Toda referência `arquivo:linha` deste relatório foi conferida contra
> `feat/v0-2-implementation` (commit `6536bbc`, PR #5), **não** contra `main`. Em `main` os números de linha do
> `ROADMAP-IMPLEMENTACAO-V1.md` diferem (largura de reflow anterior), e `PRD-006`/`PRD-007`/`PRD-008`, `ADR-0011`,
> `ADR-0013`, `core/dom` e `core/engine` sequer existem. Ler este documento contra `main` produz citação pendurada; ele
> só é verificável sobre as PRs #4 e #5.

---

Este relatório cobre **apenas a v0.3** do `ROADMAP-IMPLEMENTACAO-V1.md` — as fases **F4** (`core/graphics`,
`ROADMAP-IMPLEMENTACAO-V1.md:261`), **F5** (`core/html`, `:262`) e o ponto de integração **I2** (`:279`), que o roadmap
§3.1 agrupa sob a versão "Primeiro pixel, headless" (`:217`). Nada aqui foi implementado.

Quatro decisões de escopo foram tomadas com o solicitante antes deste plano, e valem como premissa em todo o documento:

1. **O vão entre `DomTree` e geometria é atravessado pelas portas do `PRD-007`, não por layout descartável.** O v0.3
   cria em `core/css` os agregados de fronteira e as traits `CascadeResolver`/`LayoutEngine`, com adaptadores mínimos —
   cascata de folha UA em Rust (sem parser CSS) e layout de fluxo em bloco. F9 troca os miolos atrás das mesmas traits.
2. **O primeiro pixel inclui texto renderizado**, via porta `FontProvider` com `SystemFontProvider` (descoberta de
   fontes do sistema via filesystem pure-Rust + `ttf-parser`), fallback emergencial procedural/bitmap em container bare,
   e provedor sintético/mock determinístico para testes e goldens sem depender de assets binários
   (`ROADMAP-IMPLEMENTACAO-V1.md:315`).
3. **A suíte html5lib entra vendorizada**, num recorte declarado por manifesto em `core/html/tests/data/`.
4. **Entram três fatias além do mínimo**: C-18 antecipado (DisplayList scriptável sob `GRAPHICS_DRAW`), fuzzing de HTML
   já bloqueante, e `alloy` dividido em lib + binário fino.

---

## 1. Estado assumido e o que a v0.3 acrescenta

### 1.1 O que a v0.2 já entregou (verificado no branch)

`core/engine` com o port completo sob `ADR-0011` (sete itens verdes, `PORT_SCHEMA_VERSION = 2`), o companion object-safe
do `ADR-0013` e a suíte `engine::conformance`. `core/runtime/rhai` com `RhaiEngine`/`RhaiContext`, o chokepoint
`register_guarded_binding`, `run_with_fallback` e o `NodeHandle` de I1. `core/dom` como domínio puro de zero
dependências: arena `DomTree`, os value objects, `serialize_html`. O binário `alloy --script` roda `.rhai` sob sandbox
com DOM ligado.

O que **não** existe e o v0.3 cria do zero: qualquer noção de geometria, de pintura, de fonte, de pixel e de HTML.
`core/graphics`, `core/css` e `core/html` são os stubs `add()`/`it_works()` com `#![forbid(unsafe_code)]`.

### 1.2 O que a v0.3 acrescenta, critério a critério

| #        | Critério (`PRD`)                                                         | Fase | Como fecha                                                                                        |
| -------- | ------------------------------------------------------------------------ | ---- | ------------------------------------------------------------------------------------------------- |
| **C-14** | Trait `RenderBackend` definida em `core/graphics` (`PRD-005:87`)         | F4   | `application/ports.rs`, object-safe, com suíte de conformidade e `RecordingBackend` de referência |
| **C-17** | Fallback automático para `SoftwareCpuBackend` em headless (`PRD-005:90`) | F4   | `select_backend()` com a cascata de 3 tiers **inteira**; Vulkan/OpenGL devolvem `Unavailable`     |
| **C-18** | Serialização de display list e binding com Rhai testados (`PRD-005:91`)  | I2b  | `display_list_bindings.rs` em `rhai-runtime` sob `GRAPHICS_DRAW`; antecipado do v0.9              |

E o que o v0.3 fecha **sem** ter número de critério, porque os PRDs de port foram escritos depois do roadmap:

| Entrega                                                       | Origem            | Como fecha                                                                      |
| ------------------------------------------------------------- | ----------------- | ------------------------------------------------------------------------------- |
| `TokenSink`/`TreeSink` + tokenizer suspensível + html5lib     | `PRD-008:111-119` | F5 integral, incluindo o handshake suspend/resume e `no-default-tree`           |
| Agregados de fronteira + `CascadeResolver`/`LayoutEngine`     | `PRD-007:92-101`  | F4c parcial: portas e agregados definidos, adaptadores UA-only, freeze só em I3 |
| `PRD-005` retrofitado ao contrato de ports                    | `ADR-0011:126`    | Reescrita de `PRD-005` §2/§4/§5 + `render-backend-port-contract.md`             |
| Portões de golden image, conformidade html5lib e fuzz de HTML | roadmap `:357`    | Três jobs de CI novos, todos bloqueantes                                        |

**Micro-entregáveis da versão** (`ROADMAP-IMPLEMENTACAO-V1.md:235-236`): `alloy render pagina.html -o saida.png` produz
um PNG byte a byte determinístico; a golden image roda em CI sem GPU; o subconjunto declarado da suíte html5lib fica
verde.

### 1.3 Por que 63–99 d e não os 40–62 d do roadmap

O intervalo do roadmap (`:217`) soma F4 (15–22) + F5 (25–40) e não orça nem o texto, nem o estágio de estilo/layout —
que ele deixa implícito em F9/v0.5 — nem as três fatias extras. O delta é escopo **escolhido**, não estouro:

| Bloco                                                       | Esforço `[modelado]` | Estava no roadmap?                       |
| ----------------------------------------------------------- | -------------------- | ---------------------------------------- |
| F4a — display list, `RenderBackend`, rasterizador de caixas | 12–18 d              | Sim (parte dos 15–22 de F4)              |
| F4b — texto: `FontProvider`, fontes do SO, rasterizador     | 8–12 d               | Não — decisão de escopo 2                |
| F4c — `core/css`: agregados + portas + adaptadores UA       | 10–15 d              | Não — antecipa uma fatia de F9           |
| F5 — `core/html`: tokenizer suspensível + tree builder      | 25–40 d              | Sim, integral                            |
| I2 — pipeline, `alloy` lib+bin, comando `render`, goldens   | 4–7 d                | Sim (implícito no ponto de integração)   |
| I2b — C-18 antecipado                                       | 2–4 d                | Não — decisão de escopo 4                |
| Portões — fuzz, determinismo, conformidade, ADRs e PRDs     | 2–3 d                | Parcial (`:357` liga os portões no v0.3) |

Alavanca de alívio, se F5 estourar: adiar `adoption01/02.dat` (algoritmo de adoção) do recorte html5lib para o v0.5 —
§2.9. É a única fatia do recorte que pode sair sem tornar o parser inútil para páginas estáticas.

### 1.4 ⚠️ Divergências de documentação a corrigir nesta entrega

`docs/architecture/overview.md:85-89` declara `html → dom, engine`, `css → dom, engine` e `graphics → engine`. A decisão
2.1 abaixo mantém os três **sem** dependência de `engine` — todo bridge de script vive em `core/runtime/rhai`,
exatamente como a v0.2 fez com `core/dom`. As três linhas são alvo, não estado (`ROADMAP-IMPLEMENTACAO-V1.md:103-106`),
e passam a ser corrigidas para `dom` / `dom` / `ttf-parser`.

---

## 2. As decisões de design

### 2.1 Nenhum crate de domínio depende de `engine` — o bridge é sempre `core/runtime/rhai`

A v0.2 estabeleceu isso para `core/dom` (relatório v0.2 §2.1). O v0.3 **generaliza a regra**: `core/graphics`,
`core/css` e `core/html` também não nomeiam `engine`. Cada ponte com script é um adaptador em
`core/runtime/rhai/src/infrastructure/`, ao lado do `dom_bindings.rs` que já existe.

Consequências diretas:

- O portão "Domínio sem engine" (N-04, `PRD-001:99`) fica verde **por construção** para os três crates novos; o job
  `no-engine` da CI ganha três asserções de `cargo tree`.
- `PRD-007:65-67` (adaptador de cascata em `.rhai`) e a metade "policy" de `PRD-008` continuam possíveis: quem os
  implementa, no v0.5+, é `rhai-runtime`, que já depende de `dom` e passará a depender de `css`.
- **Custo aceito, registrado como risco §6.4**: `rhai-runtime` vira o depósito de todas as pontes. Quando passar de
  ~três bridges, quebra-se em `core/runtime/rhai-bindings`. Não no v0.3.

### 2.2 O pipeline do v0.3 e onde cada estágio mora

```text
arquivo .html
   │  UTF-8 decode (alloy)                    ← sniffing de charset é v0.5 (§2.14)
   ▼
core/html   Tokenizer ──Token──► TreeSink                    (ports do PRD-008)
   │                                └── DomTreeSink (infrastructure) ──► dom::DomTree
   ▼
core/css    snapshot(&DomTree) ──DomSnapshot──► CascadeResolver ──StyledTree──►
                                                LayoutEngine  ──LayoutBoxTree──►
   ▼
alloy       paint(&LayoutBoxTree) ──DisplayList──►            (mapeamento explícito, §2.2)
   ▼
core/graphics  Box<dyn RenderBackend> ──► SoftwareCpuBackend ──► Framebuffer(RGBA8)
   ▼
alloy       encode_png(&Framebuffer) ──► saida.png
```

Duas escolhas de fronteira que não são óbvias:

**O estágio de pintura (`LayoutBoxTree → DisplayList`) mora em `alloy/src/application/paint.rs`**, não em `css` nem em
`graphics`. Pôr em `css` criaria `css → graphics`; pôr em `graphics` criaria `graphics → css`. Nenhuma das duas arestas
existe no grafo-alvo de `overview.md:85-89`, e ambas acoplariam layout a pixels — exatamente o que `ADR-0010:114-117`
separa. O raiz de composição é o lugar certo para uma função de mapeamento entre dois agregados de crates distintos
(`ADR-0010:114-119`). Quando aparecer o segundo consumidor (F8, `core/window`), ela é promovida a crate próprio; é por
isso que `alloy` vira lib (§2.12), para o mapeamento ser testável hoje.

**`core/graphics` não conhece DOM nem CSS.** Ele recebe uma `DisplayList` e devolve pixels. É o que torna I6
(`ROADMAP-IMPLEMENTACAO-V1.md:283`) verificável no futuro: se Vulkan diverge do software, o defeito está no backend, não
no layout.

### 2.3 `core/graphics`: display list, builder sanitizador, erro tipado

`domain/`:

- `DisplayList` — **first-class collection** sobre `Vec<DisplayCommand>` (`ADR-0010:130`), imutável depois de
  construída, com `len()`/`iter()`/`command(index)`. Nada de `Vec` público.
- `DisplayCommand` — `#[non_exhaustive]`, os seis comandos de `PRD-005:65-71`: `DrawRect`, `DrawText`, `DrawImage`,
  `DrawPath`, `PushClip`/`PopClip`, `PushOpacity`/`PopOpacity`. `DrawImage` e `DrawPath` são **declarados e recusados**
  pelo backend do v0.3 (`GraphicsError::Unsupported`) — o contrato nasce inteiro, a implementação é incremental.
- Value objects: `Au(i32)` (§2.5), `Rect`, `Point`, `Size`, `Color(u32)`, `Opacity`, `FontId(u16)`, `ImageId(u32)`,
  `GlyphInstance { glyph_id, position }`.
- `GraphicsError` — **um** enum `#[non_exhaustive]`: `BackendUnavailable { tier }`, `SurfaceLost`,
  `InvalidCommand { index: CommandIndex, reason }`, `Unsupported { command }`, `ReadbackFailed`. O `CommandIndex(u32)` é
  o análogo de `SourceLocation` exigido pelo `ADR-0011:93-95` — num display list, a "linha" é o índice do comando.

`application/builder.rs` — `DisplayListBuilder` é o **único** caminho de construção, e sanitiza na fronteira
(`PRD-005:80`) com duas regras distintas, porque misturá-las é o erro comum:

| Entrada                                                | Regra                                               | Por quê                                                       |
| ------------------------------------------------------ | --------------------------------------------------- | ------------------------------------------------------------- |
| `NaN`, `±inf`, largura/altura negativas                | **Recusa**: `Err(InvalidCommand { index, reason })` | Não existe interpretação correta; engolir vira bug silencioso |
| Finito mas fora do envelope (`\|coord\| > MAX_EXTENT`) | **Clampa** para `MAX_EXTENT` e segue                | Página legítima tem caixa gigante; recusar quebraria a página |
| `Opacity` fora de `[0,1]`                              | Clampa                                              | Idem                                                          |
| `PopClip`/`PopOpacity` sem push correspondente         | Recusa                                              | Pilha desbalanceada corrompe todo o resto do frame            |

Como o builder recebe `f32` só na fronteira e converte para `Au` internamente, a checagem de finitude acontece
**exatamente uma vez**, no ponto de conversão. Teste de propriedade alimenta `NaN`, `±inf`, `f32::MAX`, subnormais e
zeros negativos.

### 2.4 `RenderBackend` é object-safe, e a cascata de três tiers nasce inteira

```rust
// core/graphics/src/application/ports.rs
pub trait RenderBackend: Send + Sync {
    fn tier(&self) -> BackendTier;
    fn begin_frame(&mut self, size: SurfaceSize) -> Result<(), GraphicsError>;
    fn submit(&mut self, list: &DisplayList) -> Result<(), GraphicsError>;
    fn end_frame(&mut self) -> Result<(), GraphicsError>;
    fn read_back(&self) -> Result<Framebuffer, GraphicsError>;
}
```

Todo método fala só tipos do próprio crate → `dyn RenderBackend` compila, e o item 2 do `ADR-0011:87-89` é satisfeito
**sem companion** — o contraste com `RuntimeEngine`, que precisou do `ADR-0013`, vale ser registrado no contract record.
`Box<dyn RenderBackend>` é o que a cascata devolve; `read_back` está na trait (e não só no backend de software) porque é
ele que torna I6 verificável: no F12, Vulkan lê de volta via staging buffer e é comparado contra a mesma golden image.

`infrastructure/cascade.rs` implementa `select_backend(preference) -> Box<dyn RenderBackend>` com os **três** degraus de
`PRD-005:33-58` escritos hoje. Vulkan e OpenGL são módulos que devolvem `Err(BackendUnavailable { tier })` até o F12 —
não `todo!()`, não ausentes. É isso que faz **C-17** ser fechado de verdade em vez de tautologicamente: a queda para
software é a queda real do algoritmo, exercitada por um teste que força cada tier a falhar
(`GRAPHICS_FORCE_TIER=vulkan|opengl|software`).

`infrastructure/software/` — o rasterizador: recorte por pilha de clip, preenchimento de retângulo com cobertura
anti-aliased em inteiros, composição `src-over` premultiplicada em `u8`. Sem `unsafe`, sem SIMD explícito no v0.3.

### 2.5 Determinismo: `Au(i32)` em 1/64 px, e o que isso custa (ADR-0014)

Golden image byte a byte nos três SOs só existe se a aritmética for idêntica nos três. As regras:

- **Geometria calculada é `Au(i32)`**, unidade de 1/64 px (convenção 26.6, a mesma das métricas de fonte). Soma,
  subtração e comparação de caixas são inteiras — sem acumulação de erro de ponto flutuante e sem depender de modo de
  arredondamento. `Px(f32)` (o newtype que `CLAUDE.md` cita) permanece o tipo de **entrada** — comprimento vindo do
  autor/CSS —, convertido para `Au` por **uma** função de arredondamento documentada.
- **Ponto flutuante só onde é inevitável**: contornos de glifo. IEEE-754 com `+ - * / sqrt` é determinístico entre
  plataformas; o que não é: `mul_add`/FMA (contração), transcendentais de libm, e ordem de redução variável. Portanto:
  nenhuma chamada a `sin`/`cos`/`exp`, nenhuma `f32::mul_add`, achatamento de Bézier com **contagem fixa** de
  subdivisões derivada de uma estimativa que usa só as quatro operações e `sqrt`.
- **Determinismo em testes via `FontProvider` sintético/mock** (`ROADMAP-IMPLEMENTACAO-V1.md:315`). A suíte de testes e
  goldens de CI usa métricas e glifos sintéticos determinísticos injetados via trait `FontProvider`, desacoplando os
  testes de fontes do SO e dispensando assets `.ttf` binários embarcados.
- **Golden compara o `Framebuffer`, não o PNG.** O arquivo `.png` de referência é decodificado e comparado pixel a
  pixel; assim o portão de determinismo não fica refém do codificador.

Isso vira **ADR-0014** (unidades fixas + política de determinismo de rasterização), que também registra a decisão do
codec do §2.7.

### 2.6 Texto: porta `FontProvider`, fontes do sistema, `ttf-parser`, rasterizador próprio

- Porta **`FontProvider`** em `application/ports.rs` desacopla a obtenção de faces e métricas. Em runtime,
  `SystemFontProvider` varre os diretórios padrão do SO (`/usr/share/fonts`, `~/.local/share/fonts`, `/Library/Fonts`,
  `C:\Windows\Fonts`) em pure-Rust sem FFI, construindo um `FontCatalog` em memória de forma lazy e carregando bytes do
  arquivo sob demanda.
- `infrastructure/font/` — `FontDatabase` indexa faces por `FontId(u16)` e resolve famílias genéricas (`sans-serif`,
  `serif`, `monospace`) mapeando para tabelas de fontes padrão do SO (ex.: DejaVu/Ubuntu no Linux, SF Pro/Helvetica no
  macOS, Segoe UI/Arial no Windows) e inspecionando tabelas OpenType via `ttf-parser`. Em ambientes bare/containers sem
  fontes do SO, há um gerador emergencial procedural/bitmap de glifos para evitar falhas em execução headless.
- **`ttf-parser`** entra como a única dependência externa nova do v0.3: pure Rust, sem `unsafe` (preserva N-02,
  `PRD-001:97`), faz parsing de tabelas e contornos de arquivos do sistema — nenhum rasterizador nativo C, nenhuma
  alocação escondida. Versão fixada exata no `[workspace.dependencies]`, como manda a convenção do `Cargo.toml`.
- **Shaping do v0.3 é deliberadamente ingênuo**: `cmap` char→glyph 1:1, avanços horizontais, kerning por `kern`/`GPOS`
  simples. **Fora**: ligaduras, BiDi, escritas complexas, quebra de linha por dicionário. Declarado em §2.14, e é o que
  `core/css` (F9) e `core/text` (v0.7+) endereçam depois.
- O rasterizador de glifo é nosso: contornos → achatamento fixo → varredura scanline com cobertura em inteiros → máscara
  alfa 8 bits → composição. Cache de glifo rasterizado por `(FontId, glyph_id, tamanho em Au)`; o cache **não** pode
  alterar o resultado — teste que roda com cache frio e quente e exige bytes idênticos.

### 2.7 PNG: codificador próprio, zero dependências

`infrastructure/png.rs` — ~120 linhas: assinatura, `IHDR`, `IDAT` com blocos deflate **armazenados** (`BTYPE=00`),
`IEND`, CRC-32 e Adler-32 escritos à mão. Sem `png`, sem `miniz_oxide`, sem `flate2`.

Razão: a alternativa puxa quatro crates para _escrever_ um arquivo que só serve de saída de CLI e de golden — e uma
delas (`simd-adler32`) traz `unsafe`, que exigiria exceção ao portão de memória (`PRD-001:97`). O arquivo fica maior;
como o golden compara framebuffer (§2.5), o tamanho não é portão de nada. **Decodificação** de imagens (v0.5,
`DrawImage` real) é decisão separada e provavelmente vai adotar um crate auditado — codificar é trivial, decodificar
formato hostil não é.

### 2.8 `core/css` no v0.3: as portas do `PRD-007` com adaptadores UA-only

`domain/` — os quatro agregados de fronteira de `PRD-007:35-40`, todos `#[non_exhaustive]`, com
`css::PORT_SCHEMA_VERSION = 1`:

| Agregado              | Conteúdo no v0.3                                                                                                   |
| --------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `DomSnapshot`         | Projeção imutável de `DomTree` (tag, atributos, forma da árvore). **Nenhum tipo de `core/dom` vaza na assinatura** |
| `StyleSheetSet`       | Regras ordenadas com origem (`UserAgent`/`User`/`Author`). No v0.3 só o vetor UA, construído em Rust               |
| `StyledTree`          | Valor computado por nó — no v0.3: `display`, `color`, `background_color`, `margin`, `padding`, `font_size`         |
| `LayoutBoxTree`       | Caixas com geometria resolvida em `Au`, prontas para pintura                                                       |
| `ViewportConstraints` | Largura/altura disponível + densidade                                                                              |

`application/ports.rs` — `CascadeResolver` e `LayoutEngine` **verbatim de `PRD-007:44-61`**: árvore inteira entra,
árvore inteira sai. A granularidade grossa é mandatória (`PRD-007:51`, `:78`) e é o que protege o orçamento de
`<10μs`/hook — não há callback por nó atravessando costura alguma.

`infrastructure/` — dois adaptadores de referência, que são também o **caminho padrão** (`PRD-007:71`, o contrato é
dogfooded):

- `ua_cascade.rs` — folha UA em Rust: `display: block` para `html/body/div/p/h1..h6/section/…`, `inline` para
  `span/a/em/strong`, margens de bloco default, tamanhos de heading. Herança de `color`/`font_size`. **Sem parser CSS,
  sem seletores, sem especificidade** — o `<style>` e o `style=` são ignorados no v0.3 (§2.14).
- `block_layout.rs` — fluxo normal em bloco: largura preenche o contentor menos margens/padding, altura é a soma dos
  filhos, texto vira caixas de linha por quebra em espaço com medição real de avanço da fonte. **Sem** float, sem
  posicionamento, sem flex (F9).

`application/snapshot.rs` — `snapshot(&DomTree) -> DomSnapshot`, a função de mapeamento explícita; é a única razão de
`css → dom`.

O teste de troca que o `ADR-0011:99-102` exige: um `MockCascadeResolver` que pinta tudo de vermelho troca por composição
e muda a golden — **sem** tocar `dom`, `graphics` ou `alloy`.

### 2.9 `core/html`: ports do `PRD-008`, tokenizer suspensível desde a primeira linha

`domain/` — `Token` `#[non_exhaustive]` verbatim de `PRD-008:35-45`, mais `TagToken`/`DoctypeToken`/`Attribute`/
`QualifiedName`, `RawKind`, e o `HtmlError` único com `SourceLocation { line, column, offset }`.

`application/ports.rs` — `TokenSink`, `TokenSinkResult` e `TreeSink` verbatim de `PRD-008:51-79`.

Duas leituras do PRD que o plano fixa:

1. **`TreeSink` é declarado em `core/html`, e o adaptador que o implementa sobre `DomTree` também.** `PRD-008:64` diz
   "implemented by `core/dom`" — seguir isso ao pé da letra faria `dom` conhecer `html` e quebraria a decisão 2.1 da
   v0.2 (domínio puro, zero dependências). O adaptador mora em `core/html/src/infrastructure/dom_sink.rs`, na direção já
   documentada `html → dom`. **Entregável: emenda em `PRD-008` §3.3.**
2. **`dom` é dependência opcional**, atrás da feature default `default-tree`. `--no-default-features` dá o
   `no-default-tree` de `PRD-008:118`: o tokenizer compila e passa a suíte contra um `MockTreeSink` que constrói uma
   estrutura diferente — o que também fecha `PRD-008:114-115`.

`infrastructure/tokenizer.rs` — máquina de estados **resumível na primeira implementação** (`PRD-008:98-99`,
`ROADMAP-IMPLEMENTACAO-V1.md:316`). `run(&mut self, input, sink) -> Run`, com `Run::Suspended { resume_at }` quando o
sink devolve `TokenSinkResult::Script(handle)`, e `resume(resume_at, extra_input)`. Não há JS no v0.3; o handshake é
exercitado por um sink de teste que injeta `"<p>x"` na retomada e exige o nó no resultado. **Escrever isso agora é a
diferença entre um contrato e um retrofit no F10** — e é a razão principal de F5 custar 25–40 d.

**Recorte html5lib vendorizado**, em `core/html/tests/data/` com `MANIFEST.md` que lista arquivo por arquivo:

| Dentro                                                                | Fora, e por quê                                      |
| --------------------------------------------------------------------- | ---------------------------------------------------- |
| `tokenizer/*.test` (integral, exceto casos que exigem flag de script) | `scripted/` — precisa de `core/js` (F10)             |
| `tree-construction`: estrutura básica, aninhamento implícito, tabelas | `template.dat` — `<template>` fora do v1.0 declarado |
| entidades nomeadas e numéricas, comentários, doctype                  | conteúdo estrangeiro (SVG/MathML) — fora do v1.0     |
| algoritmo de adoção (tags de formatação mal aninhadas) — ver §1.3     | `tests_innerHTML_*` — parsing de fragmento exige F10 |

O runner (`tests/html5lib.rs`) lê o manifesto, e **falha se um arquivo do diretório não estiver listado** — impede que
alguém adicione dado sem declarar o recorte, e impede o inverso, que é o recorte encolher em silêncio.

### 2.10 A exceção de Object Calisthenics no laço quente do tokenizer (ADR-0015)

`ROADMAP-IMPLEMENTACAO-V1.md:310` já prevê: newtype por caractere e "um nível de indentação" dentro de uma máquina de
estados de ~80 estados é catastrófico em desempenho e ilegível. A exceção é **delimitada e registrada**, não implícita:

- Vale só para `core/html/src/infrastructure/tokenizer.rs` (o laço e a tabela de estados). `domain/` e `application/` de
  `core/html` seguem as nove regras sem exceção, como `core/dom`.
- O que a exceção permite: `char`/`u8` crus dentro do laço; `match` de estado com dois níveis de indentação.
- O que a exceção **não** permite: primitivo cru cruzando fronteira pública, `Vec`/`HashMap` público, `else`, nome
  abreviado, `unwrap`/`panic!` em caminho alcançável por entrada.
- Um comentário no topo do arquivo cita o ADR — é a forma de a exceção ser revisável em vez de virar precedente.

### 2.11 C-18 antecipado: display list scriptável sob `GRAPHICS_DRAW`

`core/runtime/rhai/src/infrastructure/display_list_bindings.rs`, no mesmo molde de `dom_bindings.rs`: um
`DisplayListHandle` (`EngineType` + `rhai::CustomType`, nome de script `DisplayList`) sobre
`Arc<Mutex<DisplayListBuilder>>`, com `CapabilitySet` embutido; cada método se autoguarda em `GRAPHICS_DRAW` e mapeia
`GraphicsError` → `EngineError` (variante nova `Graphics { operation, reason }` → `PORT_SCHEMA_VERSION 2 → 3`, com nota
de migração em `PRD-002` §4.2, como manda `ADR-0011:104`). Manifesto `DISPLAY_LIST_BINDINGS` entra na varredura de C-06
que já existe, e na matriz de injeção de pânico de C-09 — é assim que o v0.3 prova que o sandbox vale para um
**segundo** subsistema, e não só para o DOM.

Serialização (a outra metade de `PRD-005:91`): `DisplayList` ganha uma forma textual determinística
(`display_list_to_text`) usada no teste — não é formato de fio, é o que torna a asserção legível e o diff útil.

### 2.12 `alloy` vira lib + binário fino; o comando `render`

`alloy/src/lib.rs` expõe
`render_document(source: &str, viewport: ViewportConstraints) -> Result<Framebuffer, AlloyError>` e os estágios em
`application/` (`pipeline.rs`, `paint.rs`). `main.rs` fica com parsing de argumentos (à mão, como já é) e I/O. Sem isso,
o teste de golden só consegue chamar o binário por fora — o que transforma toda falha de pipeline em "exit code != 0"
sem diagnóstico.

CLI: `alloy render <arquivo.html> -o <saida.png> [--width 800] [--height 600]`. `--script` continua como está.

### 2.13 Os três ports novos contra os sete itens do `ADR-0011:79-105`

| Item                             | `RenderBackend` (freeze **F4**)                         | `Cascade`/`Layout` (freeze I3)                               | `TokenSink`/`TreeSink` (freeze I3)                         |
| -------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------- |
| 1 Seam PRD + variação + ameaça   | `PRD-005` **retrofit** — §2/§4/§5 reescritos            | `PRD-007` (existe)                                           | `PRD-008` (existe) + emenda §3.3                           |
| 2 Traits sem tipo de adaptador   | Object-safe direto; sem companion (§2.4)                | `&self`, `Send + Sync`, árvore inteira                       | `TreeSink` genérico em `Handle`; sink stub                 |
| 3 Agregados versionados          | `DisplayList`, `Framebuffer`; `PORT_SCHEMA_VERSION=1`   | Os 5 de §2.8; `= 1`                                          | `Token` + value objects; `= 1`                             |
| 4 Um erro tipado com localização | `GraphicsError` + `CommandIndex`                        | `CssError` + `NodeRef` do snapshot                           | `HtmlError` + `SourceLocation`                             |
| 5 Ciclo de vida e concorrência   | `docs/architecture/render-backend-port-contract.md`     | `style-layout-port-contract.md`                              | `html-parser-port-contract.md` (suspensão!)                |
| 6 Conformidade + ref + `no-*`    | `run_backend_suite` · `RecordingBackend` · `no-backend` | `run_cascade_suite`/`run_layout_suite` · mocks · `no-script` | `run_tree_sink_suite` · `MockTreeSink` · `no-default-tree` |
| 7 Congelamento                   | **Congela no v0.3** (`ADR-0011:121`)                    | Só em I3 — mudança livre até lá                              | Só em I3 — mudança livre até lá                            |

Item 5 do port de HTML é o mais carregado: é onde a semântica de suspensão/retomada (`PRD-008:83-86`) fica escrita, e a
contract record diz explicitamente que este port **tem** ponto de suspensão, ao contrário do `RuntimeEngine`
(`runtime-engine-port-contract.md:67-68`).

### 2.14 O que NÃO fazer no v0.3

- **Não** escrever parser de CSS, seletor, especificidade, `<style>` ou `style=` — é F9/v0.5. O v0.3 tem folha UA em
  Rust e nada mais.
- **Não** implementar float, `position`, Flexbox, Grid, layout inline complexo, BiDi ou ligaduras.
- **Não** tocar `core/window`, `core/network`, `core/js`, `devtools`, `extension`. Não abrir janela: o v0.3 é headless
  por definição (`ROADMAP-IMPLEMENTACAO-V1.md:217`).
- **Não** escrever `VulkanBackend` nem `OpenGLBackend` — só os degraus da cascata devolvendo `BackendUnavailable`
  (§2.4). C-15/C-16 são F12.
- **Não** implementar sniffing de charset nem decodificação além de UTF-8 — `PRD-008:47-48` põe isso explicitamente fora
  do port.
- **Não** medir `<10μs` com `criterion`: o portão entra no v0.5 (`ROADMAP-IMPLEMENTACAO-V1.md:358`). Só não introduzir
  callback por nó na costura de layout (§2.8).
- **Não** adicionar índice geracional ao `NodeId` nem mexer em `core/dom` — o tree builder usa a API que já existe.

---

## 3. Plano de implementação

| Fase    | Conteúdo                                                                             | Entregável verificável                                       | Esforço `[modelado]` |
| ------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------------ | -------------------- |
| **F4a** | `core/graphics`: display list, builder sanitizador, `RenderBackend`, cascata, raster | **C-14**, **C-17**; golden de caixas em CI sem GPU           | 12–18 d              |
| **F4b** | Texto: port `FontProvider`, `SystemFontProvider`, `ttf-parser`, rasterizador, cache  | Golden com texto determinístico via mock/sintético nos 3 SOs | 8–12 d               |
| **F4c** | `core/css`: agregados, portas do `PRD-007`, `ua_cascade`, `block_layout`             | `DomTree → PNG` sem HTML; troca de `MockCascadeResolver`     | 10–15 d              |
| **F5**  | `core/html`: `Token`, ports, tokenizer suspensível, tree builder, html5lib           | Recorte html5lib verde; handshake suspend/resume testado     | 25–40 d              |
| **I2**  | `alloy` lib+bin, `pipeline.rs`, `paint.rs`, comando `render`, goldens de página      | `alloy render pagina.html -o saida.png` determinístico       | 4–7 d                |
| **I2b** | C-18: `display_list_bindings.rs`, `EngineError::Graphics`, serialização textual      | **C-18**; varredura C-06 e injeção de pânico cobrem o novo   | 2–4 d                |
| **P**   | Portões: fuzz de HTML, determinismo, conformidade, ADR-0014/0015, PRDs, contracts    | Três jobs de CI novos, bloqueantes                           | 2–3 d                |

**Ordem, e por que ela é assim.** F4a → F4b → F4c → **primeiro pixel sem HTML** → F5 → I2 → I2b → P. O ponto não óbvio:
**o primeiro pixel chega antes do parser**. Ao fim de F4c existe um teste que monta um `DomTree` em Rust (ou pelo
`hello_dom.rhai` que a v0.2 já roda), passa pelo pipeline inteiro e produz PNG. Isso adianta em semanas a descoberta de
todo defeito de fronteira entre `dom`, `css`, `graphics` e a pintura — enquanto F5, a fase de maior variância do v0.3
(risco §6.1), ainda está sendo escrita. Inverter a ordem significa integrar tudo no fim, que é quando integração custa
caro.

O SPDD roda antes de cada fase, não depois (`PRD-001:100`, `ADR-0007`): `/spdd-analysis` + `/spdd-reasons-canvas` para
F4 (a+b+c como um canvas de graphics e um de css), F5 e I2.

**F4a — passos (12–18 d), em `core/graphics/`:**

1. **Value objects + `GraphicsError` (2–3 d)** — `Au`, `Rect`, `Point`, `Size`, `Color`, `Opacity`, `FontId`,
   `CommandIndex`; conversão `Px → Au` única e documentada; `#![forbid(unsafe_code)]` mantido.
2. **`DisplayList` + `DisplayCommand` + builder (3–4 d)** — first-class collection; as duas regras de sanitização de
   §2.3; teste de propriedade com `NaN`/`inf`/extremos; pilha de clip/opacidade balanceada.
3. **`RenderBackend` + conformidade (2–3 d)** — trait object-safe; `run_backend_suite(&mut dyn RenderBackend)`;
   `RecordingBackend` de referência; feature `no-backend`.
4. **`SoftwareCpuBackend` (3–5 d)** — `Framebuffer(RGBA8)`, preenchimento de retângulo com AA em inteiros, clip,
   opacidade, composição premultiplicada.
5. **Cascata de 3 tiers (1–2 d)** — `select_backend`, degraus Vulkan/OpenGL devolvendo `BackendUnavailable`, override
   por variável de ambiente para testar cada queda. Fecha **C-17**.
6. **PNG + infra de golden (1–2 d)** — `png.rs` (§2.7); helper `assert_golden(framebuffer, path)` que compara pixels e,
   ao falhar, escreve `<nome>.actual.png` e um mapa de diferença.

**F4b — passos (8–12 d):** port `FontProvider`, `SystemFontProvider` com varredura lazy de diretórios do SO,
`FontCatalog`, fallback emergencial em container e provedor sintético para testes (2–3 d); `ttf-parser` no
`[workspace.dependencies]` e extração de `cmap`/métricas/contornos (2 d); achatamento de Bézier com contagem fixa e
varredura scanline com cobertura inteira (3–4 d); cache de glifo + teste frio-vs-quente (1–2 d); `DrawText` no backend
de software e a primeira golden com letra sintética/determinística (1 d).

**F4c — passos (10–15 d), em `core/css/`:** os cinco agregados + `CssError` + `PORT_SCHEMA_VERSION` (2–3 d);
`snapshot(&DomTree)` e a decisão de `dom` como única dependência (1–2 d); `CascadeResolver`/`LayoutEngine` +
`run_cascade_suite`/`run_layout_suite` + mocks + feature `no-script` (2–3 d); `ua_cascade` (2–3 d); `block_layout` com
medição de texto pela fonte (3–4 d).

**F5 — passos (25–40 d), em `core/html/`:** `Token` e value objects (2–3 d); `TokenSink`/`TreeSink`/`TokenSinkResult` +
`MockTreeSink` + feature `no-default-tree` (2–3 d); **máquina de estados resumível** — estados de dados, tag, atributo,
comentário, doctype, RAWTEXT/RCDATA, referência de caractere (10–16 d); `dom_sink.rs` construindo `DomTree` com pilha de
elementos abertos, fechamento implícito, foster parenting e algoritmo de adoção (8–12 d); vendoring do recorte +
`MANIFEST.md` + runner que falha em arquivo não listado (2–3 d); teste do handshake suspend/resume com injeção de
entrada (1–2 d).

**I2 — passos (4–7 d):** `alloy` lib+bin e `AlloyError` (1 d); `pipeline.rs` amarrando decode → tokenize → snapshot →
cascade → layout (1–2 d); `paint.rs` (`LayoutBoxTree → DisplayList`) (1–2 d); comando `render` + PNG (0,5–1 d); páginas
de golden (uma de caixas, uma de texto, uma de aninhamento + entidades) e o job de CI (1 d).

**I2b — passos (2–4 d):** `EngineError::Graphics` + bump de schema + nota de migração em `PRD-002` (0,5 d);
`display_list_bindings.rs` + manifesto + entrada na varredura C-06 e na matriz de pânico (1,5–2,5 d);
`display_list_to_text` + `scripts/paint.rhai` de exemplo (0,5–1 d).

**P — passos (2–3 d):** alvos `fuzz/fuzz_targets/{tokenize,tree_build}.rs` com corpus semeado pelo html5lib e job de CI
em nightly, 10 min por alvo, bloqueante (1–1,5 d); job de determinismo (render 100× no Linux, comparação cruzada de
framebuffer nos 3 SOs) (0,5 d); ADR-0014, ADR-0015, linhas no `docs/adr/README.md`, retrofit de `PRD-005`, emendas em
`PRD-007`/`PRD-008`, três contract records, `overview.md`, `CLAUDE.md` (1 d).

**Mínimo viável** (só C-14 + C-17, sem texto, sem HTML, sem CSS): F4a ≈ **12–18 d**. **Escopo completo:** ≈ **63–99
dias-dev `[modelado]`**.

---

## 4. Armadilhas

| Armadilha                                                                              | Mitigação                                                                                                                                |
| -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Golden image diverge entre Linux, macOS e Windows por ponto flutuante                  | `Au(i32)` em toda geometria; float só em contorno de glifo, sem FMA e sem transcendental; achatamento com contagem fixa (§2.5, ADR-0014) |
| Variação de fontes do SO quebra os testes de golden em CI                              | Testes e goldens utilizam estritamente o `FontProvider` sintético/mock; runtime usa `SystemFontProvider`                                 |
| Cache de glifo altera o resultado entre execuções                                      | Teste que roda a mesma página com cache frio e quente e exige framebuffer idêntico                                                       |
| `NaN` chegando ao backend (`PRD-005:80`)                                               | Sanitização no `DisplayListBuilder`, com as duas regras distintas de §2.3; teste de propriedade                                          |
| Sanitizar clampando tudo esconde bug de layout                                         | Não-finito **recusa** (erro tipado), só o finito-fora-de-envelope clampa                                                                 |
| `css → graphics` ou `graphics → css` entrando "só para o paint"                        | Pintura no raiz de composição (`alloy/application/paint.rs`, §2.2); promovida a crate quando houver segundo consumidor                   |
| Domínio novo dependendo de `engine` porque `overview.md:85-89` diz que sim             | Decisão 2.1: bridges em `rhai-runtime`; corrigir as três linhas do `overview.md` **nesta** entrega                                       |
| `TreeSink` implementado dentro de `core/dom`, como `PRD-008:64` sugere                 | Adaptador em `core/html/infrastructure/dom_sink.rs`; emenda em `PRD-008` §3.3 (§2.9)                                                     |
| Tokenizer escrito não-resumível "para simplificar", com suspensão prometida para o F10 | `Run::Suspended`/`resume` na primeira implementação, com teste de injeção de entrada — `PRD-008:98-99`, roadmap `:316`                   |
| Object Calisthenics regra 3/4 no laço quente do tokenizer                              | Exceção delimitada e registrada em **ADR-0015**, válida só para `infrastructure/tokenizer.rs` (§2.10)                                    |
| Recorte html5lib encolhe em silêncio para o CI ficar verde                             | `MANIFEST.md` + runner que falha se houver arquivo não listado **ou** listado e ausente                                                  |
| `cargo-fuzz` exige nightly e a toolchain está pinada em 1.97.1                         | Job próprio com `+nightly` e `rustup toolchain install nightly`; o resto da CI não muda                                                  |
| `MAX_EXTENT` do clamp escolhido pequeno demais corta página legítima                   | Envelope derivado do viewport × fator documentado, com teste de página alta (10.000 px) que **não** é cortada                            |
| `EngineError::Graphics` (I2b) muda superfície congelada do port de engine              | Aditivo, mas cumpre a formalidade do `ADR-0011:104`: `PORT_SCHEMA_VERSION 2 → 3` + nota de migração em `PRD-002` §4.2 + contract record  |
| Ambiente bare container sem nenhuma fonte instalada falha renderização headless        | `SystemFontProvider` possui fallback emergencial procedural/bitmap de glifos embutido                                                    |
| Repo ainda sem `LICENSE` (nota no `Cargo.toml`) e agora com novas dependências         | Levantar com os mantenedores nesta entrega; não é bloqueante para o código, é para a distribuição                                        |
| `rhai-runtime` acumulando toda ponte de script                                         | Aceito no v0.3 (dois bridges); quebra em `core/runtime/rhai-bindings` quando chegar ao terceiro (risco §6.4)                             |
| `spdd/` sem canvas para F4/F5/I2 enquanto `PRD-001:100` os exige                       | `/spdd-analysis` + `/spdd-reasons-canvas` antes do primeiro `/spdd-generate` de cada fase, como foi feito em F3 e F6                     |

---

## 5. Verificação

Nada aqui foi executado. Nenhum item nasce marcado.

**Automatizável em CI, nos 3 SOs (`pnpm check` + `cargo test --workspace`):**

- [ ] `cargo test -p graphics -p css -p html` verde; `fmt --check` e `clippy -D warnings` continuam exit 0.
- [ ] `cargo tree -p graphics` mostra só `ttf-parser`; `cargo tree -p css` e `-p html` não mostram `engine` nem `rhai` —
      portão "Domínio sem engine" (N-04) estendido aos três crates novos.
- [ ] `cargo test -p graphics --no-default-features` (feature `no-backend`) — display list e port compilam sem backend
      concreto linkado (`ADR-0011:99-102`).
- [ ] `cargo test -p html --no-default-features` (`no-default-tree`) — tokenizer passa contra `MockTreeSink`, sem `dom`
      no grafo (`PRD-008:118`).
- [ ] `cargo test -p css --no-default-features` (`no-script`) — portas e agregados com adaptadores Rust apenas
      (`PRD-007:99`).
- [ ] `run_backend_suite` passa para `SoftwareCpuBackend` **e** `RecordingBackend` — guarda **C-14**.
- [ ] Forçando falha de Vulkan e de OpenGL, `select_backend` devolve `SoftwareCpuBackend` e a página ainda renderiza —
      guarda **C-17**; forçar só Vulkan a falhar devolve o degrau seguinte, não o último.
- [ ] `DisplayListBuilder` recusa `NaN`/`±inf`/dimensão negativa com `InvalidCommand { index }` e clampa finito fora do
      envelope; teste de propriedade não encontra entrada que chegue ao backend não sanitizada (`PRD-005:80`).
- [ ] `PopClip`/`PopOpacity` desbalanceados são recusados na construção.
- [ ] Golden image de página só com caixas bate pixel a pixel nos três SOs.
- [ ] Golden image com texto bate pixel a pixel nos três SOs, com cache de glifo frio e quente.
- [ ] 100 renderizações da mesma entrada produzem framebuffer idêntico (determinismo, `PRD-007:100`).
- [ ] `MockCascadeResolver` trocado por composição muda a golden **sem** alterar `dom`, `graphics` ou `alloy`
      (`PRD-007:95`).
- [ ] O recorte declarado da suíte html5lib fica **100%** verde; o runner falha se o manifesto e o diretório divergirem.
- [ ] `MockTreeSink` constrói uma estrutura diferente a partir do mesmo fluxo de tokens, sem mudar `core/html`
      (`PRD-008:114-115`).
- [ ] Sink que devolve `TokenSinkResult::Script` suspende o tokenizer; a retomada com entrada extra `"<p>x"` produz a
      árvore com o nó injetado (`PRD-008:116-117`).
- [ ] `cargo-fuzz` no tokenizer e no tree builder: zero pânicos em 10 min por alvo, job bloqueante (roadmap `:357`,
      `PRD-008:119`).
- [ ] Script `.rhai` com `GRAPHICS_DRAW` monta uma `DisplayList`, o host a serializa e o texto bate com o esperado;
      script **sem** `GRAPHICS_DRAW` recebe `EngineError::PermissionDenied` — guarda **C-18** e estende C-06/C-07.
- [ ] A matriz de injeção de pânico cobre também `DISPLAY_LIST_BINDINGS` (C-09 no segundo subsistema).
- [ ] Cobertura de `domain/` ≥ 85% para `graphics`, `css` e `html`.

**Só local / manual:**

- [ ] `alloy render docs/exemplos/pagina.html -o /tmp/saida.png` abre num visualizador e mostra a página; `--width` muda
      o layout coerentemente.
- [ ] `spdd/analysis/` e `spdd/prompt/` populados para F4, F5 e I2 antes do primeiro `/spdd-generate`.

**Não verificável nesta fase (declarado):**

- [ ] `VulkanBackend`/`OpenGLBackend` desenhando (**C-15**/**C-16**) — F12; o v0.3 só prova a cascata.
- [ ] Overhead `<10μs` por hook — portão entra no v0.5 (`ROADMAP-IMPLEMENTACAO-V1.md:358`).
- [ ] Fidelidade de `<script>` real (`document.write`) — o handshake é testado com sink sintético; JS é F10.
- [ ] Qualquer coisa de CSS de autor: seletor, especificidade, `<style>`, `style=` — F9.

---

## 6. Riscos

1. **F5 é a fase de maior variância do v0.3, e o roadmap já avisa** (`ROADMAP-IMPLEMENTACAO-V1.md:409-410`). Tokenização
   e construção de árvore não admitem "quase certo": foster parenting, fechamento implícito e o algoritmo de adoção são
   cada um uma fonte de casos. Se o intervalo 25–40 d for estourar, a alavanca declarada é adiar `adoption01/02.dat`
   para o v0.5 (§1.3) — decisão consciente, registrada no `MANIFEST.md`, não descoberta na véspera.

2. **Determinismo cross-OS é o portão mais frágil do v0.3.** A primeira golden com texto que divergir entre Linux e
   macOS custa dias de bisecção. Mitigação de processo: rodar o job de determinismo na matriz **desde a primeira**
   golden de caixas (F4a passo 6), não só depois do texto — assim a divergência, se vier, chega com uma superfície
   pequena para investigar.

3. **Antecipar `core/css` pode congelar cedo demais o que F9 vai querer diferente.** `StyledTree` com cinco propriedades
   computadas é confortável hoje e apertado quando entrar herança de verdade, unidades relativas e Flexbox. Mitigação:
   os agregados são `#[non_exhaustive]` e o freeze é em **I3**, não agora (`ADR-0011:123`) — F9 tem liberdade de mudar
   com bump de schema. O risco real é psicológico: tratar o adaptador UA como "o cascade" e nunca o substituir.

4. **`rhai-runtime` vira o depósito de bridges.** Com `dom_bindings` + `display_list_bindings` já são dois; F9 traz o
   adaptador de cascata scriptável e F11 o hot-reload. A quebra em `core/runtime/rhai-bindings` precisa ser decidida
   antes do terceiro, não depois do quinto.

5. **A dependência nova (`ttf-parser`) abre frente de licenciamento auditável por `cargo-deny`**, eliminando a
   necessidade de versionar assets binários `.ttf` de terceiros no repositório graças ao uso de fontes do sistema via
   `SystemFontProvider` e provedores sintéticos em testes.

6. **O v0.3 escreve três ports de uma vez.** É o maior lote de superfície pública do projeto até aqui, e o `ADR-0011`
   exige sete itens para cada um, incluindo suíte de conformidade, adaptador de referência, feature `no-*` e contract
   record. Subestimar a papelada é o jeito mais provável de o "P" de 2–3 d virar uma semana — ou, pior, de os contract
   records nunca serem escritos e o contrato de ports perder autoridade, que é o risco §6 do próprio roadmap.

---

## 7. Arquivos tocados

| Arquivo                                                                                                              | Mudança                                                                                                                                |
| -------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `Cargo.toml`                                                                                                         | `[workspace.dependencies]` += `ttf-parser` com versão exata                                                                            |
| `core/graphics/Cargo.toml`                                                                                           | + `ttf-parser`; features `software-backend` (default) e `no-backend`                                                                   |
| `core/graphics/src/domain/`                                                                                          | **novo** — `Au`, `Rect`/`Point`/`Size`, `Color`, `Opacity`, `FontId`, `DisplayList`, `DisplayCommand`, `GraphicsError`, `CommandIndex` |
| `core/graphics/src/application/` (`ports.rs`, `builder.rs`, `conformance.rs`)                                        | **novo** — `RenderBackend`, `FontProvider`, `DisplayListBuilder` sanitizador, `run_backend_suite`                                      |
| `core/graphics/src/infrastructure/` (`cascade.rs`, `software/`, `font/`, `png.rs`)                                   | **novo** — cascata de 3 tiers, rasterizador, `SystemFontProvider`, `FontCatalog`, glifos, codificador PNG                              |
| `core/graphics/tests/` (+ `tests/golden/`)                                                                           | **novo** — conformidade, sanitização, cascata, goldens de referência com `FontProvider` sintético                                      |
| `core/css/Cargo.toml`                                                                                                | + `dom = { path = "../dom" }`; features `builtin-adapters` (default) e `no-script`                                                     |
| `core/css/src/domain/`                                                                                               | **novo** — `DomSnapshot`, `StyleSheetSet`, `StyledTree`, `LayoutBoxTree`, `ViewportConstraints`, `CssError`                            |
| `core/css/src/application/` (`ports.rs`, `snapshot.rs`, `conformance.rs`)                                            | **novo** — `CascadeResolver`/`LayoutEngine`, `snapshot(&DomTree)`, suítes                                                              |
| `core/css/src/infrastructure/` (`ua_cascade.rs`, `block_layout.rs`)                                                  | **novo** — adaptadores de referência, que são o caminho padrão                                                                         |
| `core/css/tests/`                                                                                                    | **novo** — asserção de retângulo, determinismo, troca por mock                                                                         |
| `core/html/Cargo.toml`                                                                                               | + `dom` **opcional**; features `default-tree` (default) e `no-default-tree`                                                            |
| `core/html/src/domain/`                                                                                              | **novo** — `Token`, `TagToken`, `Attribute`, `QualifiedName`, `RawKind`, `HtmlError`, `SourceLocation`                                 |
| `core/html/src/application/` (`ports.rs`, `conformance.rs`)                                                          | **novo** — `TokenSink`, `TokenSinkResult`, `TreeSink`, `run_tree_sink_suite`                                                           |
| `core/html/src/infrastructure/` (`tokenizer.rs`, `tree_builder.rs`, `dom_sink.rs`)                                   | **novo** — máquina de estados resumível, construção de árvore, adaptador sobre `DomTree`                                               |
| `core/html/tests/` + `tests/data/MANIFEST.md`                                                                        | **novo** — runner html5lib, recorte vendorizado, `MockTreeSink`, teste de suspend/resume                                               |
| `core/engine/src/domain/error.rs`                                                                                    | + variante `Graphics { operation, reason }`; `PORT_SCHEMA_VERSION` 2 → 3                                                               |
| `core/runtime/rhai/Cargo.toml` + `src/infrastructure/display_list_bindings.rs`                                       | **novo** — + `graphics`; `DisplayListHandle` sob `GRAPHICS_DRAW`, manifesto `DISPLAY_LIST_BINDINGS`                                    |
| `alloy/src/lib.rs`, `alloy/src/application/` (`pipeline.rs`, `paint.rs`), `alloy/src/main.rs`                        | **novo/refatorado** — lib + binário fino, comando `render`, mapeamento `LayoutBoxTree → DisplayList`                                   |
| `alloy/tests/`                                                                                                       | **novo** — golden de página fim a fim, determinismo                                                                                    |
| `scripts/paint.rhai`                                                                                                 | **novo** — exemplo de C-18                                                                                                             |
| `fuzz/`                                                                                                              | **novo** — alvos `tokenize` e `tree_build` + corpus semeado                                                                            |
| `.github/workflows/ci.yml`                                                                                           | **novo** — jobs `golden`, `html-conformance`, `fuzz` (todos bloqueantes); `no-engine` estendido aos 3 crates                           |
| `deny.toml`                                                                                                          | Licença de `ttf-parser`                                                                                                                |
| `docs/adr/0014-…` (determinismo e unidades), `docs/adr/0015-…` (exceção no tokenizer), `docs/adr/README.md`          | **novo** — dois MADRs + linhas no índice                                                                                               |
| `docs/requirements/PRD-005-…`                                                                                        | **Retrofit ao `ADR-0011`**: variação, ameaça, ciclo de vida, conformidade, `no-backend`, freeze em F4                                  |
| `docs/requirements/PRD-007-…`, `PRD-008-…`                                                                           | Emendas: adaptadores UA-only no v0.3; `TreeSink` implementado em `core/html`, não em `core/dom`                                        |
| `docs/requirements/PRD-002-…`                                                                                        | Nota de migração do `PORT_SCHEMA_VERSION` 2 → 3 (`EngineError::Graphics`)                                                              |
| `docs/architecture/render-backend-port-contract.md`, `style-layout-port-contract.md`, `html-parser-port-contract.md` | **novo** — os três contract records dos sete itens                                                                                     |
| `docs/architecture/overview.md:85-89`, `CLAUDE.md`                                                                   | `html`/`css`/`graphics` sem `engine`; "Current State" reescrito                                                                        |
| `spdd/analysis/`, `spdd/prompt/`                                                                                     | **novo** — canvases de F4, F5 e I2 (`PRD-001:100`)                                                                                     |
| `docs/reports/IMPLEMENTACAO-DETALHADA-V0-3.md`, `docs/README.md`                                                     | **novo** — este relatório + linha na árvore de `reports/`                                                                              |

---

> Nenhuma linha deste plano foi implementada. O que **foi** feito nesta rodada: leitura de
> `ROADMAP-IMPLEMENTACAO-V1.md`, `IMPLEMENTACAO-DETALHADA-V0-2.md`, `PRD-005`, `PRD-007`, `PRD-008`, `ADR-0011`,
> `ADR-0013`, `docs/architecture/overview.md`, `runtime-engine-port-contract.md`, e inspeção do estado real do workspace
> no branch `feat/v0-2-implementation` (`core/graphics`, `core/css` e `core/html` ainda são o stub `add()`; a API
> pública de `DomTree` foi lida de `core/dom/src/domain/tree.rs`). Todas as referências `arquivo:linha` foram conferidas
> contra esses arquivos. **Não verificado**: a disponibilidade e a versão exata de `ttf-parser` a fixar (checar no
> momento da implementação) e o conteúdo exato dos arquivos do upstream html5lib-tests, cujo recorte da §2.9 é declarado
> por capacidade e vira lista nominal no `MANIFEST.md` no ato do vendoring. Os esforços em dias-dev são `[modelado]`; os
> blocos que existem no roadmap reaproveitam `ROADMAP-IMPLEMENTACAO-V1.md:261-262`, e os demais não têm velocidade
> histórica para calibrá-los.
