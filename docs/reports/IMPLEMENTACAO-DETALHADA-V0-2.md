# Implementação da v0.2 — plano detalhado de F3 + F6 + I1

| Campo               | Valor                                                                                                                            |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **Status**          | 🟡 Não iniciado — a v0.1 (F0+F1+F2) **está concluída** (2026-08-30). Ver a nota de sincronização logo abaixo                     |
| **Cobertura**       | ~0% — 0 de 5 critérios da v0.2 (C-03, C-06, C-07, C-08, C-09) têm implementação                                                  |
| **Esforço**         | 26–40 dias-dev `[modelado]` (F3 12–18 · F6 12–18 · companion object-safe + I1 3–6); `ROADMAP-IMPLEMENTACAO-V1.md:226` orça 24–36 |
| **Depende de**      | v0.1 inteira. F6 exige `RhaiEngine`/`RhaiContext` de F2; I1 exige F2 **e** F3 (`ROADMAP-IMPLEMENTACAO-V1.md:290`)                |
| **Atenção**         | ⚠️ O ADR do companion object-safe é **0013**, não 0012 — `ADR-0011:108` já reserva 0012 para a escolha do motor JS (`boa`)       |
| **Fecha requisito** | C-06, C-07, C-08, C-09 integralmente · C-03 via ponto de integração I1                                                           |

Este relatório cobre **apenas a v0.2** do `ROADMAP-IMPLEMENTACAO-V1.md` — as fases **F3** (`core/dom`), **F6** (sandbox)
e o ponto de integração **I1** (`DomNode` scriptável), que o roadmap §3.1 agrupa sob a versão "DOM scriptável e contido"
(`ROADMAP-IMPLEMENTACAO-V1.md:226`). Nada aqui foi implementado.

Decisões de escopo já tomadas com o solicitante, e assumidas ao longo do documento:

- O entregável é este **plano** — nenhum código é escrito nesta rodada.
- A v0.1 conforme `IMPLEMENTACAO-DETALHADA-V0-1.md` está **pronta**: `core/engine` com `EngineValue`, `EngineError`
  (variantes `PermissionDenied`, `ExecutionLimitExceeded`, `Compilation`, `TypeMismatch`, `Binding`, `ScriptPanic`),
  `Capability`/`CapabilitySet`, as traits de `PRD-002:35-59` verbatim, `EngineFunction`, `Into/FromEngineValue` e
  `MockEngine`; `core/runtime/rhai` com `RhaiEngine`/`RhaiContext`, `compile → CompiledScript(Arc<rhai::AST>)`, `eval`,
  os três limites de execução, marshaling, `catch_unwind` → `ScriptPanic` e a `FixtureNode` de C-02; binário
  `alloy --script`; CI 3 SOs, lockfile versionado, `rust-toolchain.toml`, `deny.toml`, `spdd/`, `forbid(unsafe_code)`.
- O escopo é o **delta** da v0.2 sobre esse estado. O fluxo **SPDD** é prescrito para F3 e F6, não executado aqui.

> **Nota de sincronização — 2026-08-30.** A v0.1 foi implementada pelo **caminho sólido do `ADR-0011`**, não pelo
> "verbatim" (ver as emendas em `IMPLEMENTACAO-DETALHADA-V0-1.md`). Ajustes que a v0.2 já deve assumir:
>
> - `core/engine` **não depende de `rhai`**. `EngineType` (marker próprio) substitui `rhai::CustomType`; um único
>   `EngineError` (sem `type Error` associado). Os genéricos de `PRD-002` são métodos _provided_ sobre um núcleo
>   object-safe — `dyn ExecutionContext` já compila; o companion `dyn RuntimeEngine` (ADR-0013) continua sendo tarefa da
>   v0.2.
> - `EngineError` na v0.1 tem as variantes `Compilation`, `ExecutionLimitExceeded`, `PermissionDenied`, `TypeMismatch`,
>   `Conversion`, `Binding`, `ScriptRuntime`, `ScriptPanic`.
> - `ExecutionContext::register_native_fn` recebe `(name, Arity, NativeFn)`. A checagem de capability por binding (F6)
>   deve embrulhar esse ponto — `CapabilitySet::require` já existe em `core/engine`.
> - A prova de C-02 é `core/runtime/rhai/tests/fixture_node.rs` (`FixtureNode` + `impl rhai::CustomType`); o `DomNode`
>   real e o `NodeHandle` continuam sendo trabalho de I1.

---

## 1. Estado assumido e o que a v0.2 acrescenta

### O que a v0.1 já entregou (assumido)

