# Implementação da v0.1 — plano detalhado de F0 + F1 + F2

| Campo               | Valor                                                                                                                                     |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| **Status**          | ✅ Entregue (2026-08-30) — F0 + F1 + F2 implementados pelo **caminho sólido** (`ADR-0011`); ver emendas abaixo                            |
| **Cobertura**       | 4 de 4 critérios da v0.1: **C-01, C-02, C-04, C-05 fechados**. C-03 segue aberto para a v0.2 (I1)                                         |
| **Esforço**         | 28–42 dias-dev `[modelado]` no escopo completo (F0 6–9 · F1 10–15 · F2 12–18); 6–9 d `[modelado]` no mínimo viável                        |
| **Depende de**      | Nada — F0 pode começar já. F1 exige `Cargo.lock` versionado antes de `rhai` entrar                                                        |
| **Atenção**         | ⚠️ A decisão de projeto aceita `core/engine → rhai`, que contradiz `ADR-0002:49` e `overview.md:82` — os dois exigem emenda nesta entrega |
| **Fecha requisito** | C-01, C-04, C-05 integralmente · C-02 sobre fixture de teste · C-03 permanece aberto para a v0.2                                          |

---

> ## ⚠️ Emendas — v0.1 implementada pelo **caminho sólido** (`ADR-0011`), não pelo "verbatim"
>
> A primeira emenda (2026-08-29) cobre **F0 + F1**; a segunda (2026-08-30, mais abaixo) cobre **F2**. Ambas seguem o
> **contrato de portas do `ADR-0011`**, e **não** o caminho "seguir `PRD-002` verbatim + `core/engine → rhai`" descrito
> nas decisões §2.1 e §2.2 deste relatório. Aquelas duas decisões estão **substituídas**:
>
> | Decisão original (§2)                                       | O que foi feito                                                                                                                                                                                                                                                      |
> | ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | §2.1 "seguir `PRD-002` verbatim, mesmo brigando com o tipo" | Duas divergências documentadas: sem `type Error` associado (um único `EngineError`, `ADR-0011` item 4); `EngineType` no lugar de `rhai::CustomType`. Os genéricos de `PRD-002` (`eval::<T>`, `register_fn`, …) viram métodos _provided_ sobre um núcleo object-safe. |
> | §2.2 "`core/engine` passa a depender de `rhai`"             | **Não.** `core/engine` depende só de `bitflags`. `ADR-0002:49`, `overview.md:82` e o mapa de crates do `CLAUDE.md` continuam corretos e **não** foram enfraquecidos. Job de CI `no-engine` prova que nenhum interpretador entra no grafo.                            |
> | §2.7 fixture `FixtureNode` para C-02                        | Implementada em F2 (`core/runtime/rhai/tests/fixture_node.rs`): `FixtureNode` com `impl EngineType` + `impl rhai::CustomType`; script lê `node.tag` e muta `node.text`. C-03 (o `DomNode` real) segue aberto para a v0.2 (I1).                                       |
> | §2.8 crate binário `alloy`                                  | Implementado em F2. `cargo run -p alloy` abre e encerra (código 0); `alloy --script <path>` compila com `RhaiEngine`, roda sob o sandbox de limites e imprime o retorno. Parsing de argumentos na mão, zero dependências.                                            |
>
> ### Segunda emenda — 2026-08-30 · F2 implementada
>
> `core/runtime/rhai` está implementado sob o mesmo contrato do `ADR-0011`: `RhaiEngine` / `RhaiContext` /
> `RhaiCompiledScript(Arc<rhai::AST>)`, marshaling `EngineValue ⇄ rhai::Dynamic` por `match`, os três limites de
> `set_max_*` mais uma guarda `on_progress` de relógio de parede, `catch_unwind` → `ScriptPanic`, e a ponte `EngineType`
> → `rhai::CustomType` como extensão do adaptador. `rhai-runtime` é o **único** crate que nomeia um tipo `rhai`; o job
> de CI `no-engine` continua verde. Uma assinatura de F1 foi ajustada na integração:
> `ExecutionContext::register_native_fn` ganhou um parâmetro `Arity`.
>
> **Fechado agora:** C-02 (suíte `engine::conformance` verde contra `RhaiEngine` + `fixture_node`), C-04 (laço infinito
> abortado por limite de operações **e** por limite de tempo). **Mecanismo de C-09** presente (`ScriptPanic` com o host
> vivo); o handler de fallback com log em DevTools é F6/v0.2. **Aberto:** C-03 (v0.2 I1), C-06…C-13 (fases seguintes).
> `deny.toml` ganhou `MPL-2.0` (`smartstring`) e `CC0-1.0` (`tiny-keccak`). Artefatos SPDD:
> `spdd/{analysis,prompt}/…-rhai-runtime-v0-1-f2.md`. Com isso a **v0.1 inteira** ("O engine vive") está entregue.
>
> ### Terceira emenda — 2026-08-30 · endurecimento da base (contrato `ADR-0011` + verificação de portões)
>
> Pós-entrega, para deixar a base sólida antes de seguir:
>
> - **`ADR-0011` item 3**: `EngineValue` / `ValueKind` agora são `#[non_exhaustive]`; `engine::PORT_SCHEMA_VERSION` (=1)
>   é a única chave de versão do contrato de fronteira. `rhai-runtime` ganhou um braço `_` explícito no marshaling.
> - **`ADR-0011` item 1**: `PRD-002` ganhou §2.1 (variation model) e §2.2 (threat model — autor confiável-mas-falível,
>   **não** código hostil, que é `core/js`/`PRD-006`).
> - **`ADR-0011` item 5**: contrato de lifecycle/concorrência escrito em
>   `docs/architecture/runtime-engine-port-contract.md` (dono do estado durável, threading, re-entrância, cancelamento,
>   tetos, falha) — que também é o registro item-a-item dos 7 pontos do contrato.
> - **Lacuna de hook lifecycle registrada** (`PRD-002` §4.1 + doc de `call_function` em `ports.rs`): invocar função
>   **definida no script** (`on_reload` etc.) não é expressável na porta v0.1; decisão fica para a v0.2. 2 checks de
>   conformidade novos fixam o significado atual de `call_function` (= binding nativo registrado).
> - **Portões verificados nesta máquina** (`cargo-deny 0.20`, `cargo-llvm-cov`): `cargo deny check` →
>   `advisories ok, bans ok, licenses ok, sources ok`; cobertura de `engine` → **94,6% de linhas** (portão: 85%).
>   `deny.toml` reescrito para o schema 0.20 (`wildcards`, `allow-wildcard-paths`, `private.ignore`) e
>   `RUSTSEC-2026-0249` (`smartstring` unmaintained, transitivo via `rhai`, sem vuln) ignorado com justificativa. Os 9
>   manifestos-stub passaram a herdar `*.workspace = true`. Item 2 (companion `dyn RuntimeEngine`) segue como adiamento
>   explícito para a v0.2/ADR-0013.
>
> O restante deste documento (tabelas de esforço, armadilhas) permanece como registro histórico do plano, lido sob estas
> emendas.

