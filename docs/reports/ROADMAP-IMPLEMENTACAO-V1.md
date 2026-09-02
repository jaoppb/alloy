# Roadmap de implementação — do bootstrap até a entrega da v1.0

| Campo               | Valor                                                                                                          |
| ------------------- | -------------------------------------------------------------------------------------------------------------- |
| **Status**          | ❌ Não iniciado — os 11 crates são stubs gerados por `cargo new`; não existe binário executável                |
| **Cobertura**       | ~0% (0 de 18 critérios de aceite dos PRDs)                                                                     |
| **Esforço**         | 239–363 dias-dev `[modelado]` no escopo completo, em 7 releases; 92–140 d até a v0.3 desenhar o primeiro pixel |
| **Depende de**      | Nada — F0 pode começar hoje                                                                                    |
| **Atenção**         | ⚠️ Os cinco PRDs miram `v0.1.0-alpha`. **Não existe documento de escopo para v1.0** — a seção 2 o define       |
| **Fecha requisito** | PRD-001 a PRD-005 (integral)                                                                                   |

> **Nota de extensão — 2026-09-02.** Este roadmap não previa medição de desempenho além do `criterion` da F13. O
> `PRD-009` acrescenta a **trilha E (medição)** e as fases **B0–B7** — harness de benchmarks web em container
> (Speedometer, JetStream, MotionMark, Basemark Web), matriz de tiers de hardware, perfis de uso padrão e avançado e a
> tabela de requisitos mínimos de sistema. Plano, esforço (51–78 d `[modelado]`) e encaixe por versão:
> `BENCHMARKS-WEB-E-REQUISITOS-MINIMOS.md`. Decisões em `ADR-0016` (containers e as duas lanes) e `ADR-0017` (tiers,
> orçamentos e o que pode ser alegado). Critérios novos: **C-19 … C-28**, continuando a numeração da seção 2. Portões
> novos: `bench-smoke` na v0.3, gate de regressão noturno na v0.5, tabela de requisitos mínimos na v1.0.

---

## 1. Estado atual — evidências

### O que existe

O workspace tem 11 membros declarados explicitamente em `Cargo.toml:1-13`, com o glob `core/runtime/*` reservado para
futuros backends de script. Todos os 11 crates compilam e todos passam nos testes: `cargo test --workspace` produz 22
linhas `test result: ok`, correspondendo a **11 testes reais** — um `it_works()` por crate — e 11 suítes vazias de
doc-test.

O conteúdo de cada crate é idêntico e tem 14 linhas:

```rust
// core/engine/src/lib.rs:1-3 — idêntico nos 11 crates
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}
```

A infraestrutura de qualidade local existe e é séria: `lefthook.yml:1-33` roda `cargo fmt`, `clippy -D warnings`,
`prettier` e `markdownlint-cli2` no pre-commit, e `cargo test` + `cargo check` no pre-push. `package.json:7-15` agrega
isso em `pnpm check`.

A especificação está madura e é utilizável como contrato de implementação — é o ativo mais valioso do repositório hoje.
As assinaturas de `RuntimeEngine` e `ExecutionContext` estão escritas em `PRD-002:31-56`; os nove bitflags de
`Capability` em `PRD-003:35-47`; os perfis de capability por subsistema em `PRD-003:55-58`; o fluxo de swap atômico em
`PRD-004:37-63`; a cascata de três tiers gráficos em `PRD-005:37-49`; e os comandos de `DisplayList` em `PRD-005:65-72`.

### O que não existe

Nenhuma das ausências abaixo é uma impressão — cada uma é uma busca com resultado zero:

```bash
find . -name "main.rs" -not -path "./target/*"; grep -rn "\[\[bin\]\]" --include="*.toml" .
# → 0 resultados
```

**Não existe binário.** O Alloy não é executável de forma alguma hoje: são 11 bibliotecas sem ponto de entrada. Este é o
item mais subestimado do inventário, porque um browser é uma aplicação, e nenhuma das fases seguintes é demonstrável sem
um crate `alloy` que amarre janela, event loop e subsistemas.

```bash
grep -rn "^\[dependencies\]" -A3 --include=Cargo.toml core devtools extension
# → única dependência: engine = { path = "../../engine" } em core/runtime/rhai/Cargo.toml:7
```

**Zero dependências externas.** Não há `rhai`, `vulkano`, `glow`, `glutin`, `winit`, `bitflags`, `notify` nem
`boa_engine` declarados. Toda tecnologia citada nos ADRs é, hoje, uma intenção.

```bash
ls .github rust-toolchain.toml deny.toml .cargo spdd
# → 0 resultados
find . -type d \( -name benches -o -name fuzz -o -name tests \)
# → 0 resultados
find . -name "*.rhai"
# → 0 resultados
grep -rn "forbid(unsafe_code)\|rust-version\|\[workspace.package\]\|\[workspace.dependencies\]" .
# → 0 resultados
```

Sem CI, sem toolchain fixada, sem MSRV, sem auditoria de dependências, sem testes de integração, sem benchmarks, sem
fuzzing, sem versões centralizadas no workspace — e **nenhum script `.rhai`**, nem sequer um exemplo. A pasta `spdd/`
exigida por `PRD-001:100` também não existe, embora o ADR-0007 a declare obrigatória para todo incremento funcional.