O engine vive: `alloy --script hello.rhai` compila por `RhaiEngine`, executa com limite de instruções e imprime o
retorno; um laço infinito aborta com `EngineError::ExecutionLimitExceeded` (C-04); o `MockEngine` prova que o consumidor
é genérico sobre o engine (C-05). `core/dom` **ainda é o stub** `add()` — a v0.1 usou `FixtureNode` em
`core/runtime/rhai/tests/` e deixou C-03 explicitamente aberto (`IMPLEMENTACAO-DETALHADA-V0-1.md:176-181`).

### O que a v0.2 acrescenta, critério a critério

| #        | Critério (`PRD`)                                                       | Fase | Como fecha                                                                                     |
| -------- | ---------------------------------------------------------------------- | ---- | ---------------------------------------------------------------------------------------------- |
| **C-03** | `DomNode` legível e mutável por script Rhai (`PRD-002:89`)             | I1   | `NodeHandle` (`CustomType`) + global `document` em `rhai/infrastructure/dom_bindings.rs`       |
| **C-06** | Verificação de capability em **todo** binding nativo (`PRD-003:76`)    | F6   | Ponto único `register_guarded_binding`; tabela `Binding`; teste de conformidade varre a tabela |
| **C-07** | Capability negada → `EngineError::PermissionDenied` (`PRD-003:77`)     | F6   | A guarda testa `CapabilitySet::contains` antes de tocar o DOM; falha marshala para a variante  |
| **C-08** | `ExecutionContext` isolados, escopos separados (`PRD-003:78`)          | F6   | `create_context` já dá `rhai::Scope` novo; v0.2 acopla `CapabilitySet` + `DomTree` próprios    |
| **C-09** | Script em pânico não derruba o host e aciona o fallback (`PRD-003:79`) | F6   | `catch_unwind` (v0.1) → `ScriptPanic`; v0.2 adiciona o **fallback** e o log de diagnóstico     |

Micro-entregáveis da versão (`ROADMAP-IMPLEMENTACAO-V1.md:240-242`): um script Rhai constrói uma árvore DOM e a
serializa na saída; um script sem `DOM_MUTATE` recebe `PermissionDenied` ao escrever; um script que entra em pânico é
contido e o processo continua vivo, com o fallback assumindo.

### ⚠️ Divergência de documentação a corrigir nesta entrega

`overview.md:85` e `CLAUDE.md:48` declaram `core/dom` com dependência `engine`. A decisão 2.1 abaixo mantém `core/dom`
**sem dependência alguma** — a ligação DOM↔engine é um adaptador em `core/runtime/rhai`. As duas linhas são alvo, não
estado (`ROADMAP-IMPLEMENTACAO-V1.md:103-106`), e passam a ser corrigidas para `None`.

---

## 2. As decisões de design

### 2.1 `core/dom` é crate de domínio puro — zero dependências

`core/dom` é quase inteiramente `domain/`: entidades, value objects e invariantes, sem I/O. Segue `ADR-0010:72-73`
("`domain` depende de **nada**"). **Não** depende de `engine`, e portanto não puxa `rhai` transitivamente — o que mantém
o portão "Domínio sem engine" (N-04, `PRD-001:99`) verde por construção e evita que `core/html`, `core/css` e `core/js`,
todos consumidores de `dom` (`overview.md:84-87`), herdem o interpretador.

O registro de `DomNode` como tipo de engine (C-03) vive em `core/runtime/rhai/infrastructure/dom_bindings.rs` — é código
específico do `rhai`, e a conversão pela costura é uma função de mapeamento explícita, nunca um tipo de adaptador
re-exportado (`ADR-0011:70-72`). Consequência: `core/runtime/rhai/Cargo.toml` ganha `dom = { path = "../../dom" }`;
`core/dom` não ganha nada.

### 2.2 A árvore: arena com invariantes, `NodeId(u32)` literal

`DomTree` é o agregado: `Vec<Slot>` indexado por `NodeId(u32)` (`ADR-0010:131` e `CLAUDE.md:94` escrevem esse newtype ao
pé da letra). `Slot` é `Occupied(NodeData)` ou `Tombstone`. **Remoção deixa tombstone e não reutiliza o índice na v0.2**
— um `NodeId` obsoleto resolve para `Tombstone` → `DomError::NodeNotFound`, sem índice geracional; reuso + geração fica
para v0.9 (C-13). `NodeData` = `kind: NodeKind`, `parent: Option<NodeId>`, `children: Children`. `NodeKind` = `Document`
· `Element(ElementData)` · `Text(TextContent)` · `Comment(CommentContent)`.

Value objects e first-class collections (`ADR-0010:131-132`): `TagName` (valida não-vazio, ASCII, minúsculo na
construção), `TextContent`, `AttributeName`, `AttributeValue`; `Children(Vec<NodeId>)` e `AttributeMap` (ordem de
inserção preservada) — nunca coleção padrão pública.

