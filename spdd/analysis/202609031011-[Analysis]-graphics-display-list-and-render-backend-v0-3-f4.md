# SPDD Analysis — v0.3 F4 (`core/graphics`): display list, render backend and text

| Campo        | Valor                                                                                           |
| ------------ | ----------------------------------------------------------------------------------------------- |
| Fase         | F4a + F4b do `ROADMAP-IMPLEMENTACAO-V1.md:261`, agrupadas na v0.3 "Primeiro pixel, headless"    |
| Fecha        | **C-14** (`PRD-005:87`), **C-17** (`PRD-005:90`)                                                |
| Port         | `RenderBackend` — mecanismo, seam PRD `PRD-005` (retrofit), **congela em F4** (`ADR-0011:121`)  |
| Depende de   | Nada além da fundação: `core/graphics` não conhece `dom`, `css`, nem `engine` (decisão 2.1)     |
| Estado atual | `core/graphics/src/lib.rs` tem 8 linhas — doc-comment e `#![forbid(unsafe_code)]`, zero funções |

## Original Business Requirement

Critérios de aceitação de `PRD-005:87-91`, verbatim (os itens 1, 4 e 5 são C-14, C-17 e C-18; C-18 é I2b, fora desta
análise):

```text
- [ ] `RenderBackend` trait defined in `core/graphics`.
- [ ] `VulkanBackend` (`vulkano`) initialized and capable of clearing/drawing display lists.
- [ ] Automatic fallback to `OpenGLBackend` (`glow`/`glutin`) when Vulkan instance creation fails.
- [ ] Automatic fallback to `SoftwareCpuBackend` when running headless without a GPU driver.
- [ ] Display list serialization and script binding tested with Rhai engine.
```

Comandos declarativos exigidos, verbatim de `PRD-005:65-70`:

```text
- `DrawRect { rect: Rect, color: Color, border_radius: f32 }`
- `DrawText { glyphs: Vec<GlyphInstance>, color: Color, font_id: FontId }`
- `DrawImage { image_id: ImageId, src_rect: Rect, dst_rect: Rect }`
- `DrawPath { path: Path2D, fill: Option<Color>, stroke: Option<Stroke> }`
- `PushClip { clip_rect: Rect }` / `PopClip`
- `PushOpacity { opacity: f32 }` / `PopOpacity`
```

Fronteira de script e saneamento, verbatim de `PRD-005:78-81`:

```text
- **Capability Gate**: Scripts require `GRAPHICS_DRAW`.
- **Safe Builder**: Scripts interact exclusively via `DisplayListBuilder` and `RenderPass` pipeline hooks.
- **Fault Trapping**: Malformed draw commands (e.g. `NaN` coordinates, out-of-bounds colors) are sanitized at
  the builder boundary and do not trigger GPU driver crashes.
```

Decisão de escopo 2 do `IMPLEMENTACAO-DETALHADA-V0-3.md:29-32`, verbatim:

```text
2. **O primeiro pixel inclui texto renderizado**, via porta `FontProvider` com `SystemFontProvider` (descoberta de
   fontes do sistema via filesystem pure-Rust + `ttf-parser`), fallback emergencial procedural/bitmap em container
   bare, e provedor sintético/mock determinístico para testes e goldens sem depender de assets binários.
```

## Domain Concept Identification

### Existing Concepts (from codebase)

- **`Capability::GRAPHICS_DRAW`** (`core/engine/src/domain/capability.rs:23`): o bit já existe, assim como os perfis
  `css_style()` (`:92`) e `ui_window()` (`:104`) que o concedem. Nenhuma capability nova é necessária — mas
  `core/graphics` **não** referencia `engine`; quem consome o bit é `core/runtime/rhai` no I2b.
- **`PORT_SCHEMA_VERSION`** (`core/engine/src/lib.rs:65`, hoje `2`): o padrão de versionamento de agregado de fronteira
  exigido pelo `ADR-0011:90-92`. `core/graphics` ganha o seu, começando em `1`.