### ⚠️ Quatro defeitos já presentes no código atual

Nenhum deles é causado por este roadmap — mas os quatro estão no caminho do trabalho da F0, e o custo de corrigi-los
agora é próximo de zero.

**1. `rustfmt` não está instalado — o portão de qualidade está quebrado hoje.**

```bash
rustup component list --installed
# → cargo, clippy, rust-std, rustc — rustfmt ausente
cargo fmt --all --check
# → error: 'cargo-fmt' is not installed for the toolchain 'stable-x86_64-unknown-linux-gnu'
```

O hook `rust-fmt` de `lefthook.yml:7-11` roda `cargo fmt` em todo commit que toque `.rs`. Na máquina atual, **qualquer
commit de código Rust falha**. Correção: `rustup component add rustfmt`, e fixar a toolchain para que isso não dependa
da máquina de cada dev.

**2. `Cargo.lock` está ignorado.** `.gitignore:4` remove o lockfile do versionamento. Isso é a convenção para
bibliotecas, e o **oposto** do correto para uma aplicação: destrói a reprodutibilidade do build entre devs e CI, e torna
impossível auditar supply-chain com `cargo-deny`, porque não há árvore de versões congelada para auditar. Com um browser
puxando `vulkano`, um motor JS e uma pilha TLS, isso deixa de ser detalhe.

**3. A tabela de dependências entre crates é ficção.** `docs/architecture/overview.md:80-92` documenta `dom → engine`,
`html → dom, engine`, `window → graphics, engine` e assim por diante. Nenhum `Cargo.toml` declara **nenhuma** dessas
relações. É o alvo, não o estado — e a distinção precisa ficar explícita no documento antes que alguém a leia como
descrição.

**4. `edition = "2024"` sem `rust-version`.** Os 11 manifestos exigem a edição 2024 e nenhum declara MSRV. Um
contribuidor com toolchain antiga descobre o problema num erro de compilação obscuro, não numa mensagem de requisito.

---

## 2. Requisitos × estado, critério a critério

Os cinco PRDs somam **18 critérios de aceite** formais. Todos estão em `❌`. A tabela é o inventário completo do que a
v1.0 precisa fechar.

| #        | Critério                                                                     | Origem       | Status |
| -------- | ---------------------------------------------------------------------------- | ------------ | ------ |
| **C-01** | Traits `RuntimeEngine` e `ExecutionContext` definidas em `core/engine`       | `PRD-002:87` | ❌     |
| **C-02** | `RhaiEngine` em `core/runtime/rhai` passando testes de conformidade da trait | `PRD-002:88` | ❌     |
| **C-03** | Struct de domínio (`DomNode`) legível e mutável a partir de script Rhai      | `PRD-002:89` | ❌     |
| **C-04** | Loop infinito abortado com `EngineError::ExecutionLimitExceeded`             | `PRD-002:90` | ❌     |
| **C-05** | Teste com engine mockado provando troca sem tocar crates de domínio          | `PRD-002:91` | ❌     |
| **C-06** | Verificação de capability em **todo** binding de função nativa               | `PRD-003:76` | ❌     |
| **C-07** | Capability não autorizada retorna `EngineError::PermissionDenied`            | `PRD-003:77` | ❌     |
| **C-08** | Subsistemas mantêm `ExecutionContext` isolados, com escopos separados        | `PRD-003:78` | ❌     |
| **C-09** | Script que entra em pânico não derruba o host e aciona o fallback            | `PRD-003:79` | ❌     |
| **C-10** | Watcher detecta modificação de `.rhai` com debounce                          | `PRD-004:79` | ❌     |
| **C-11** | Edição válida compila em background e troca atomicamente                     | `PRD-004:80` | ❌     |
| **C-12** | Script com erro de sintaxe não substitui o AST ativo e loga diagnóstico      | `PRD-004:81` | ❌     |
| **C-13** | DOM e estado de janela intactos após múltiplos hot-reloads                   | `PRD-004:82` | ❌     |
| **C-14** | Trait `RenderBackend` definida em `core/graphics`                            | `PRD-005:87` | ❌     |
| **C-15** | `VulkanBackend` inicializando e desenhando display lists                     | `PRD-005:88` | ❌     |
| **C-16** | Fallback automático para `OpenGLBackend` quando Vulkan falha                 | `PRD-005:89` | ❌     |
| **C-17** | Fallback automático para `SoftwareCpuBackend` em headless                    | `PRD-005:90` | ❌     |
| **C-18** | Serialização de display list e binding com Rhai testados                     | `PRD-005:91` | ❌     |

Somam-se cinco requisitos não-funcionais, que **não têm critério de aceite mensurável escrito** e por isso precisam de
um número antes de virarem verificáveis — a seção 5 propõe esses números.