---

Este relatório cobre **apenas a v0.1** do `ROADMAP-IMPLEMENTACAO-V1.md` — as fases **F0** (fundação), **F1**
(`core/engine`) e **F2** (`core/runtime/rhai`), que o roadmap §3.1 agrupa sob a versão "O engine vive". Nada aqui foi
implementado.

Decisões de escopo já tomadas com o solicitante, e assumidas ao longo do documento:

- O entregável é este **plano** — nenhum código é escrito nesta rodada.
- As assinaturas de `PRD-002:35-59` são seguidas **verbatim**, mesmo onde brigam com o sistema de tipos. As
  consequências estão na seção 2.
- `core/engine` **passa a depender de `rhai`** como consequência do item anterior; `ADR-0002`, o mapa de crates e
  `overview.md:82` são emendados, e um `ADR-0011` registra a escolha.
- O fluxo **SPDD** (`/spdd-analysis` → `/spdd-reasons-canvas`) é **prescrito** como trabalho de F0/F1, não executado
  aqui.
- A correção do `rustfmt` / toolchain é **prescrita** em F0, não executada aqui.

---

## 1. Estado atual — evidências

### O que já existe

O workspace declara 10 membros explícitos mais o glob `core/runtime/*` em `Cargo.toml:3-14`, com `resolver = "3"`
(`Cargo.toml:2`). O glob casa `core/runtime/rhai` e está reservado a futuros backends — o motivo está em
`ADR-0006:59-61`.

Os 11 crates têm conteúdo **idêntico** — verificado com `diff` de cada `src/lib.rs` contra `core/engine/src/lib.rs`,
todos sem diferença:

```rust
// core/engine/src/lib.rs:1-3 — idêntico nos 11 crates
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}
```

A única dependência entre crates hoje é `engine = { path = "../../engine" }` em `core/runtime/rhai/Cargo.toml:7`. Nenhum
outro `Cargo.toml` declara dependência alguma.

O portão de qualidade local **funciona** — medido nesta máquina, commit `cd9631b`:

- `cargo fmt --all --check` → exit 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → exit 0.
- `cargo test --workspace` → 22 linhas `test result: ok` (11 testes `it_works()` reais + 11 suítes de doc-test vazias).
- `rustc` e `cargo` 1.97.1; componente `rustfmt` **instalado** (`rustup component list --installed`).

`lefthook.yml:4-23` roda `cargo fmt`, `clippy -D warnings`, `prettier` e `markdownlint-cli2` no pre-commit;
`lefthook.yml:25-32` roda `cargo test --workspace` e `cargo check` no pre-push. `package.json:12` agrega o lado Rust +
Markdown em `pnpm check` — o lado Markdown não foi exercido aqui (exige `node_modules`).