- **`SourceLocation`** (`core/engine/src/domain/source.rs:43`): o modelo de "erro tipado com metadado de localização" do
  `ADR-0011:93-95`. Num display list a "linha" é o índice do comando.
- **Suíte de conformidade** (`core/engine/src/conformance.rs`): código `pub` de biblioteca, não `#[cfg(test)]`, chamado
  pelos `tests/` dos adaptadores. É o único caminho excluído do `arch-lint` (`arch-lint.toml:8-11`).
- **Erro tipado com `thiserror`** (`core/dom/src/domain/error.rs:10-11`): a convenção fora de `core/engine` (ADR-0015).
  `core/graphics` segue `core/dom`, não a carve-out manual do `core/engine`.

### New Concepts Required

- **`Au`** — unidade de comprimento inteira, 1/64 px (convenção 26.6). É a resposta ao problema de determinismo: soma e
  comparação de caixas viram aritmética inteira. Relaciona-se com `Px(f32)`, que é a unidade de **entrada** (comprimento
  vindo do autor/CSS), convertida por uma única função documentada.
- **`DisplayList`** — a sequência imutável de comandos declarativos; o agregado de fronteira do port. É o que desacopla
  layout de pixels e torna o I6 verificável.
- **`DisplayCommand`** — o vocabulário declarativo de `PRD-005:65-70`. Nasce inteiro (seis comandos) mesmo com dois
  deles recusados pelo backend do v0.3: o contrato é o que congela em F4, não a implementação.
- **`DisplayListBuilder`** — o único caminho de construção e o ponto de saneamento de `PRD-005:80`. Governa a pilha de
  clip/opacidade e a fronteira `f32 → Au`.
- **`RenderBackend`** — o port: recebe `DisplayList`, produz `Framebuffer`. Não conhece DOM nem CSS.
- **`BackendTier`** — o degrau da cascata (`Vulkan`/`OpenGL`/`Software`), necessário para que a queda seja observável e
  testável em vez de tautológica.
- **`Framebuffer`** — RGBA8 lido de volta pelo backend. É o objeto que a golden image compara, não o PNG.
- **`FontProvider` / `FontCatalog` / `FontId`** — a obtenção de faces e métricas, isolada atrás de porta para que a CI
  use métricas sintéticas e o runtime use fontes do SO.
- **`GlyphInstance`** — glifo posicionado; a unidade que `DrawText` carrega.
- **`GraphicsError` / `CommandIndex`** — o erro único do port e seu metadado de localização.

### Key Business Rules

- **Um comando não-sanitizável nunca chega ao backend.** Coordenada não-finita ou dimensão negativa não tem
  interpretação correta: é recusa com erro tipado. Governa `DisplayListBuilder`, `DisplayCommand`.
- **Uma página legítima nunca é recusada por ser grande.** Valor finito fora do envelope clampa e segue; o mesmo para
  `Opacity` fora de `[0,1]`. Governa `DisplayListBuilder`, `Opacity`, `Rect`.
- **A pilha de clip/opacidade é balanceada por construção.** `Pop*` sem `Push*` corrompe todo o resto do frame, então é
  recusado na construção, não no backend. Governa `DisplayListBuilder`.
- **A mesma entrada produz o mesmo framebuffer, em qualquer SO.** Governa `Au`, a conversão `Px → Au`, o achatamento de
  Bézier e o cache de glifo.
- **O cache nunca altera o resultado.** Governa `FontCatalog` e o cache de glifo rasterizado.
- **A cascata sempre devolve um backend utilizável.** Nunca `todo!()`, nunca ausência: cada degrau indisponível devolve
  `BackendUnavailable { tier }` e o algoritmo desce. Governa `select_backend`, `BackendTier`.

## Strategic Approach

### Solution Direction

`core/graphics` é um crate de domínio puro em três camadas que recebe uma `DisplayList` e devolve pixels, sem conhecer
DOM, CSS ou engine de script. O fluxo é `DisplayListBuilder → DisplayList → Box<dyn RenderBackend> → Framebuffer → PNG`.
O port é object-safe desde a assinatura, o adaptador de software é a referência contra a qual Vulkan e OpenGL se
provarão no F12, e o texto entra por uma porta (`FontProvider`) para que a CI possa ser determinística sem versionar
assets binários.