| NFR      | Texto                                                    | Origem        | Problema para verificar                                         |
| -------- | -------------------------------------------------------- | ------------- | --------------------------------------------------------------- |
| **N-01** | Overhead Rust→Engine `<10μs` por hook de evento          | `PRD-001:96`  | Falta definir a carga de referência e o percentil (p50? p99?)   |
| **N-02** | Zero operações `unsafe` expostas a runtimes de script    | `PRD-001:97`  | Não há lint configurada que force isso; hoje é honra pessoal    |
| **N-03** | 100% de isolamento de crash entre scripts de subsistemas | `PRD-001:98`  | "100%" só é aferível por teste de injeção de falha, inexistente |
| **N-04** | Todo crate de domínio testável com e sem engine anexado  | `PRD-001:99`  | Exige feature flag de teste que ainda não existe                |
| **N-05** | Todo incremento funcional com prompt SPDD correspondente | `PRD-001:100` | `spdd/` não existe; é débito de processo desde o commit inicial |

### 2.1 O que a v1.0 entrega — definição de pronto

A v1.0 prova a tese do produto: **um browser cuja mecânica é Rust e cuja política é script trocável a quente**, capaz de
abrir uma página real simples e de ser reconfigurado sem recompilar.

Entra no escopo: os 18 critérios acima; um binário `alloy` multiplataforma (Linux, macOS, Windows); HTML5 tokenizer e
tree builder cobrindo o subconjunto de páginas estáticas; CSS com seletores, cascata, box model e layout de fluxo normal
mais Flexbox; rede com HTTP/1.1 sobre TLS; texto com fontes do sistema; imagens raster; e `core/js` executando
JavaScript de conteúdo com bindings de DOM.

Fica **fora** da v1.0, e deve ser dito em voz alta para não voltar como surpresa: HTTP/2 e HTTP/3, CSS Grid, animações e
transições CSS, WebGL e Canvas 2D, WebAssembly, a ponte WebExtensions do crate `extension`, vídeo e áudio, e
acessibilidade por API de plataforma. O crate `extension` permanece stub na v1.0 — e isso deve constar do release notes.

### 2.2 Qual runtime de JavaScript adotar

Incluir `core/js` na v1.0 foi decisão explícita, e é o maior multiplicador de escopo do roadmap. A decisão que resta é
**não escrever um motor de JavaScript**.

| Opção            | O que é                                     | Custo                                                                                             | Veredito                                                                 |
| ---------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| **`boa_engine`** | Motor ECMAScript em Rust puro               | Cobertura de `test262` incompleta e desempenho bem abaixo de JIT; ainda em evolução de API        | **Recomendada**                                                          |
| `rquickjs`       | Binding para QuickJS (C)                    | Exige toolchain C nos 3 SOs da matriz; `unsafe` na fronteira colide com **N-02** (`PRD-001:97`)   | Rejeitada: contradiz a tese de segurança do produto                      |
| `rusty_v8`       | Binding para V8 (C++)                       | Build de dezenas de minutos, binário muito grande, toolchain C++ e cross-compile penoso nos 3 SOs | Rejeitada: custo de build e de portabilidade desproporcional para a v1.0 |
| Motor próprio    | Parser, interpretador e GC escritos do zero | Anos-dev antes do primeiro `alert()`                                                              | Rejeitada: consumiria sozinho o cronograma inteiro                       |

`boa_engine` é a escolha coerente com o projeto: mantém a matriz de CI sem compilador C/C++ e preserva **N-02**. O custo
assumido é explícito: **páginas com JavaScript moderno pesado não vão funcionar na v1.0**, e a taxa de `test262`
(seção 5) existe justamente para tornar esse limite visível em vez de vergonhoso.

O que **não** se deve fazer é encaixar o motor JS na trait `RuntimeEngine` de `PRD-002:31-56`. Essa trait é do Muscle —
`ADR-0006:63-68` separa "Web Content JavaScript (`core/js`)" de "Browser Muscle Engine (`core/engine` +
`core/runtime/rhai`)" como responsabilidades distintas, e a seção 2.3 mostra que os dois lados nem sequer têm o mesmo
modelo de ameaças. A forma de `RuntimeEngine` também não serve: `create_context`, `compile` e `eval` não modelam _realm_
por aba, `Origin`, fila de microtasks nem event loop, que é justamente o que o runtime de conteúdo precisa.

O desacoplamento que interessa é obtido repetindo o **padrão**, não a trait: `core/js` define a sua própria porta em
`application/`, com `boa_engine` como um adaptador em `infrastructure/`. Trocar de motor JS continua sendo trocar um
adaptador, e as duas fronteiras de script continuam sem se misturar.

### 2.3 ⚠️ Lacuna de especificação: JS de conteúdo é código hostil

O modelo de ameaças de `PRD-003:21-24` descreve quatro cenários — script com bug, loop infinito, escalada de privilégio
e poluição entre abas — e todos pressupõem **um script escrito pelo usuário**, isto é, alguém que quer que o browser
funcione. JavaScript de página é a categoria oposta: código arbitrário de terceiros, potencialmente adversário,
executando a cada navegação.

O enum `Capability` de `PRD-003:35-47` **não tem nenhuma flag que distinga conteúdo de muscle**. Pelo modelo atual, o
script de uma página e o script de customização do usuário são cidadãos do mesmo tipo, diferindo apenas nos bits
concedidos. Isso é insuficiente: falta _same-origin policy_, falta isolamento por aba com mecanismo definido —
`PRD-003:24` nomeia o risco mas não prescreve a defesa — e falta decidir se abas compartilham processo.