A especificação está madura e serve de contrato: as traits em `PRD-002:35-59`, os 9 bitflags de `Capability` em
`PRD-003:37-46`, os critérios de aceite em `PRD-002:87-91` / `PRD-003:76-79`, os NFRs em `PRD-001:96-100`, e a estrutura
de camadas em `ADR-0010:54-74`.

### O que não existe

Cada ausência é uma busca com resultado zero:

```bash
find . -name "main.rs" -not -path "./target/*"; grep -rn "\[\[bin\]\]" --include=Cargo.toml .
# → 0 resultados  (não há binário; Alloy não é executável)
ls .github rust-toolchain.toml deny.toml .cargo spdd
# → 0 resultados  (sem CI, sem toolchain fixada, sem cargo-deny, sem SPDD)
grep -rn "forbid(unsafe_code)\|\[workspace.package\]\|\[workspace.dependencies\]\|rust-version" .
# → 0 resultados
```

Não há `rhai`, `bitflags` nem qualquer crate externo no grafo. Toda a v0.1 é, hoje, um contrato escrito.

### ⚠️ Divergências entre o roadmap e o estado atual

O `ROADMAP-IMPLEMENTACAO-V1.md` foi escrito no commit `0599cec`; o `HEAD` é `cd9631b`. Três pontos mudaram ou estavam
imprecisos:

1. **`rustfmt` já está instalado.** O roadmap §1 defeito 1 afirma que `cargo fmt` falha nesta máquina. Não falha mais —
   `cargo fmt --all --check` retorna exit 0. O `rust-toolchain.toml` continua ausente e ainda deve ser criado, mas o
   portão **não está quebrado** hoje.
2. **A contagem de membros do workspace.** O roadmap fala em "11 membros declarados explicitamente em
   `Cargo.toml:1-13`". São **10 explícitos + o glob `core/runtime/*`** (`Cargo.toml:11`), num arquivo de 15 linhas.
3. **`overview.md:82` já compromete `core/engine` com "None (Pure abstraction)".** A decisão de aceitar
   `core/engine → rhai` contradiz essa linha diretamente — ela terá de ser editada nesta entrega, não só o `ADR-0002`.

### Defeitos pré-existentes ainda no caminho da v0.1

Nenhum foi causado por este pedido — mas todos passam pelo código que F0/F1/F2 vão escrever, e corrigi-los agora custa
quase nada.

1. **`Cargo.lock` ignorado.** `.gitignore:4` remove o lockfile. É a convenção de biblioteca e o **oposto** do correto
   para uma aplicação com árvore de dependências grande. Precisa sair de `.gitignore` **antes** de `rhai` entrar em F1,
   senão a primeira dependência externa entra sem árvore congelada para `cargo-deny` auditar.
2. **`edition = "2024"` sem `rust-version`.** Os 11 manifestos exigem a edição 2024 e nenhum declara MSRV. Contribuidor
   com toolchain antiga recebe um erro de compilação obscuro.
3. **A tabela de dependências de `overview.md:80-92` é alvo, não estado.** Documenta `dom → engine`,
   `html → dom, engine` etc. — nenhuma delas existe em `Cargo.toml`. A distinção precisa ficar explícita no documento.
4. **`spdd/` não existe** apesar de `ADR-0007:39-44` e `PRD-001:100` a exigirem para todo incremento funcional.

---

## 2. As 9 decisões de design

### 2.1 Seguir `PRD-002:35-59` verbatim

As traits `RuntimeEngine` e `ExecutionContext` são transcritas como estão. Isso implica aceitar, sem correção:

- `eval<T: FromEngineValue>(&self, context: &mut Self::Context, script: &str)` (`PRD-002:42`) recebe **uma `&str`
  nova**, não o `CompiledScript` que `compile()` (`PRD-002:41`) devolve.
- `register_fn<F, Args, Ret>(...) where F: EngineFunction<Args, Ret>` (`PRD-002:49-51`) referencia um trait
  `EngineFunction` que **não existe** em nenhum arquivo nem PRD.
- `register_type<T: 'static + CustomType>` (`PRD-002:48`) exige um trait `CustomType` visível em `core/engine`.

O custo é transferido para F11 (hot-reload precisa de `Arc<AST>`, não de recompilar a cada `eval`) e para qualquer
consumidor que queira `dyn RuntimeEngine` — `eval<T>` genérico torna a trait **não object-safe**. Mitigações concretas
na seção 4.

### 2.2 `core/engine` passa a depender de `rhai`

`CustomType` é `rhai::CustomType`. Para `register_type<T: CustomType>` existir em `core/engine` **verbatim**, o crate
precisa nomear esse trait — e as regras de orphan impedem uma ponte `impl<T: engine::CustomType> rhai::CustomType for T`
a partir de `rhai-runtime`. Portanto:

| Opção                                       | O que é                                                           | Custo                                                                                      | Veredito                                                               |
| ------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| `core/engine` re-exporta `rhai::CustomType` | `core/engine/Cargo.toml` ganha `rhai`; `pub use rhai::CustomType` | `core/engine` deixa de ser abstração pura; `dom`/`css`/`html` puxam `rhai` transitivamente | **Escolhida** — decisão do solicitante; refinamento obrigatório abaixo |
| `core/engine` define `EngineType` próprio   | Marker trait local; `rhai-runtime` faz a ponte                    | Desvio pequeno do texto de `PRD-002:48`                                                    | Rejeitada nesta rodada (contradiz "verbatim")                          |

**Refinamento não-negociável:** `rhai` entra como dependência **opcional** de `core/engine`, atrás de
`default = ["rhai"]`. Os crates de domínio dependem de `engine` com `default-features = false`. Sem isso, o portão
"Domínio sem engine" (N-04, roadmap §5) cai já na v0.1 — ver seção 6, risco 1.

**Entregáveis de documentação desta decisão:** editar `ADR-0002:49` e `ADR-0002:40`, editar a linha
`core/engine … None (Pure abstraction)` em `overview.md:82`, ajustar o mapa de crates em `CLAUDE.md`, e abrir
`docs/adr/0011-*.md` (MADR) + linha em `docs/adr/README.md`.

### 2.3 `EngineError` compartilhado sob o `type Error` associado

`PRD-002:37` dá a cada engine um `type Error` próprio. Mas C-04 pede `EngineError::ExecutionLimitExceeded` e C-07 pede
`EngineError::PermissionDenied` — variantes de um enum **único**, e C-05 exige que os consumidores sejam genéricos sobre
qualquer engine. Resolução compatível com o texto: definir `EngineError` (enum) em `core/engine` e, em cada
implementação, `type Error = EngineError`. Variantes mínimas da v0.1:

`ExecutionLimitExceeded` · `PermissionDenied` · `Compilation { message, line, column }` ·
`TypeMismatch { expected, found }` · `Binding(String)` · `ScriptPanic(String)`.

### 2.4 `EngineValue` e as conversões

`EngineValue` é o enum de fronteira, sem tipos do `rhai` vazando: `Unit` · `Bool(bool)` · `Int(i64)` · `Float(f64)` ·
`Text(String)` · `Array(Vec<EngineValue>)` · `Map(BTreeMap<String, EngineValue>)`. `IntoEngineValue` / `FromEngineValue`
(`PRD-002:75`) são traits em `core/engine/application/`. `PRD-002:75` proíbe travessia por ponteiro cru — a conversão é
um `match` sobre variantes (permitido pela regra 2 de `ADR-0010:130`).

### 2.5 `Capability` e `CapabilitySet` — só o tipo entra na v0.1

`PRD-003:35-47` define `bitflags! struct Capability` com 9 flags. `PRD-002:40` recebe `CapabilitySet` em
`create_context`. Object Calisthenics (`ADR-0010:131`) proíbe primitivo nu no domínio — então
`CapabilitySet(Capability)` é o newtype que `create_context` guarda no contexto. A **verificação** por binding
(C-06/C-07) é F6/v0.2: na v0.1 o `CapabilitySet` é apenas carregado e armazenado, com `register_fn` deixando um `TODO`
no ponto de checagem.

### 2.6 Limite de execução via API do `rhai`

`Engine::set_max_operations`, `set_max_call_levels` e `set_max_expr_depths` (`PRD-002:68` cita "instruction counter" e
"recursion depth"). Quando o teto estoura, o `rhai` devolve `EvalAltResult::ErrorTooManyOperations`, que é mapeado para
`EngineError::ExecutionLimitExceeded` — é o que fecha **C-04**. Os tetos vêm de uma struct de config, não hardcoded no
corpo da função.

### 2.7 `DomNode` de C-02 é uma fixture de teste, não `core/dom`