Invariantes, garantidas só por métodos de `DomTree` (`CLAUDE.md:96` — sem campo público mutável): **(1)** aciclicidade —
`append_child` recusa se `child ∈ ancestors(parent)` → `WouldCycle`; **(2)** pai único — anexar nó com pai o desanexa
antes; **(3)** sem auto-pai → `SelfParent`; **(4)** `Document` é raiz única, não desanexável nem removível →
`CannotDetachDocument`; **(5)** todo `NodeId` em um `Children` resolve para `Occupied` com `parent` de volta.

### 2.3 Travessia sem recursão

`descendants(root)` e `ancestors(node)` são iteradores com pilha `Vec<NodeId>` explícita — **sem recursão**. Satisfaz
"um nível de indentação por função" (`ADR-0010:129`) e blinda contra estouro de pilha em árvore de profundidade hostil
(relevante quando `core/html` alimentar o DOM da rede, v0.3+). **Não** há exceção de Object Calisthenics para `core/dom`
— ao contrário do interior do tokenizer (`ROADMAP-IMPLEMENTACAO-V1.md:324`), as 9 regras valem inteiras.

### 2.4 `DomError` e o mapeamento pela costura

`core/dom/src/domain/error.rs` define **um** enum (`ADR-0011:73`): `NodeNotFound(NodeId)` · `WouldCycle` · `SelfParent`
· `CannotDetachDocument` · `InvalidTagName(String)` · `NotAnElement(NodeId)`. `core/dom` não conhece `EngineError`. No
adaptador, `dom_bindings.rs` mapeia `DomError` para uma variante nova
`EngineError::Dom { operation: String, reason: String }` — adição mínima a `core/engine/src/domain/error.rs`,
justificada porque `Binding(String)` (v0.1) perde a distinção entre "capability negada" e "operação de DOM inválida".

### 2.5 Superfície scriptável: `NodeHandle` + `document`

O script manipula um `NodeHandle { tree: Rc<RefCell<DomTree>>, id: NodeId }` — `Clone` barato. `impl CustomType` via
`build_type` (o `CustomType` re-exportado de `core/engine`, `IMPLEMENTACAO-DETALHADA-V0-1.md:239`). Superfície da v0.2:

| Membro no script                  | Capability exigida | Efeito                                    |
| --------------------------------- | ------------------ | ----------------------------------------- |
| `document` (global)               | `DOM_READ`         | `NodeHandle` do nó `Document`             |
| `node.tag`, `node.text` (getter)  | `DOM_READ`         | Lê `ElementData`/`TextContent`            |
| `node.children()`                 | `DOM_READ`         | `Array` de `NodeHandle`                   |
| `node.create_element(tag)`        | `DOM_MUTATE`       | Cria elemento solto, devolve `NodeHandle` |
| `node.append_child(child)`        | `DOM_MUTATE`       | Aplica invariantes 1–3 de 2.2             |
| `node.text = "…"` (setter)        | `DOM_MUTATE`       | Substitui `TextContent`                   |
| `node.set_attribute(name, value)` | `DOM_MUTATE`       | Insere/atualiza em `AttributeMap`         |

A mutação passa por `tree.try_borrow_mut()` — falha de empréstimo vira `EngineError::Dom { reason: "DOM ocupado" }` em
vez de `panic` de `RefCell`. `Rc<RefCell<_>>` é o idioma correto: `rhai` é single-threaded por `eval`, e
`ExecutionContext` (`PRD-002:45`) **não** exige `Send + Sync` — só `RuntimeEngine` (`PRD-002:35`), e `RhaiEngine`
continua `Send + Sync` por embrulhar apenas o `rhai::Engine` sem estado. O `RhaiContext` passa a `!Send` (2.7).

### 2.6 A guarda de capability — ponto único, resolvida na criação do contexto

C-06 exige verificação em **todo** binding. O mecanismo é um único ponto de estrangulamento —
`RhaiContext::register_guarded_binding(name, required: Capability, handler)` embrulha `handler` numa closure que faz
`self.capabilities.contains(required)?` antes de chamar. `register_fn` cru fica reservado a funções **puras**. Uma
tabela `&[Binding { name, required, handler }]` é percorrida uma vez ao montar o contexto, e o teste de conformidade
(§5) itera essa mesma tabela afirmando que nenhum binding de DOM escapa da guarda.

O `CapabilitySet` é capturado **por valor** (`bitflags` é `Copy`) em `create_context` — sem relookup por chamada. A
verificação é `and` de bits + branch, o que preserva o orçamento `<10μs` por hook (`PRD-001:96`,
`ROADMAP-IMPLEMENTACAO-V1.md:323`). Revogação em runtime não existe na v0.2: `ADR-0004` fixa as capabilities na criação
do contexto.