### Key Design Decisions

- **`RenderBackend` object-safe direto, sem companion `dyn`**: todo método fala só tipos do próprio crate. Trade-off —
  perde-se o açúcar genérico que `RuntimeEngine` tem; ganha-se satisfazer o item 2 do `ADR-0011:87-89` sem repetir o
  `ADR-0013`. → Recomendado: o contraste com `RuntimeEngine` é informação de arquitetura e fica registrado no contract
  record.
- **`read_back` na trait, não só no backend de software**: trade-off — obriga Vulkan a implementar staging buffer no
  F12. → Recomendado: é exatamente o que torna o I6 verificável; sem isso não há como comparar Vulkan contra a golden do
  software.
- **`Au(i32)` para geometria calculada, `Px(f32)` só na entrada**: trade-off — toda aritmética passa a exigir
  `checked_*`/`saturating_*` sob o portão de clippy, o que custa tempo de escrita e legibilidade. → Recomendado: é a
  única forma de a golden bater byte a byte nos três SOs, que é o portão que a v0.3 liga (`roadmap:357`).
- **Codificador PNG próprio com deflate armazenado**: trade-off — arquivo maior e ~120 linhas escritas à mão (CRC-32,
  Adler-32). → Recomendado: a alternativa puxa quatro crates para escrever o único artefato de saída da versão, e a
  golden compara framebuffer, não PNG, então o tamanho não é portão de nada.
- **Duas regras distintas de saneamento (recusar vs. clampar)**: trade-off — mais superfície de teste do que uma regra
  só. → Recomendado: clampar tudo esconderia bug de layout; recusar tudo quebraria página legítima.
- **`FontProvider` como porta, com provedor sintético nos testes**: trade-off — uma indireção a mais no caminho quente
  do texto. → Recomendado: sem ela, a golden de texto fica refém das fontes instaladas no runner de CI — a armadilha que
  o `ROADMAP-IMPLEMENTACAO-V1.md:315` nomeia.

### Alternatives Considered

- **Rasterizador de fonte de terceiros (`fontdue`, `ab_glyph`, FreeType)**: rejeitado. FreeType é C e cai na primeira
  linha da regra proposta em S-01 do `VIOLACAO-N02-UNSAFE-NO-RHAI.md` (parsing de bytes escolhidos pelo autor da
  página). Os puros-Rust trariam sua própria política de arredondamento, que é justamente a variável que o determinismo
  cross-OS precisa controlar.
- **`f32` em toda a geometria, com arredondamento no fim**: rejeitado. Acumula erro e depende de modo de arredondamento;
  a golden nos três SOs seria uma loteria.
- **Adiar todo o texto para o v0.5**: rejeitado pela decisão de escopo 2 — um "primeiro pixel" sem texto não exercita
  `DrawText`, e a fronteira `css ↔ fonte` (a porta `TextMeasurer` da F4c) só aparece com texto real.
- **`DisplayList` com só os comandos implementados**: rejeitado. O port congela em F4 (`ADR-0011:121`); um comando
  adicionado depois exige bump de schema. Declarar os seis e recusar dois é mais barato.

## Risk & Gap Analysis

### Requirement Ambiguities

- **`PRD-005:65-70` mistura unidades**: `border_radius: f32` e `opacity: f32` no mesmo enum em que `rect: Rect` deveria
  ser inteiro. Resolução adotada: a assinatura pública do **builder** aceita `f32` (fronteira), o `DisplayCommand`
  armazenado guarda `Au`/`Opacity`. A conversão acontece exatamente uma vez.
- **`Path2D` e `Stroke` (`PRD-005:68`) não têm forma especificada em lugar nenhum.** Resolução: `DrawPath` é declarado
  com tipos mínimos e recusado com `Unsupported`; a forma real fica para a versão que o implementar.