`PRD-002:89` pede um struct de domínio (`DomNode`) legível e mutável por script. A v0.1 **não inclui F3** (`core/dom`) —
o roadmap §3.1 põe a `DomNode` real (C-03) na v0.2, ponto de integração I1. Para provar **C-02** ("trait compliance
tests"), F2 usa uma `FixtureNode { id, tag, text }` em `core/runtime/rhai/tests/`, com `impl CustomType`, e um teste que
lê e muta `node.text` via script. C-03 fica explicitamente **aberto** até a v0.2.

### 2.8 O crate binário `alloy`

| Opção                   | Custo                                                           | Veredito                                         |
| ----------------------- | --------------------------------------------------------------- | ------------------------------------------------ |
| `std::env::args` na mão | Zero dependências; parsing trivial para `alloy --script <path>` | **Escolhida para a v0.1**                        |
| `pico-args`             | 1 crate pequeno, sem derive                                     | Aceitável quando surgirem subcomandos (`render`) |
| `clap`                  | Árvore grande logo antes de `cargo-deny` entrar; derive macros  | Adiada — reavaliar na v0.3                       |

Na v0.1 o binário só precisa: `cargo run -p alloy` abre e encerra com código 0 nos 3 SOs, e `alloy --script hello.rhai`
compila o arquivo por `RhaiEngine`, executa com limite de instruções e imprime o retorno. Adicionar `"alloy"` a
`Cargo.toml` members **sem** regredir o glob para `core/*` (`ADR-0006:59-61`).

### 2.9 O que NÃO fazer na v0.1

- **Não** implementar verificação de capability nos bindings — é F6/v0.2 (C-06, C-07).
- **Não** implementar o watcher de hot-reload nem `eval_ast` completo — é F11/v0.9 (C-10 a C-13).
- **Não** criar `core/dom` real, nem tocar `core/html`, `core/css`, `core/graphics`.
- **Não** definir `panic = "abort"` em nenhum profile — quebraria o trapping de C-09 (v0.2).
- **Não** perseguir o NFR `<10μs` (`PRD-001:96`) — `criterion` só entra na v0.5 (roadmap §5).

---

## 3. Plano de implementação

| Fase   | Conteúdo                                                                                                             | Entregável verificável                                           | Esforço `[modelado]` |
| ------ | -------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | -------------------- |
| **F0** | Fundação: lockfile, toolchain, `[workspace.*]`, `forbid(unsafe_code)`, CI 3 SOs, `deny.toml`, crate `alloy`, `spdd/` | `pnpm check` verde nos 3 SOs · `cargo run -p alloy` abre e fecha | 6–9 d                |
| **F1** | `core/engine`: `EngineValue`, `EngineError`, `Capability`/`CapabilitySet`, traits verbatim, conversões, `MockEngine` | **C-01**, **C-05** com engine mockado                            | 10–15 d              |
| **F2** | `core/runtime/rhai`: `RhaiEngine`/`RhaiContext`, `compile`, `eval`, limites, marshaling, `catch_unwind`, fixture     | **C-02** (fixture), **C-04**                                     | 12–18 d              |

**F0 — passos (6–9 d `[modelado]`):**

1. **Lockfile e toolchain (1–1,5 d)** — remover `Cargo.lock` de `.gitignore:4` e versioná-lo; criar
   `rust-toolchain.toml` com canal fixo (`1.97.1`), `components = ["rustfmt", "clippy"]`, `profile = "minimal"`.
2. **Centralização no workspace (1 d)** — `[workspace.package]` (version, `edition = "2024"`, `rust-version`, license) e
   `[workspace.dependencies]` com `rhai` e `bitflags` **pinados exatos** e com features fixas (sem `only_i32`, sem
   `f32_float` — ver seção 4).
3. **`#![forbid(unsafe_code)]` (0,5 d)** — em cada `core/*/src/lib.rs`; exceção só em `infrastructure/`, com comentário
   justificando (roadmap §5, linha "Memória").
4. **CI (2–3 d)** — `.github/workflows/ci.yml`: matriz `ubuntu` · `macos` · `windows`; `pnpm check`;
   `cargo test --workspace`; `cargo deny check`; cobertura de `domain/` via `cargo-llvm-cov`. Portões que passam a
   bloquear na v0.1: formatação, lint, `forbid(unsafe_code)`, `cargo-deny`, matriz 3 SOs, cobertura de `domain/`
   (roadmap §5).
5. **`deny.toml` (0,5–1 d)** — allowlist de licenças cobrindo `rhai` e transitivas; `[advisories]`.
6. **Crate `alloy` (1 d)** — `alloy/Cargo.toml` + `alloy/src/main.rs` com `#![forbid(unsafe_code)]`; adicionar `"alloy"`
   a `Cargo.toml` members.
7. **SPDD (0,5 d + fluxo)** — criar `spdd/`; rodar `/spdd-analysis @docs/requirements/PRD-002-…` → `spdd/analysis/`,
   depois `/spdd-reasons-canvas` → `spdd/prompt/`, **antes** do primeiro `/spdd-generate` (`ADR-0007:39-44`).

**F1 — passos (10–15 d `[modelado]`), tudo em `core/engine/`:**

1. **`domain/` (3–4 d)** — `EngineValue` (seção 2.4); `EngineError` (seção 2.3); `Capability` (bitflags,
   `PRD-003:37-46`) + `CapabilitySet(Capability)` com métodos `contains` / `granted`.
2. **`application/ports.rs` (4–6 d)** — `RuntimeEngine` e `ExecutionContext` transcritos de `PRD-002:35-59`;
   `EngineFunction<Args, Ret>` (o trait auxiliar ausente — primeira versão só cobre aridade fixa); `IntoEngineValue` /
   `FromEngineValue`.
3. **Re-export (0,5 d)** — `pub use rhai::CustomType`, atrás da feature `rhai` (seção 2.2).
4. **`MockEngine` (2–3 d)** — `core/engine/tests/mock_engine.rs`: implementa `RuntimeEngine` sem linkar `rhai-runtime`,
   executa um "script" trivial e devolve `EngineValue`. Fecha **C-05** e sustenta o portão N-04 com
   `--no-default-features`.
5. **`lib.rs` fachada (0,5 d)** — `pub use` de `domain`/`application`.

**F2 — passos (12–18 d `[modelado]`), tudo em `core/runtime/rhai/`:**

1. **`infrastructure/` esqueleto (2–3 d)** — `RhaiEngine` embrulha `rhai::Engine`; `RhaiContext` embrulha
   `rhai::Scope` + `CapabilitySet` + handle do `Engine`.
2. **`compile` + `CompiledScript` (2 d)** — `rhai::Engine::compile` → `CompiledScript(Arc<rhai::AST>)` (já no formato
   que F11 vai precisar), mapeando erro de sintaxe para `EngineError::Compilation` com `line`/`column` (`PRD-002:81`).
3. **`eval` verbatim (2–3 d)** — recompila a cada chamada (custo aceito da decisão 2.1); adicionar um `eval_ast` **fora
   da trait** para F11 reusar depois.
4. **Limites (2–3 d)** — `set_max_operations` / `set_max_call_levels` / `set_max_expr_depths` a partir de config;
   `on_progress`; `ErrorTooManyOperations` → `EngineError::ExecutionLimitExceeded`. Fecha **C-04**.
5. **Marshaling (2–3 d)** — `EngineValue` ↔ `rhai::Dynamic` por `match`, sem `unsafe`; teste de round-trip preservando
   `i64`/`f64` de 64 bits.
6. **`register_fn` + `catch_unwind` (1,5–2 d)** — adaptador `EngineFunction` → `rhai::Engine::register_fn`; checagem de
   capability é `TODO` (F6); `panic::catch_unwind` (`AssertUnwindSafe`) em torno de `eval` → `EngineError::ScriptPanic`
   (mecanismo de C-09).
7. **Fixture C-02 (1–1,5 d)** — `FixtureNode` + `impl CustomType` em `tests/`; teste lê e muta `node.text` via script
   Rhai e confere o struct Rust.

**Mínimo viável** (só C-01 + C-05, sem binário, sem CI, sem `rhai`): parte de F1 ≈ **6–9 d `[modelado]`**. **Escopo
completo da v0.1:** F0 + F1 + F2 ≈ **28–42 dias-dev `[modelado]`** (roadmap §3.1).

Ordem: **F0 antes de F1** é obrigatório — o lockfile tem de ser versionado antes de `rhai` entrar. **F1 antes de F2** é
a direção da dependência (`rhai-runtime → engine`). Dentro de F1, o `MockEngine` (passo 4) pode ser escrito em paralelo
com `ports.rs`, e é o que prova que o `ADR-0002` foi respeitado.

---

## 4. Armadilhas

| Armadilha                                                                                                             | Mitigação                                                                                                                               |
| --------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `PRD-002:48` exige `CustomType` visível em `core/engine`, e orphan rules impedem a ponte a partir de `rhai-runtime`   | Decisão 2.2: `pub use rhai::CustomType` atrás da feature `rhai`; custo estrutural registrado na seção 6                                 |
| `PRD-002:42` — `eval<T>` recebe `&str` e ignora `compile()`; F11 (hot-reload) precisa de `Arc<AST>`                   | Implementar `eval` verbatim, mas guardar `CompiledScript(Arc<rhai::AST>)` já em F2 e expor um `eval_ast` fora da trait                  |
| `PRD-002:51` — trait `EngineFunction<Args, Ret>` referenciado e nunca definido                                        | Defini-lo em `core/engine/application/`; v0.1 cobre só closures de aridade fixa; variádico genérico fica para depois                    |
| `eval<T>` genérico na trait ⇒ `RuntimeEngine` não é object-safe (`Box<dyn RuntimeEngine>` impossível)                 | v0.1 usa monomorfização (`fn run<E: RuntimeEngine>`); se surgir necessidade de `dyn`, criar trait-objeto separado sem o método genérico |
| `rhai` **não-opcional** em `core/engine` ⇒ `dom`/`css`/`html` puxam `rhai` e o portão "Domínio sem engine" (N-04) cai | `rhai` como dep **opcional** atrás de `default = ["rhai"]`; crates de domínio usam `engine` com `default-features = false` — já em F1   |
| `cargo deny check` (F0) reprova a árvore de `rhai` por licença/aviso antes de a allowlist existir                     | Popular `deny.toml` com as licenças de `rhai` e transitivas **no mesmo PR** que adiciona `rhai` a `workspace.dependencies`              |
| `#![forbid(unsafe_code)]` + `catch_unwind` exige `AssertUnwindSafe`                                                   | É API segura — `AssertUnwindSafe` não é `unsafe`. Garantir que nenhum profile em `Cargo.toml` use `panic = "abort"`                     |
| `EngineValue::Int(i64)`/`Float(f64)` ≠ `rhai::INT`/`rhai::FLOAT` se `rhai` for compilado com `only_i32`/`f32_float`   | Fixar as features de `rhai` em `[workspace.dependencies]`; teste de round-trip de 64 bits (F2 passo 5)                                  |
| Glob `core/runtime/*` "simplificado" para `core/*` casa `core/runtime/` sem manifesto e aborta todo comando Cargo     | Manter membros explícitos (`ADR-0006:59-61`); não regredir ao adicionar `alloy/`                                                        |
| Fixture `DomNode` de C-02 confundida com a `DomNode` real de `core/dom` (F3/v0.2)                                     | Nomear `FixtureNode`, mantê-la em `rhai-runtime/tests/`, e documentar no PR que C-03 continua aberto                                    |
| `spdd/` inexistente enquanto `ADR-0007` e `PRD-001:100` a exigem                                                      | F0 passo 7 cria `spdd/` e roda o canvas para a própria v0.1 antes do primeiro `/spdd-generate`                                          |

---

## 5. Verificação

Nada aqui foi executado. Nenhum item nasce marcado.

**Automatizável em CI, nos 3 SOs (`pnpm check` + `cargo test --workspace`):**

- [ ] `cargo fmt --all --check` continua exit 0 com os módulos `domain/`/`application/`/ `infrastructure/` novos — hoje
      já passa (medido, `cd9631b`), ao contrário do roadmap §1.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0.
- [ ] `core/engine` compila e testa com `--no-default-features` (feature `no-engine` / `rhai` off), provando **N-04** —
      o teste falha se `rhai` virar dependência não-opcional.
- [ ] `MockEngine` executa um script e devolve `EngineValue` **sem** linkar `rhai-runtime` — guarda **C-05** (troca de
      engine sem tocar o consumidor).
- [ ] `RuntimeEngine` e `ExecutionContext` existem em `core/engine` com as assinaturas de `PRD-002:35-59` — guarda
      **C-01**; falha se alguém "melhorar" a assinatura sem ADR.
- [ ] Script `while true {}` via `RhaiEngine` aborta com `EngineError::ExecutionLimitExceeded` em < 100 ms `[modelado]`
      — guarda **C-04**, em vez de travar a suíte.
- [ ] `FixtureNode` mutada por script (`node.text = "x"`) reflete a mutação no struct Rust — guarda **C-02**.
- [ ] Erro de sintaxe em `compile()` retorna `EngineError::Compilation` com `line`/`column` preenchidos — guarda
      `PRD-002:81`.
- [ ] `panic!` dentro de função nativa registrada vira `EngineError::ScriptPanic` e **não** aborta o processo de teste —
      mecanismo de **C-09** (o teste completo é da v0.2).
- [ ] Round-trip `EngineValue → rhai::Dynamic → EngineValue` preserva `i64` e `f64` de 64 bits.

**Só local / pré-CI (F0, uma vez):**

- [ ] `Cargo.lock` versionado e `.gitignore` sem a linha `Cargo.lock`.
- [ ] `rust-toolchain.toml` fixa o canal e `components = ["rustfmt", "clippy"]`.
- [ ] `cargo deny check` exit 0 com `deny.toml` cobrindo a árvore de `rhai`.
- [ ] `cargo run -p alloy` abre e encerra com código 0 nos 3 SOs.
- [ ] `spdd/analysis/` e `spdd/prompt/` populados para a v0.1 antes do primeiro `/spdd-generate`.

**Não verificável nesta fase (declarado):**

- [ ] Overhead `<10μs` por hook (`PRD-001:96`, N-01) — sem `criterion` até a v0.5 (roadmap §5). Não medir aqui; apenas
      não introduzir cópia óbvia no caminho quente do `eval`.

---

## 6. Riscos

1. **`core/engine → rhai` colapsa a fronteira que justifica o crate.** `ADR-0002:49` promete "domain crates stay pure
   Rust data models with zero external script runtime dependencies"; `overview.md:82` diz "None (Pure abstraction)". Com
   `rhai` não-opcional, um futuro `core/runtime/js` compila `rhai` transitivamente e o portão "Domínio sem engine"
   (N-04) cai. A mitigação — `rhai` opcional + `default-features = false` nos consumidores — **tem de entrar em F1**,
   não depois.

2. **"Verbatim" congela assinaturas que a própria spec não fecha.** `EngineFunction` (`PRD-002:51`) não existe; `eval`
   ignora `compile` (`PRD-002:42`); `eval<T>` genérico mata object-safety. Seguir ao pé da letra empurra o custo para
   F11 (hot-reload) e para quem precisar de `dyn RuntimeEngine`. O roadmap §7 risco 6 já avisa: spec não corrigida na
   fase que a contradiz vira ficção.

3. **F2 é a fase de maior variância da v0.1.** O marshaling `EngineValue ↔ Dynamic`, os três limites de execução e o
   `catch_unwind` são detalhe fino de API do `rhai`; 12–18 d `[modelado]` é otimista se a fixture de C-02 exigir tipos
   compostos (array/map) no round-trip.

4. **F0 são seis a nove dias sem funcionalidade visível.** CI, `deny.toml`, toolchain, lockfile e o `alloy` vazio são
   pré-requisito de tudo e não fecham nenhum critério. Cortar F0 "para ganhar tempo" é o caminho mais curto para build
   não-reprodutível quando `rhai` entrar.

5. **SPDD continua débito de processo.** `spdd/` não existe (busca com resultado zero) apesar de `ADR-0007:39-44` e
   `PRD-001:100`. A decisão foi rodar o fluxo primeiro — se isso não acontecer antes do primeiro `/spdd-generate`, o
   débito passa a valer também para a v0.1, e corrói a autoridade dos outros ADRs.

---

## 7. Arquivos tocados

| Arquivo                                 | Mudança                                                                                                                |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `.gitignore:4`                          | Remover a linha `Cargo.lock`                                                                                           |
| `Cargo.lock`                            | **novo** — versionado a partir de F0                                                                                   |
| `rust-toolchain.toml`                   | **novo** — canal fixo + `components = ["rustfmt", "clippy"]`                                                           |
| `Cargo.toml`                            | `[workspace.package]` (MSRV, edition, license) + `[workspace.dependencies]` (`rhai`, `bitflags`); `"alloy"` em members |
| `.github/workflows/ci.yml`              | **novo** — matriz 3 SOs, portões da v0.1 (roadmap §5)                                                                  |
| `deny.toml`                             | **novo** — allowlist de licenças cobrindo `rhai`                                                                       |
| `alloy/Cargo.toml`, `alloy/src/main.rs` | **novo crate binário** — `--script`, `#![forbid(unsafe_code)]`                                                         |
| `core/engine/Cargo.toml`                | `rhai` (opcional, `default = ["rhai"]`) + `bitflags` de `workspace.dependencies`                                       |
| `core/engine/src/lib.rs`                | Substituir o stub `add()` pela fachada; abrir `domain/` e `application/`                                               |
| `core/engine/src/domain/`               | **novo** — `EngineValue`, `EngineError`, `Capability`, `CapabilitySet`                                                 |
| `core/engine/src/application/ports.rs`  | **novo** — traits `PRD-002:35-59` verbatim, `EngineFunction`, `Into/FromEngineValue`                                   |
| `core/engine/tests/mock_engine.rs`      | **novo** — `MockEngine` (fecha C-05)                                                                                   |
| `core/runtime/rhai/Cargo.toml`          | + `rhai` de `workspace.dependencies`                                                                                   |
| `core/runtime/rhai/src/lib.rs`          | Substituir o stub; abrir `infrastructure/`                                                                             |
| `core/runtime/rhai/src/infrastructure/` | **novo** — `RhaiEngine`, `RhaiContext`, `compile`, `eval`, limites, marshaling                                         |
| `core/runtime/rhai/tests/`              | **novo** — `FixtureNode` + testes de C-02 e C-04                                                                       |
| `spdd/analysis/`, `spdd/prompt/`        | **novo** — exigido por `ADR-0007` / `PRD-001:100`                                                                      |
| `docs/adr/0011-*.md`                    | **novo** — escolha de acoplar `core/engine` a `rhai`                                                                   |
| `docs/adr/README.md`                    | Linha para o ADR-0011                                                                                                  |
| `docs/adr/0002-*.md:40,49`              | Emenda: `core/engine` deixa de ser abstração pura                                                                      |
| `docs/architecture/overview.md:80-92`   | `core/engine` → dependência de `rhai`; marcar a tabela de deps como alvo                                               |
| `CLAUDE.md`                             | Ajustar o mapa de crates (linha de `core/engine`)                                                                      |
| `docs/README.md:21-22`                  | Acrescentar este relatório à árvore de `reports/`                                                                      |

---

> Nenhuma linha deste plano foi implementada — não há binário nem módulos `domain/` para exercitar. O que **foi**
> executado nesta máquina, no commit `cd9631b`: `cargo fmt --all --check` (exit 0),
> `cargo clippy --workspace --all-targets --all-features -- -D warnings` (exit 0), `cargo test --workspace` (22 linhas
> `test result: ok` = 11 testes reais) e as buscas de ausência da seção 1 (`.github`, `rust-toolchain.toml`,
> `deny.toml`, `.cargo`, `spdd/` — todas com resultado zero). Os esforços em dias-dev são `[modelado]`, reaproveitados
> de `ROADMAP-IMPLEMENTACAO-V1.md` §3.1, sem velocidade histórica para calibrá-los — a primeira fase concluída deve
> recalibrar as seguintes.