A v1.0 precisa de três adições ao PRD-003, e a fase **F7** as implementa: um perfil `WEB_CONTENT` que concede apenas
`DOM_READ | DOM_MUTATE` sobre a árvore _da própria aba_, uma noção de `Origin` como value object carregado no
`ExecutionContext`, e um limite de instruções por _task_ distinto do limite de script muscle. **Atualizar o PRD-003 é
entregável de F7**, não trabalho opcional de documentação.

---

## 3. Escada de releases e fases

Fase é unidade de trabalho; **release é o que se entrega**. As 14 fases abaixo se agrupam em sete versões, e cada versão
só sai quando os seus micro-entregáveis rodam de verdade — nada de versão que existe só na tabela de planejamento.

### 3.1 As sete versões

| Versão   | Nome                     | Fases               | O que passa a ser possível                                                | Fecha                         | Esforço `[modelado]` |
| -------- | ------------------------ | ------------------- | ------------------------------------------------------------------------- | ----------------------------- | -------------------- |
| **v0.1** | O engine vive            | F0 + F1 + F2        | Rodar script Rhai a partir do binário, com limite de execução             | C-01, C-02, C-04, C-05        | 28–42 d              |
| **v0.2** | DOM scriptável e contido | F3 + F6 · I1        | Script monta e muta uma árvore DOM dentro do sandbox de capabilities      | C-03, C-06, C-07, C-08, C-09  | 24–36 d              |
| **v0.3** | Primeiro pixel, headless | F4 + F5 · I2        | Transformar um `.html` em `.png` sem janela e sem GPU                     | C-14, C-17                    | 40–62 d              |
| **v0.5** | Browser de verdade       | F8 + F9 · I4        | Abrir uma URL real em janela nativa nos 3 SOs, com CSS de fluxo e Flexbox | —                             | 50–75 d              |
| **v0.7** | JavaScript de conteúdo   | F7 + F10 · I3       | Página com `<script>` alterando o DOM, com fronteira de origem aplicada   | Lacuna 2.3 + PRD-003          | 45–70 d              |
| **v0.9** | Maleável e acelerado     | F11 + F12 · I5 · I6 | Hot-reload sem perder a página aberta, e Vulkan com cascata de fallback   | C-10 a C-13, C-15, C-16, C-18 | 37–53 d              |
| **v1.0** | Endurecido               | F13                 | Instalar e usar: todos os portões verdes e pacote nos 3 SOs               | Os 18 critérios               | 15–25 d              |

**Micro-entregáveis por versão** — cada item é uma coisa que alguém consegue rodar e ver. Se não roda, a versão não
saiu.

**v0.1** — `cargo run` abre e fecha o binário nos três SOs · `alloy --script hello.rhai` executa e imprime o retorno ·
um script em laço infinito é abortado por limite de instruções · um engine mockado substitui o Rhai sem que nenhum crate
de domínio mude uma linha.

**v0.2** — um script Rhai constrói uma árvore DOM e a serializa na saída · um script sem `DOM_MUTATE` recebe
`PermissionDenied` ao tentar escrever · um script que entra em pânico é contido e o processo continua vivo, com o
fallback assumindo.

**v0.3** — `alloy render pagina.html -o saida.png` produz um PNG byte a byte determinístico · a golden image roda em CI
sem GPU · o subconjunto declarado da suíte html5lib fica verde.

**v0.5** — `alloy https://example.com` abre janela nativa e renderiza a página real · redimensionar a janela refaz o
layout · **é a primeira versão apresentável a quem não é do time**, e a primeira candidata a release público.

**v0.7** — uma página com `<script>` altera o DOM e a tela repinta · script de uma origem não alcança o DOM de outra aba
· a taxa de `test262` do subconjunto passa a ser publicada a cada release.

**v0.9** — editar um `.rhai` com o browser aberto muda o comportamento sem recarregar a página · com Vulkan disponível
ele é usado, e forçar a falha derruba para OpenGL e depois para software, sem que layout ou DOM percebam.

**v1.0** — pacote instalável nos três SOs · os 18 critérios verdes · release notes dizendo em voz alta o que **não**
existe (HTTP/2, Grid, animações, WebGL, Wasm, WebExtensions, vídeo, áudio).

### 3.2 As fases

Quatro trilhas correm em paralelo com 2–4 devs. **A** — runtime e sandbox. **B** — parsing, DOM, CSS e layout. **C** —
gráficos, janela e rede. **D** — `core/js` e bindings de DOM. As fases são sequenciais **dentro** de cada trilha; entre
trilhas, o que manda são os pontos de integração.