- **`PRD-005` não numera seus critérios** — não existe a string "C-14" no arquivo. A numeração vem do roadmap e do
  `CLAUDE.md`. O retrofit da fase P deve introduzir os rótulos no próprio PRD.

### Edge Cases

- **Página muito alta (10.000 px)**: se `MAX_EXTENT` for escolhido pequeno demais, o clamp corta página legítima. Exige
  teste explícito que prove que não é cortada.
- **`Framebuffer` de dimensão zero**: `begin_frame` com largura ou altura 0 — decidir entre erro tipado e no-op
  observável antes de escrever o backend.
- **Contêiner bare sem nenhuma fonte instalada**: `SystemFontProvider` não encontra face alguma; sem fallback, toda
  renderização headless falha.
- **Glifo ausente do `cmap`**: char sem mapeamento precisa de comportamento definido (`.notdef`), não de erro.
- **Pilha de clip aberta no fim do frame**: `Push` sem `Pop` no fim da lista — é erro de construção ou fecha
  implicitamente? Decidir na regra de balanceamento.

### Technical Risks

- **Determinismo cross-OS é o portão mais frágil da versão** (risco §6.2 do relatório): a primeira golden de texto que
  divergir entre Linux e macOS custa dias de bisecção. Mitigação de processo: rodar o job de determinismo já na golden
  de **caixas** da F4a passo 6, quando a superfície de investigação ainda é pequena.
- **O portão de clippy do workspace proíbe o vocabulário de um rasterizador**: `arithmetic_side_effects`,
  `as_conversions`, `indexing_slicing` e `string_slice` são todos `deny` (`Cargo.toml:51-66`). Mitigação: `domain/`
  mantém o portão integral com `checked_*`/`TryFrom`/`.get()`; `#[allow]` por função, comentado e citando o `ADR-0017`,
  só em `infrastructure/software/` e `infrastructure/png.rs`.
- **⚠️ Achado que contradiz o plano-fonte**: `IMPLEMENTACAO-DETALHADA-V0-3.md:462` afirma que o workspace tem "zero
  `#[allow(...)]` em código de produção". São **oito**, incluindo
  `#[allow(clippy::as_conversions, clippy::cast_precision_loss)]` em `core/engine/src/domain/value.rs:85`, dentro de
  `domain/`. O portão `allow-count` da fase P precisa baselinar no número real.
- **`ttf-parser` é a primeira dependência externa nova desde a v0.1**, e o risco §6.5 do relatório declara dependência
  de ordem: rodar `cargo-geiger` na árvore atual e registrar a saída **antes** de fixá-la.
- **Nenhum crate do workspace declara `[features]` hoje** — a feature `no-backend` exigida pelo `ADR-0011:99-102`
  estreia o mecanismo. O `cargo test -p dom --no-default-features` que a CI já roda (`ci.yml:113`) é um no-op.
- **`arch-lint.toml` não tem escopo para `graphics`** — o crate entraria sem regra de camada nenhuma.

### Acceptance Criteria Coverage

| AC#          | Description                                                       | Addressable? | Gaps/Notes                                                           |
| ------------ | ----------------------------------------------------------------- | ------------ | -------------------------------------------------------------------- |
| `PRD-005:87` | `RenderBackend` trait defined in `core/graphics` (**C-14**)       | Yes          | Guardado por `run_backend_suite` passando para dois adaptadores      |
| `PRD-005:88` | `VulkanBackend` inicializado e desenhando (**C-15**)              | No           | Declarado fora do v0.3 — é F12. A F4a só escreve o degrau da cascata |
| `PRD-005:89` | Fallback automático para `OpenGLBackend` (**C-16**)               | No           | Idem — F12                                                           |
| `PRD-005:90` | Fallback automático para `SoftwareCpuBackend` headless (**C-17**) | Yes          | Fechado de verdade: teste força cada tier a falhar via env var       |
| `PRD-005:91` | Serialização de display list + binding Rhai (**C-18**)            | Partial      | A serialização textual e o binding são **I2b**, não esta fase        |