### 2.7 Contrato de ciclo de vida e concorrência (ADR-0011 item 5)

- **Estado durável**: o `DomTree` é do host Rust (Skeleton, `ADR-0003:41`); o `RhaiContext` guarda um clone do `Rc` e o
  host lê a árvore mutada após `eval` retornar.
- **Threading**: um `RhaiContext` por subsistema, preso a uma thread (`!Send`); sem DOM compartilhado entre threads.
- **Reentrância**: nenhum binding da v0.2 chama de volta para o script; `try_borrow_mut` é a rede de segurança.
  Reentrância real (callbacks, `document.write`) é problema de F5/F10.
- **Cancelamento / tetos**: herdados de F2. Script abortado por limite deixa o `DomTree` parcial — o fallback (2.8)
  parte de uma árvore **limpa**.

### 2.8 C-09: o fallback, não só o trapping

O trapping (`catch_unwind(AssertUnwindSafe(...))` → `EngineError::ScriptPanic`) já existe de F2
(`IMPLEMENTACAO-DETALHADA-V0-1.md:258-259`). A v0.2 fecha C-09 adicionando o **fallback** de `PRD-003:66-69`:

1. `eval` devolve `Err(ScriptPanic | ExecutionLimitExceeded | PermissionDenied | Compilation | Dom)`.
2. O host (na v0.2, o binário `alloy`) registra o diagnóstico — caminho do script, linha/coluna quando houver, variante
   — em `stderr`. O barramento de eventos do DevTools de `PRD-003:67` fica para depois (o crate `devtools` é stub).
3. O host executa `scripts/default_dom.rhai` embarcado, num **contexto guardado novo**, sobre um `DomTree` limpo.
4. Se o próprio fallback falhar, uma rotina Rust mínima monta `<html><body></body></html>` e o processo segue.
5. `alloy` encerra com **código 0**. `panic = "abort"` continua proibido em todos os profiles
   (`IMPLEMENTACAO-DETALHADA-V0-1.md:200`), senão o `catch_unwind` não recupera.

Um panic hook local, instalado só durante `eval` e removido logo após, captura a localização e evita poluir `stderr` com
o backtrace padrão.

### 2.9 ADR-0013 — companion object-safe de `RuntimeEngine`

`eval<T: FromEngineValue>` (`PRD-002:42`) e os `register_*<…>` de `ExecutionContext` (`PRD-002:48-53`) são métodos
genéricos → nenhuma das traits é object-safe → não existe `Box<dyn RuntimeEngine>`
(`IMPLEMENTACAO-DETALHADA-V0-1.md:279`). `ADR-0011:67-69` já **exige** "uma forma companion object-safe" para todo port.
A v0.2 é o primeiro código que chama um engine **fora** dos testes de `core/engine` (I1 liga o binário a um engine),
então é aqui que essa dívida herdada da v0.1 se paga, antes de virar retrofit.

ADR-0013 adiciona a `core/engine/src/application/` um par object-safe — todos os métodos `-> Result<_, EngineError>`:

```text
DynRuntimeEngine
    create_context_dyn(CapabilitySet) -> Box<dyn DynExecutionContext>
    eval_value(&mut dyn DynExecutionContext, &str) -> EngineValue
DynExecutionContext
    set_value(name, EngineValue)
    call(name, &[EngineValue]) -> EngineValue
    reset_scope()
```

- Sem método genérico na fronteira `dyn`; `register_type`/`register_fn` genéricos só na fiação concreta de
  `infrastructure/`.
- `eval::<T>` vira **função livre** em `dyn_bridge.rs`: `T::from_engine_value(engine.eval_value(ctx, src)?)`.
- Blanket impl `impl<E: RuntimeEngine<Error = EngineError>> DynRuntimeEngine for E` — `MockEngine` e `RhaiEngine` ganham
  o companion de graça, sem virar tipos novos.

Numeração: **0013**, não 0012 — `ADR-0011:108` reserva 0012 para a escolha do motor JS. O acoplamento
`core/engine → rhai` da v0.1 (que `IMPLEMENTACAO-DETALHADA-V0-1.md:22` planejou registrar como "ADR-0011", número já
tomado) deve ser dobrado no **retrofit de `PRD-002` sob `ADR-0011`** (`ADR-0011:100,106`), sem ADR autônomo.

### 2.10 O que NÃO fazer na v0.2

- **Não** implementar `Origin`, perfil `WEB_CONTENT` nem isolamento por aba — é F7/v0.7
  (`ROADMAP-IMPLEMENTACAO-V1.md:229,308-310`). C-08 na v0.2 é só escopo + capability + `DomTree` separados por contexto.
