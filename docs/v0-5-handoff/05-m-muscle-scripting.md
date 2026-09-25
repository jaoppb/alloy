# M — política no _muscle_ (`.rhai`)

## Contexto

Todas as fases anteriores deram ao motor (Skeleton) capacidade de rede, janela e layout — mas política de uso continua
hardcoded em Rust. M move essa política para scripts `.rhai`, seguindo ADR-0003 (Skeleton/Muscle): o motor oferece
capacidades cravadas em `CapabilitySet`, o script decide o quê e quando. Depende de **EE** (já entregue —
`SubsystemName` enumera os subsistemas reais), **B4**, **C1** e **C2** (todas prontas ou a caminho).

> **Só o item 4 (`css_bindings.rs`) e a parte de `cascade.rhai` do item 5 dependem de B4** — registram `StyledTree`, o
> agregado que B4 está mudando de forma agora. Os itens 1–3 (`css_cascade()`, `NETWORK_BINDINGS`, `WINDOW_BINDINGS`), o
> resto do item 5 (`default_ui.rhai`/`default_network.rhai`) e os itens 6–7 não tocam `core/css` e podem ser despachados
> agora, em paralelo com o fechamento de B4 — ver [`PARALELO-COM-B4.md`](PARALELO-COM-B4.md).

## Estado atual

```bash
grep -n "pub fn network_interceptor\|pub fn ui_window\|pub fn css_cascade" core/engine/src/domain/capability.rs
# → network_interceptor() e ui_window() já existem; css_cascade() ainda não (checar se falta)
grep -rn "NETWORK_BINDINGS\|WINDOW_BINDINGS" core/runtime/rhai-bindings/src/
# → 0 resultados: nenhuma tabela de binding de rede/janela existe ainda
```

`core/runtime/rhai-bindings/src/dom_bindings.rs` já é o molde a seguir — `NodeHandle` como
`EngineType`+`rhai::CustomType`, cada método se autoguardando por capability, erro mapeado para `EngineError::Subsystem`
(pós-EE).

## Passos

1. **Perfis de capability** — confirme que `engine::capability::profiles` tem `css_cascade()`
   (`DOM_READ | GRAPHICS_DRAW`); se faltar, adicione ao lado de `network_interceptor()`/`ui_window()` já existentes.

2. **`core/runtime/rhai-bindings/src/net_bindings.rs`** — `NETWORK_BINDINGS`: fetch, allow, deny, rewrite, header — cada
   operação exigindo `NETWORK_FETCH` (e `FS_WRITE_CACHE` quando grava cache), instalada via `install_guarded_table`
   (`sandbox.rs:40`).

3. **`core/runtime/rhai-bindings/src/window_bindings.rs`** — `WINDOW_BINDINGS`: repaint, title, route, atalho de teclado
   — exigindo `WINDOW_MANAGE`/`GRAPHICS_DRAW`/`DOM_READ` conforme a operação.

4. **`core/runtime/rhai-bindings/src/css_bindings.rs`** — registra `DomSnapshot`/`StyledTree` como `rhai` `CustomType`
   **somente leitura**; o adaptador `CascadeResolver` scriptável (`PRD-007` §3.4) mora aqui, com capability
   `DOM_READ | GRAPHICS_DRAW`, **nunca** `DOM_MUTATE`.

5. **`scripts/{default_ui.rhai, default_network.rhai, cascade.rhai}`** — `include_str!`; ciclo `on_init()` /
   `on_event(event)` / `on_process(state)`. Cada chamada passa pelo `run_with_fallback` generalizado que já existe (Fase
   0, R) — falha de script vira diagnóstico + adaptador Rust embutido, e a página continua renderizando (o mesmo
   contrato de fallback que `alloy --script` já demonstra desde a v0.2).

6. **Estender a matriz de injeção de pânico** — `rhai-bindings/tests/fault_injection.rs` já cobre
   `DOM_MUTATE`/`DOM_READ`; adicione `WINDOW_BINDINGS` e `NETWORK_BINDINGS` à mesma matriz (um pânico dentro de qualquer
   binding guardado precisa ser capturado, nunca derrubar o processo — C-09).

7. **`core/runtime/rhai/benches/hook_overhead.rs`** (`criterion`) — mede o _round-trip_ completo de `on_event`; commite
   a primeira execução como _baseline_. Meta do plano: p99 < 10 μs.

## Crates de referência

- `core/runtime/rhai-bindings/src/dom_bindings.rs` — o molde de _binding_ que se autoguarda por capability e mapeia erro
  de domínio para `EngineError::Subsystem`.
- `core/runtime/rhai-bindings/src/dom_fallback.rs` — o padrão de `run_with_fallback` a reusar para os scripts de
  rede/janela/cascata.

## Definition of Done

- [ ] Script de UI sem `NETWORK_FETCH` que tenta buscar → `EngineError::PermissionDenied`.
- [ ] Um adaptador de cascata `.rhai` muda uma propriedade computada e a _golden_ correspondente muda, com capability
      limitada a `DOM_READ | GRAPHICS_DRAW`.
- [ ] O mesmo adaptador em pânico cai no adaptador Rust embutido e a página ainda renderiza.
- [ ] Matriz de injeção de pânico cobre `WINDOW_BINDINGS` e `NETWORK_BINDINGS`, além do que já cobria antes.
- [ ] `cargo bench -p rhai-runtime --bench hook_overhead` roda e produz uma _baseline_ commitada.
- [ ] `cargo test --workspace` continua todo verde.

## Convenção de commit

```text
feat(rhai-bindings): NETWORK_BINDINGS, WINDOW_BINDINGS, scriptable cascade (v0.5 M)

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
```