| Fase    | Trilha | Conteúdo                                                                                                                                                                    | Entregável verificável                                       | Esforço `[modelado]` |
| ------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | -------------------- |
| **F0**  | —      | Fundação: `rustfmt`, `rust-toolchain.toml`, MSRV, `Cargo.lock` versionado, `[workspace.dependencies]`, CI 3 SOs, `cargo-deny`, `forbid(unsafe_code)`, crate binário `alloy` | `pnpm check` verde nos 3 SOs; `cargo run` abre e fecha       | 6–9 d                |
| **F1**  | A      | `core/engine`: traits, `EngineValue`, `CapabilitySet`, `EngineError`, conversões `Into/FromEngineValue`                                                                     | **C-01**, **C-05** fechados com engine mockado               | 10–15 d              |
| **F2**  | A      | `core/runtime/rhai`: compilação de AST, escopo, registro de tipos, limites de execução                                                                                      | **C-02**, **C-04**                                           | 12–18 d              |
| **F3**  | B      | `core/dom`: `NodeId`, arena, `Children`, invariantes de árvore, travessia                                                                                                   | Testes de aciclicidade e de mutação                          | 12–18 d              |
| **F4**  | C      | `core/graphics`: `DisplayList`, `RenderBackend`, `SoftwareCpuBackend` primeiro                                                                                              | **C-14**, **C-17**; golden images em CI sem GPU              | 15–22 d              |
| **F5**  | B      | `core/html`: tokenizer e tree builder do subconjunto estático                                                                                                               | Suíte html5lib do subconjunto                                | 25–40 d              |
| **F6**  | A      | Sandbox: checagem por binding, contextos isolados, trapping e fallback                                                                                                      | **C-06**, **C-07**, **C-08**, **C-09**                       | 12–18 d              |
| **F7**  | A+D    | Fronteira de conteúdo: perfil `WEB_CONTENT`, `Origin`, isolamento por aba, **atualização do PRD-003**                                                                       | Teste de escalada entre abas falhando por `PermissionDenied` | 10–15 d              |
| **F8**  | C      | `core/window` + `core/network`: janela nos 3 SOs, event loop, HTTP/1.1 sobre TLS                                                                                            | Página remota baixada e exibida em bitmap                    | 20–30 d              |
| **F9**  | B      | `core/css`: tokenizer, seletores, cascata, box model, fluxo normal + Flexbox                                                                                                | Testes de layout por asserção de retângulo                   | 30–45 d              |
| **F10** | D      | `core/js`: `boa_engine`, bindings de DOM, event loop, microtasks, `<script>` no parser                                                                                      | `document.getElementById().textContent = …` alterando a tela | 35–55 d              |
| **F11** | A      | Hot-reload: watcher com debounce, compilação em background, swap `Arc<AST>`, `on_reload()`                                                                                  | **C-10** a **C-13**                                          | 12–18 d              |
| **F12** | C      | `VulkanBackend` e `OpenGLBackend` com cascata de fallback                                                                                                                   | **C-15**, **C-16**, **C-18**                                 | 25–35 d              |
| **F13** | —      | Endurecimento: fuzzing, `test262`, benchmarks, empacotamento, release notes                                                                                                 | Todos os portões da seção 5 verdes                           | 15–25 d              |

### 3.3 Pontos de integração

Onde as trilhas obrigatoriamente se encontram — e onde vale marcar reunião:

| Ponto  | Junta     | Contrato que precisa estar de pé                                                                      | Depois de |
| ------ | --------- | ----------------------------------------------------------------------------------------------------- | --------- |
| **I1** | A ↔ B     | `DomNode` registrado no engine e mutável por script — fecha **C-03**                                  | F2 + F3   |
| **I2** | B ↔ C     | Árvore DOM vira `DisplayList` e chega ao rasterizador de software: **primeiro pixel**                 | F4 + F5   |
| **I3** | B ↔ D     | `dom` estável o bastante para os bindings de JS; congelar a API pública de `dom` aqui                 | F5 + F7   |
| **I4** | C ↔ todos | Binário abre janela real nos 3 SOs, busca URL remota e renderiza: **primeira demo apresentável**      | F8 + I2   |
| **I5** | A ↔ D     | Event loop de JS convive com o watcher de hot-reload sem deadlock e sem _starvation_                  | F10 + F11 |
| **I6** | C ↔ todos | Vulkan e OpenGL entram sem que layout ou DOM percebam — prova de que `DisplayList` desacoplou de fato | F12       |

**Mínimo viável** (a tese provada, headless, sem JS e sem GPU): tudo até a **v0.3** ≈ **92–140 d**. **Escopo completo da
v1.0:** as sete versões ≈ **239–363 dias-dev**.

Traduzir isso em calendário exige assumir a eficiência do paralelismo, que é onde estimativas costumam mentir. Com 3
devs e aproveitamento de ~65% `[modelado]` — trilhas que esperam umas pelas outras nos pontos de integração, revisão de
código, incidentes —, são ~123–186 dias úteis, ou **6–9 meses de calendário**. Com 2 devs, o paralelismo de quatro
trilhas deixa de existir e o intervalo vai para 9–14 meses.

A ordem não é negociável em dois pontos. **F4 antes de F12**: escrever o rasterizador de software primeiro dá a CI um
backend determinístico e sem GPU para golden images desde o início — inverter isso significa não ter como testar
renderização até quase o fim. **F7 antes de F10**: definir a fronteira de conteúdo hostil antes de existir um motor JS
evita retrofit de segurança, que é o modo mais caro e mais malsucedido de adicionar isolamento.