- **Não** tocar `core/html`, `core/css`, `core/graphics`, `core/window`, `core/js`.
- **Não** implementar o watcher de hot-reload nem `on_reload()` — é F11/v0.9.
- **Não** perseguir `<10μs` com `criterion` — o portão só entra na v0.5 (`ROADMAP-IMPLEMENTACAO-V1.md:374`); apenas não
  introduzir cópia óbvia no caminho da guarda.
- **Não** adicionar índice geracional ao `NodeId` — tombstone sem reuso basta para a v0.2 (2.2).

---

## 3. Plano de implementação

| Fase   | Conteúdo                                                                                                              | Entregável verificável                                 | Esforço `[modelado]` |
| ------ | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ | -------------------- |
| **F3** | `core/dom`: `NodeId`, arena, `Children`, `TagName`, `AttributeMap`, invariantes, travessia, serializador              | Testes de aciclicidade, pai único, ordem de travessia  | 12–18 d              |
| **F6** | Sandbox: `register_guarded_binding`, tabela `Binding`, contextos isolados, fallback, panic hook; ADR-0013 + companion | **C-06**, **C-07**, **C-08**, **C-09**                 | 12–18 d              |
| **I1** | `dom_bindings.rs`: `NodeHandle` (`CustomType`), global `document`, mapeamento `DomError → EngineError::Dom`           | **C-03** — script monta e muta o DOM, host lê de volta | 3–6 d                |

**F3 — passos (12–18 d `[modelado]`), tudo em `core/dom/`:**

1. **Value objects + `DomError` (2–3 d)** — `NodeId(u32)`; `TagName`/`AttributeName`/`AttributeValue`/`TextContent` com
   validação na construção.
2. **`Children` + `AttributeMap` (2–3 d)** — first-class collections; `insert_before` posicional; ordem preservada.
3. **`DomTree` arena + invariantes (4–6 d)** — `Slot`, `NodeData`; `create_document/element/text`;
   `append_child`/`detach`/`remove` aplicando as invariantes 1–5 de 2.2. Duas mutações de slot sequenciais por índice —
   sem `&mut` simultâneo, sem `unsafe`.
4. **Travessia (2–3 d)** — `Descendants` (pré-ordem, pilha explícita) e `Ancestors`; sem recursão.
5. **Serializador (2 d)** — `serialize_html(&DomTree, NodeId) -> String` puro e determinístico em
   `application/serialize.rs`; escape de `&<>` — é a "saída" do micro-entregável.
6. **Testes (1–2 d)** — aciclicidade recusada; anexar nó com pai o move; `detach` do `Document` recusado; ordem de
   `Descendants`; round-trip `build → serialize`.

**F6 — passos (12–18 d `[modelado]`), em `core/runtime/rhai/` e `core/engine/`:**

1. **ADR-0013 + companion (3–5 d)** — `DynRuntimeEngine`/`DynExecutionContext` em `application/ports.rs`; blanket impls
   e `eval::<T>` livre em `dyn_bridge.rs`; `MockEngine` exercita o caminho `dyn`. Emenda `PRD-002` e o índice de ADR.
2. **`sandbox.rs` (3–4 d)** — `register_guarded_binding`; struct `Binding`; guarda sobre o `CapabilitySet` capturado por
   valor; falha → `EngineError::PermissionDenied` (C-07). `RhaiContext` ganha `capabilities` + a tabela.
3. **`fallback.rs` (2–3 d)** — dado um `Err` de `eval`: `scripts/default_dom.rhai` em contexto novo; se falhar, rotina
   Rust mínima; diagnóstico em `stderr`; panic hook escopado. Fecha o **fallback** de C-09.
4. **Isolamento de contexto (1–2 d)** — `CapabilitySet` e `Rc<RefCell<DomTree>>` próprios por `create_context`; teste de
   C-08 (variável de A invisível em B; binding negado em B e permitido em A; panic em A não corrompe B).
5. **Matriz de injeção de pânico (2 d)** — para **cada** binding da tabela, `panic!` no handler e afirma: `eval` →
   `ScriptPanic`, processo vivo, fallback executado. É o portão que passa a bloquear na v0.2
   (`ROADMAP-IMPLEMENTACAO-V1.md:372`).
6. **Suíte de conformidade (1–2 d)** — `core/engine/tests/engine_conformance.rs` que `MockEngine` e `RhaiEngine` passam
   (`ADR-0011:79-82`), incluindo o caminho `dyn` e a varredura da tabela `Binding` (C-06).

**I1 — passos (3–6 d `[modelado]`), em `core/runtime/rhai/infrastructure/dom_bindings.rs`:** `NodeHandle` + `CustomType`
com getters sob `DOM_READ` e mutadores via `register_guarded_binding` sob `DOM_MUTATE`, `DomError → EngineError::Dom`
(2–3 d); `RhaiContext::bind_dom(Rc<RefCell<DomTree>>)` — método **concreto**, fora da trait — injeta o `document`, e
`alloy` usa o `RhaiEngine` concreto para a demo (1–2 d); `alloy --script build_dom.rhai` imprime `serialize_html`, em
erro faz fallback + exit 0 (0,5–1 d).

**Mínimo viável** (C-03 + C-07 só): F3 passos 1–3 + I1 ≈ **10–15 d `[modelado]`**. **Escopo completo:** F3 + F6 + I1 ≈
**26–40 dias-dev `[modelado]`**. Ordem: **F3 antes de I1**; **ADR-0013 antes do resto de F6**; F3 e F6 passo 1 correm em
paralelo (trilhas B e A).

---

## 4. Armadilhas

| Armadilha                                                                                                                | Mitigação                                                                                                                            |
| ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| `overview.md:85` / `CLAUDE.md:48` dizem `core/dom → engine`; segui-los puxa `rhai` para `html`/`css`/`js` e derruba N-04 | Decisão 2.1: `core/dom` sem dependências; a ligação é adaptador em `rhai-runtime`; corrigir as duas linhas para `None` nesta entrega |
| Checagem de capability "em todo binding" (C-06) é fácil de esquecer num `register_fn` novo                               | Ponto único `register_guarded_binding` + tabela `Binding`; teste de conformidade varre a tabela e falha se um binding de DOM escapar |
| Guarda por chamada versus `<10μs` por hook (`PRD-001:96`)                                                                | `CapabilitySet` capturado por valor (`Copy`) na criação do contexto; a guarda é `and` de bits + branch, sem relookup                 |
| `eval<T>` genérico (`PRD-002:42`) impede `Box<dyn RuntimeEngine>` que I1 quer                                            | ADR-0013: companion object-safe `DynRuntimeEngine`/`DynExecutionContext`; `eval::<T>` vira função livre                              |
| `Rc<RefCell<DomTree>>` torna `RhaiContext` `!Send`, e `RuntimeEngine: Send + Sync` (`PRD-002:35`)                        | Só `RuntimeEngine` exige `Send + Sync`; `ExecutionContext` (`PRD-002:45`) não. `RhaiEngine` fica `Send + Sync`; o contexto não       |
| Getter e setter tocando `RefCell` no mesmo passo → panic de empréstimo                                                   | `try_borrow_mut`/`try_borrow`; falha → `EngineError::Dom { reason: "DOM ocupado" }`. Nenhum binding da v0.2 chama de volta ao script |
| Script abortado por limite deixa o `DomTree` meio-montado                                                                | O fallback parte de um `DomTree` **limpo**, nunca do parcial; documentado no contrato 2.7                                            |
| Object Calisthenics regra 1 (`ADR-0010:129`) em travessia de árvore convida à recursão                                   | Iteradores com pilha `Vec<NodeId>` explícita; **sem** exceção de calisthenics para `core/dom` (diferente do tokenizer)               |
| `NodeId(u32)` obsoleto após `remove` aponta para lixo                                                                    | Remoção deixa `Tombstone`, índice **não** reutilizado na v0.2 → `DomError::NodeNotFound`; geração fica para v0.9                     |
| `panic = "abort"` reintroduzido num profile quebra o `catch_unwind` de C-09                                              | Guarda de CI: teste que injeta `panic!` num binding e espera o processo vivo — falha imediatamente se algum profile usar `abort`     |
| ADR numerado 0012 colide com a reserva de `ADR-0011:108` (motor JS)                                                      | Usar **0013** para o companion; dobrar o acoplamento `engine → rhai` no retrofit de `PRD-002` sob `ADR-0011`, sem ADR novo           |
| `spdd/` sem canvas para F3/F6 enquanto `PRD-001:100` os exige                                                            | Passo SPDD prescrito: `/spdd-analysis` sobre `PRD-002`/`PRD-003` → `/spdd-reasons-canvas` antes do primeiro `/spdd-generate` de F3   |

---

## 5. Verificação

Nada aqui foi executado. Nenhum item nasce marcado.

**Automatizável em CI, nos 3 SOs (`pnpm check` + `cargo test --workspace`):**

- [ ] `cargo test -p dom` verde com os módulos `domain/`/`application/` novos; `cargo fmt --all --check` e
      `clippy -D warnings` continuam exit 0.