---

## 4. Armadilhas

Armadilha é técnica e imediata; risco de cronograma está na seção 7.

| Armadilha                                                                                                                                | Mitigação                                                                                                                                           |
| ---------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rustfmt` ausente quebra o hook de `lefthook.yml:7-11` em todo commit de `.rs`                                                           | `rustup component add rustfmt` na F0 e `rust-toolchain.toml` com `components = ["rustfmt", "clippy"]`                                               |
| `Cargo.lock` ignorado em `.gitignore:4` impede build reproduzível e auditoria                                                            | Remover a linha na F0 e versionar o lockfile antes da primeira dependência externa entrar                                                           |
| Glob `core/*` volta a ser "simplificado" e casa `core/runtime/`, que não tem manifesto — aborta todo comando Cargo                       | Membros explícitos em `Cargo.toml:1-13`, com o motivo registrado no ADR-0006                                                                        |
| NFR de `<10μs` por hook (`PRD-001:96`) versus checagem de capability em **todo** binding (`PRD-003:76`)                                  | Resolver capability na criação do contexto, não por chamada; `criterion` medindo o hook desde a F1, não no fim                                      |
| Object Calisthenics regra 3 e 4 (`ADR-0010:131-132`) em loop quente de tokenizer: newtype por caractere é catastrófico                   | Declarar `domain/` como território das regras e o interior do tokenizer como exceção medida; registrar a exceção em ADR                             |
| `Arc<AST>` trocado enquanto um script executa (`PRD-004:52`)                                                                             | Clonar o `Arc` no início da chamada; a troca só afeta a invocação seguinte — e isso vira teste explícito                                            |
| Reset de escopo do hot-reload apaga estado que o script assumia persistente                                                              | `on_reload()` de `PRD-001` §5.2 é o único caminho de re-hidratação; documentar que scripts são sem estado                                           |
| `NaN` e coordenadas absurdas chegando ao driver (`PRD-005:80`)                                                                           | Sanitizar em `DisplayListBuilder`, com teste de propriedade alimentando `NaN`, `inf` e valores extremos                                             |
| Três backends gráficos triplicam manutenção e teste                                                                                      | Software é o backend de referência; Vulkan e OpenGL provam-se contra as golden images dele                                                          |
| Rasterização de fonte difere entre Linux, macOS e Windows e quebra golden images                                                         | Fonte embarcada e rasterizador próprio nos testes de referência; nunca fonte do sistema em golden image                                             |
| `<script>` síncrono bloqueia o tokenizer e `document.write` reentra nele — o pipeline imutável de `ADR-0010:114` não acomoda reentrância | Tratar o tokenizer como máquina de estados suspensível desde a F5; **não** descobrir isso na F10                                                    |
| `dom` precisa despachar eventos para `js` enquanto `js` depende de `dom` (`overview.md:84`) — ciclo de crates                            | Porta (trait) em `dom/application/` implementada por `js`; o ciclo se quebra por inversão de dependência, não por crate utilitário compartilhado    |
| Event loop de JS versus watcher de hot-reload de `PRD-004:32-34`: dois laços disputando a thread principal                               | Um único event loop dono da thread; watcher e compilação em worker, comunicando por canal — verificado em **I5**                                    |
| `boa_engine` ainda muda API entre versões                                                                                                | Fixar versão exata no lockfile e isolar todo uso atrás de `js/infrastructure/`, nunca vazando tipos de `boa` para `domain/`                         |
| `spdd/` inexistente enquanto `PRD-001:100` a exige em todo incremento                                                                    | Criar `spdd/` na F0 e gerar o primeiro canvas para a própria F1, ou revisar o ADR-0007 assumindo que a regra não vale — as duas saídas são honestas |

---

## 5. Portões de qualidade e métricas

Todo portão abaixo é **bloqueante em CI**: falhou, não entra. Números marcados `[modelado]` são alvos propostos, ainda
não calibrados contra medição real — a primeira execução de cada um vira a linha de base e substitui o alvo.

| Portão                         | Instrumento                           | Limite                                                       | O invariante que ele guarda                                                         |
| ------------------------------ | ------------------------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| Formatação                     | `cargo fmt --all --check`             | Zero diferenças                                              | Revisão discute lógica, não estilo                                                  |
| Lint                           | `clippy --all-targets -D warnings`    | Zero avisos                                                  | Já é a regra de `lefthook.yml:13-15`; a CI impede burlar com `--no-verify`          |
| Memória                        | `#![forbid(unsafe_code)]` por crate   | Exceção só em `infrastructure/`, com comentário justificando | **N-02** (`PRD-001:97`) deixa de ser honra pessoal e vira erro de compilação        |
| Supply-chain                   | `cargo-deny check`                    | Zero CVEs conhecidas; licenças em allowlist                  | Um browser puxa árvore de dependências grande demais para confiar sem auditoria     |
| Cobertura de `domain/`         | `cargo-llvm-cov`                      | ≥ 85% `[modelado]`                                           | Domínio não tem I/O nem desculpa: invariante não coberto é invariante não garantido |
| Cobertura de `infrastructure/` | `cargo-llvm-cov`                      | ≥ 50% `[modelado]`                                           | Alvo menor é honesto: driver de GPU não se testa em CI                              |
| Overhead de hook               | `criterion`                           | p99 < 10μs (`PRD-001:96`)                                    | O NFR vira número aferido a cada PR, não promessa de ADR                            |
| Robustez de parser             | `cargo-fuzz` em HTML, CSS e JS        | Zero pânicos em 10 min por alvo `[modelado]`                 | Entrada da rede é hostil por definição; pânico em parser é DoS                      |
| Conformidade HTML              | Suíte html5lib do subconjunto         | 100% do subconjunto declarado na seção 2.1                   | Impede "quase parseia" virar dívida silenciosa                                      |
| Conformidade JS                | `test262`, por subconjunto            | Taxa registrada e **monotônica** entre releases              | Torna o limite do `boa_engine` visível e mensurável, e detecta regressão            |
| Renderização                   | Golden images no `SoftwareCpuBackend` | Diferença de pixel zero contra a referência                  | Prova que Vulkan e OpenGL não divergem do backend de referência — **I6**            |
| Isolamento de falha            | Injeção de pânico por subsistema      | Host sobrevive em 100% dos casos                             | **N-03** (`PRD-001:98`) só é aferível assim                                         |
| Domínio sem engine             | _Feature_ `no-engine` na CI           | Todo crate de domínio compila e testa sem runtime anexado    | **N-04** (`PRD-001:99`); é a prova viva de que o ADR-0002 foi respeitado            |
| Portabilidade                  | Matriz Linux · macOS · Windows        | Verde nos três                                               | Regressão de plataforma aparece no PR, não na véspera do release                    |

Duas métricas de acompanhamento, sem limite bloqueante, revisadas por release: **tempo de build limpo** por SO (um
browser degrada isso rápido e em silêncio) e **número de exceções ao `forbid(unsafe_code)`**, que só deve crescer com
justificativa em revisão.

Os portões não entram todos de uma vez — cada um passa a bloquear na versão em que o alvo que ele guarda passa a
existir. Ligar um portão antes disso só produz ruído verde; ligar depois deixa dívida entrar sem ser vista.

| Passa a bloquear em | Portões que entram em vigor                                                                      |
| ------------------- | ------------------------------------------------------------------------------------------------ |
| **v0.1**            | Formatação · lint · `forbid(unsafe_code)` · `cargo-deny` · matriz 3 SOs · cobertura de `domain/` |
| **v0.2**            | Domínio sem engine · isolamento de falha por injeção de pânico                                   |
| **v0.3**            | Golden images no backend de software · conformidade html5lib · fuzzing de HTML e CSS             |
| **v0.5**            | Overhead de hook por `criterion` · cobertura de `infrastructure/`                                |
| **v0.7**            | Taxa de `test262` monotônica · fuzzing do parser JS                                              |
| **v0.9**            | Golden images de Vulkan e OpenGL conferidas contra a referência de software                      |
| **v1.0**            | Todos acima, sem exceção aberta e sem teste marcado como `ignored`                               |

---

## 6. Verificação

Nada aqui foi executado. Nenhum item nasce marcado.

**Automatizável em CI, nos três SOs (`pnpm check` + `cargo test --workspace`):**

- [ ] `cargo fmt --all --check` passa — hoje **falha** por ausência de `rustfmt` na toolchain.
- [ ] Um crate de domínio compila e testa com a _feature_ `no-engine`, provando que **N-04** é real e não aspiracional.
- [ ] Chamar capability não concedida devolve `EngineError::PermissionDenied` (**C-07**) — o teste falha se alguém
      alargar um perfil de `PRD-003:55-58` por conveniência.
- [ ] Script Rhai em loop infinito aborta por limite de instruções (**C-04**), em vez de travar a suíte.
- [ ] Script que entra em pânico não derruba o processo e aciona o fallback (**C-09**).
- [ ] Edição com erro de sintaxe **não** substitui o AST em execução (**C-12**) — guarda a regra de rollback de
      `PRD-004:72`.
- [ ] DOM e estado de janela sobrevivem a dez hot-reloads seguidos (**C-13**).
- [ ] `DisplayListBuilder` sanitiza `NaN` e infinitos sem chegar ao backend (`PRD-005:80`).
- [ ] Taxa de `test262` do subconjunto ≥ a do release anterior — falha em regressão, nunca em número absoluto baixo.

**Só em headless sem GPU (runner de CI comum):**

- [ ] Sem driver Vulkan nem OpenGL, a inicialização cai em `SoftwareCpuBackend` (**C-17**) e a página ainda renderiza.
- [ ] Golden images de uma página estática batem pixel a pixel com a referência.

**Só em hardware com GPU real (não verificável em CI até existir runner com GPU):**

- [ ] `VulkanBackend` inicializa e desenha (**C-15**).
- [ ] Com Vulkan indisponível de propósito, a cascata cai em OpenGL (**C-16**) sem que layout perceba.
- [ ] Saída do Vulkan é indistinguível da golden image do backend de software (**I6**).

**Só com interação manual, nos três SOs:**

- [ ] Janela abre, redimensiona e fecha em Linux, macOS e Windows.
- [ ] Editar um `.rhai` com o browser aberto muda o comportamento sem piscar a janela e sem perder a página carregada.

---

## 7. Riscos

1. **`core/js` é o caminho crítico e o maior risco isolado da v1.0.** Mesmo embarcando `boa_engine`, o trabalho aberto
   são os bindings de DOM, o event loop, a fila de microtasks e a superfície de Web APIs — historicamente o item que
   domina o cronograma de qualquer browser. A F10 é a fase com maior variância de todo o roadmap, e o intervalo de 35–55
   d é otimista se a superfície de API crescer por demanda das páginas de teste.

2. **Parsing HTML5 é rotineiramente subestimado.** A especificação de tokenização e construção de árvore é longa, cheia
   de casos de recuperação de erro, e não admite "quase certo": páginas reais dependem de cada regra de _foster
   parenting_ e de fechamento implícito. F5 pode estourar sozinha o equivalente a uma fase inteira.

3. **Três backends gráficos antes de um funcionar bem.** Vulkan e OpenGL na F12 chegam depois do rasterizador de
   software, e a ordem protege o cronograma — mas se a F12 escorregar, a v1.0 sai com um backend só. Isso é aceitável e
   deve ser decidido conscientemente, não por acidente na véspera.

4. **Quatro trilhas com 2–4 devs significa mais trilhas do que gente.** I3 e I5 são os gargalos: se a API de `dom` não
   congelar em I3, a trilha D refaz bindings; se o event loop de I5 não for resolvido por um único dono, dois laços
   disputando a thread principal produzem _deadlock_ intermitente — a classe de bug mais cara de diagnosticar.

5. **O débito de processo SPDD já existe e cresce em silêncio.** `PRD-001:100` exige prompt SPDD para todo incremento
   funcional e `spdd/` não existe desde o commit inicial. Ou a F0 cria a pasta e passa a gerar os canvases, ou o
   ADR-0007 é revisado para descrever o que o time realmente faz. Manter uma regra escrita que ninguém segue corrói a
   autoridade de todos os outros ADRs.

6. **A especificação envelhece mais rápido que o código.** `docs/architecture/overview.md:80-92` já documenta
   dependências que não existem. Sem a disciplina de atualizar ADR e PRD dentro da fase que os contradiz — como a F7 faz
   com o PRD-003 —, a documentação vira ficção e o repositório perde seu ativo mais valioso.

---

## 8. Arquivos tocados

| Arquivo                               | Mudança                                                                                      |
| ------------------------------------- | -------------------------------------------------------------------------------------------- |
| `rust-toolchain.toml`                 | **novo** — toolchain fixada com `rustfmt` e `clippy`                                         |
| `.gitignore:4`                        | Remover `Cargo.lock` da lista                                                                |
| `Cargo.lock`                          | **novo** — versionado a partir da F0                                                         |
| `Cargo.toml`                          | `[workspace.package]` e `[workspace.dependencies]` centralizando versão e MSRV               |
| `.github/workflows/ci.yml`            | **novo** — matriz 3 SOs, portões da seção 5                                                  |
| `deny.toml`                           | **novo** — allowlist de licenças e auditoria de CVE                                          |
| `alloy/`                              | **novo crate binário** — ponto de entrada, event loop, composição dos subsistemas            |
| `core/*/src/lib.rs`                   | Substituir os 11 stubs pela fachada real; abrir `domain/`, `application/`, `infrastructure/` |
| `core/*/Cargo.toml`                   | Declarar as dependências que `overview.md:80-92` já documenta                                |
| `core/js/Cargo.toml`                  | `boa_engine` + porta de eventos implementada para `dom`                                      |
| `scripts/*.rhai`                      | **novo** — scripts padrão de UI, rede e pipeline, embarcados como fallback                   |
| `spdd/prompt/`, `spdd/analysis/`      | **novo** — exigido por `PRD-001:100`                                                         |
| `fuzz/`                               | **novo** — alvos de HTML, CSS e JS                                                           |
| `core/graphics/tests/golden/`         | **novo** — imagens de referência e fonte embarcada para determinismo                         |
| `docs/requirements/PRD-003-…`         | Perfil `WEB_CONTENT`, `Origin` e isolamento por aba — entregável de F7                       |
| `docs/adr/0011-…`                     | **novo** — escolha do motor JS e a exceção de Object Calisthenics no tokenizer               |
| `docs/architecture/overview.md:80-92` | Marcar a tabela como alvo até que as dependências existam de fato                            |
| `docs/README.md:10-27`                | Acrescentar `reports/` à árvore                                                              |

---

> Nenhum item deste relatório foi executado: não existe binário para executar. Toda a análise vem da leitura do código e
> da especificação no branch `main` (commit `0599cec`), das buscas com resultado zero reproduzidas na seção 1, e de
> `cargo test --workspace` e `cargo clippy` rodados nesta máquina. Os esforços em dias-dev são `[modelado]` — não há
> velocidade histórica deste time para calibrá-los, e a primeira fase concluída deve ser usada para recalibrar todas as
> seguintes.