- [ ] `cargo test -p dom --no-default-features` compila e passa — `core/dom` **não** linka engine algum (portão "Domínio
      sem engine", `PRD-001:99`, que passa a bloquear na v0.2).
- [ ] `append_child` que criaria ciclo devolve `DomError::WouldCycle` e **não** altera a árvore.
- [ ] Anexar um nó que já tem pai o remove do `Children` anterior (invariante de pai único).
- [ ] `detach`/`remove` do nó `Document` devolve `DomError::CannotDetachDocument`.
- [ ] `Descendants` visita em pré-ordem determinística; `Ancestors` termina na raiz.
- [ ] `build → serialize_html → String` bate com o texto esperado, atributos em ordem de inserção, `&<>` escapados.
- [ ] Script Rhai com `DOM_READ | DOM_MUTATE` monta `document`, cria elementos, seta `text`/atributos; o `DomTree` Rust
      reflete a árvore após `eval` — guarda **C-03**.
- [ ] Script com `CapabilitySet` sem `DOM_MUTATE` recebe `EngineError::PermissionDenied` ao chamar `append_child` ou
      `node.text = …`; getters ainda funcionam — guarda **C-06**/**C-07**.
- [ ] Dois contextos do mesmo `RhaiEngine`: variável setada em A não aparece em B; binding negado em B e permitido em A;
      `panic` em A não afeta o `eval` seguinte de B — guarda **C-08**.
- [ ] Para **cada** binding da tabela `Binding`, injetar `panic!` no handler: `eval` devolve `EngineError::ScriptPanic`,
      o processo de teste continua, o fallback roda — guarda **C-09** e o portão "isolamento de falha por injeção de
      pânico".
- [ ] `engine_conformance.rs` passa para `MockEngine` **e** `RhaiEngine`, incluindo o caminho
      `Box<dyn     DynRuntimeEngine>` (ADR-0013) e a varredura da tabela de bindings.
- [ ] `DomError` → `EngineError::Dom { operation, reason }` no adaptador; `core/dom` nunca nomeia `EngineError`.

**Só local / manual:**

- [ ] `alloy --script build_dom.rhai` imprime o HTML serializado; `alloy --script panics.rhai` imprime diagnóstico em
      `stderr`, roda o fallback e encerra com código 0.
- [ ] `spdd/analysis/` e `spdd/prompt/` populados para F3 e F6 antes do primeiro `/spdd-generate`.

**Não verificável nesta fase (declarado):**

- [ ] Overhead `<10μs` por binding guardado (`PRD-001:96`, N-01) — sem `criterion` até a v0.5. Não medir aqui; apenas
      manter a guarda como `and` de bits sem alocação.
- [ ] Isolamento por `Origin`/aba (`PRD-003` §2 cenário 4) — é F7/v0.7; a v0.2 não o toca.

---

## 6. Riscos

1. **A superfície de `NodeHandle` cresce por demanda dos scripts de teste.** A tabela de 2.5 é o mínimo para o
   micro-entregável. `insertBefore`, `removeChild`, clonagem, consulta por seletor — cada um é um binding guardado a
   mais. O intervalo de 3–6 d para I1 é otimista se a demo exigir manipulação não-trivial da árvore.

2. **`DomTree` com Object Calisthenics integral é mais verboso do que parece.** Aciclicidade, pai único e coerência de
   `Children` sem `else` e com um nível de indentação viram muitos métodos privados curtos. F3 pode encostar nos 18 d se
   os testes de invariante exigirem cenários compostos (mover subárvore, remover nó com filhos).

3. **ADR-0013 mexe numa assinatura de port já "congelada" em F1.** `ADR-0011:83-85` diz que mudar a superfície pública
   depois do freeze exige bump de versão de schema e nota de migração. O companion é aditivo (não quebra a trait
   genérica), mas a formalidade tem de ser cumprida, ou o próprio contrato de ports perde autoridade —
   `ROADMAP-IMPLEMENTACAO-V1.md:448-451`.

4. **O fallback de C-09 depende de um script embarcado que também pode falhar.** `scripts/default_dom.rhai` precisa ser
   trivial e testado como qualquer código. A rotina Rust de último recurso (passo F6.3) não é opcional — sem ela, um
   erro no fallback reabre o buraco que C-09 fecha.

5. **A correção de `overview.md:85` / `CLAUDE.md:48` é parte do escopo.** Se `core/dom` sem dependência não for
   registrado nesses dois documentos **nesta** entrega, o próximo a ler o mapa implementa `dom → engine` e reintroduz o
   acoplamento transitivo com `rhai`.

6. **SPDD continua débito de processo.** `PRD-001:100` exige prompt SPDD para todo incremento funcional; F3 e F6 são
   incrementos funcionais. Rodar o fluxo antes do primeiro `/spdd-generate`, ou revisar `ADR-0007` — ignorar as duas
   saídas não é opção honesta.

---

## 7. Arquivos tocados

| Arquivo                                                                                             | Mudança                                                                                                                               |
| --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `core/dom/Cargo.toml`                                                                               | Permanece **sem dependências** (domínio puro)                                                                                         |
| `core/dom/src/lib.rs`                                                                               | Substituir o stub pela fachada; `#![forbid(unsafe_code)]`; `pub use domain::{…}`                                                      |
| `core/dom/src/domain/` (`node`, `tag_name`, `children`, `attributes`, `tree`, `error`, `traversal`) | **novo** — value objects, `Children`/`AttributeMap`, `DomTree` (arena + invariantes 1–5 de §2.2), `DomError`, iteradores sem recursão |
| `core/dom/src/application/serialize.rs`                                                             | **novo** — `serialize_html(&DomTree, NodeId) -> String` determinístico                                                                |
| `core/dom/tests/`                                                                                   | **novo** — aciclicidade, pai único, detach do `Document`, ordem de travessia, round-trip                                              |
| `core/engine/src/domain/error.rs`                                                                   | Adicionar variante `EngineError::Dom { operation, reason }`                                                                           |
| `core/engine/src/application/` (`ports.rs`, `dyn_bridge.rs`)                                        | `DynRuntimeEngine`/`DynExecutionContext` + blanket impls + `eval::<T>` livre (ADR-0013)                                               |
| `core/engine/tests/` (`engine_conformance.rs`, `mock_engine.rs`)                                    | **novo/estendido** — suíte de conformidade + troca via `Box<dyn DynRuntimeEngine>`                                                    |
| `core/runtime/rhai/Cargo.toml`                                                                      | + `dom = { path = "../../dom" }`                                                                                                      |
| `core/runtime/rhai/src/infrastructure/` (`dom_bindings.rs`, `sandbox.rs`, `fallback.rs`)            | **novo** — `NodeHandle`/`document`, `register_guarded_binding`+`Binding`, fallback + panic hook                                       |
| `core/runtime/rhai/tests/`                                                                          | **novo** — C-03, C-06/C-07, C-08, C-09, matriz de injeção de pânico                                                                   |
| `alloy/src/main.rs`                                                                                 | Demo: montar DOM em contexto guardado, imprimir `serialize_html`; em erro → fallback + exit 0                                         |
| `scripts/default_dom.rhai`, `scripts/hello_dom.rhai`                                                | **novo** — fallback embarcado + exemplo do micro-entregável                                                                           |
| `docs/adr/0013-object-safe-runtime-engine-companion.md`, `docs/adr/README.md`                       | **novo** — MADR do companion + linha no índice                                                                                        |
| `docs/requirements/PRD-002-abstract-runtime-engine.md`                                              | Emenda: forma companion object-safe (`ADR-0011:67-69`)                                                                                |
| `docs/adr/0011-…`                                                                                   | Nota: acoplamento `engine → rhai` é o retrofit de `PRD-002` sob este contrato, sem ADR próprio                                        |
| `docs/architecture/overview.md:85`, `CLAUDE.md:48`                                                  | `core/dom` → `Dependencies: None` no mapa de crates                                                                                   |
| `spdd/analysis/`, `spdd/prompt/`                                                                    | **novo** — canvases de F3 e F6 (`PRD-001:100`)                                                                                        |
| `docs/reports/IMPLEMENTACAO-DETALHADA-V0-2.md`, `docs/README.md:26`                                 | **novo** — este relatório + linha na árvore de `reports/`                                                                             |

---

> Nenhuma linha deste plano foi implementada. O que **foi** feito nesta rodada: leitura de
> `ROADMAP-IMPLEMENTACAO-V1.md`, `IMPLEMENTACAO-DETALHADA-V0-1.md`, `PRD-001`/`PRD-002`/`PRD-003`/`PRD-004`,
> `ADR-0003`/`ADR-0004`/`ADR-0005`/`ADR-0010`/`ADR-0011`, `docs/architecture/overview.md` e do estado do workspace no
> branch `main` (commit `cd9631b` — os 11 crates ainda são o stub `add()`). Todas as referências `arquivo:linha` foram
> conferidas contra esses arquivos. **Não verificado**: que a v0.1 realmente entrega o que
> `IMPLEMENTACAO-DETALHADA-V0-1.md` planeja — este relatório assume esse plano como executado, conforme pedido; se a
> v0.1 desviar (por exemplo, `EngineError` sem a variante `Binding`, ou `CompiledScript` sem `Arc`), os passos de F6 e
> I1 mudam. Os esforços em dias-dev são `[modelado]`, reaproveitados de `ROADMAP-IMPLEMENTACAO-V1.md:226,272,275` sem
> velocidade histórica para calibrá-los.
