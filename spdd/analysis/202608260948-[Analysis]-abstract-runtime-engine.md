# SPDD Analysis: Abstract Runtime Engine & Script Execution Interface

## Original Business Requirement

### PRD-002: Abstract Runtime Engine & Script Execution

- **Status**: Accepted
- **Author**: Core Architecture Team
- **Date**: 2026-08-22
- **Target Release**: v0.1.0-alpha

---

## 1. Executive Summary

Alloy requires an abstract execution layer that decouples the Rust domain data models from any specific scripting
interpreter. The primary launch backend is **Rhai** ([rhai.rs](https://rhai.rs/)), but the engine abstraction must
support future backends such as JavaScript (Boa/QuickJS) and WebAssembly without altering domain crates.

---

## 2. Problem Statement

Binding domain crates directly to a specific embedded scripting engine creates tight coupling:

- Type registrations and memory management become language-specific.
- Replacing or testing alternative script engines requires invasive refactors across all crates.
- Sandbox security rules cannot be uniformly enforced across different runtime engines.

---

## 3. Architecture & Trait Specifications

### 3.1 The `RuntimeEngine` Trait Hierarchy

The abstract engine layer resides in `core/engine` and provides:

```rust
pub trait RuntimeEngine: Send + Sync {
    type Context: ExecutionContext;
    type CompiledScript: Send + Sync;
    type Error: std::error::Error + Send + Sync + 'static;

    fn create_context(&self, capabilities: CapabilitySet) -> Result<Self::Context, Self::Error>;
    fn compile(&self, script_source: &str) -> Result<Self::CompiledScript, Self::Error>;
    fn eval<T: FromEngineValue>(&self, context: &mut Self::Context, script: &str) -> Result<T, Self::Error>;
}

pub trait ExecutionContext {
    type Error: std::error::Error + Send + Sync + 'static;

    fn register_type<T: 'static + CustomType>(&mut self) -> Result<(), Self::Error>;
    fn register_fn<F, Args, Ret>(&mut self, name: &str, f: F) -> Result<(), Self::Error>
    where
        F: EngineFunction<Args, Ret>;
    fn set_variable<V: IntoEngineValue>(&mut self, name: &str, value: V) -> Result<(), Self::Error>;
    fn call_function<Ret: FromEngineValue>(
        &mut self,
        name: &str,
        args: &[EngineValue],
    ) -> Result<Ret, Self::Error>;
    fn reset_scope(&mut self) -> Result<(), Self::Error>;
}
```

### 3.2 Rhai Engine Implementation (`core/runtime/rhai`)

The `RhaiEngine` implements `RuntimeEngine`:

- Wraps `rhai::Engine` and `rhai::Scope`.
- Registers domain types via `rhai::CustomType`.
- Enforces strict execution limits (instruction counter limits, recursion depth limits).
- Provides type marshaling between native Rust structs and `rhai::Dynamic`.

---

## 4. Requirements & Invariants

1. **Deterministic Type Conversions**: Domain types implementing `IntoEngineValue` and `FromEngineValue` must safely
   cross the runtime boundary without raw pointer dereferences.
2. **Zero Global State**: Runtime engines and execution contexts must be instantiable per subsystem and per isolate.
3. **Execution Limits**: All script executions must enforce maximum execution instruction steps (preventing infinite
   `while(true)` loops) and memory allocation ceilings.
4. **Transparent Error Mapping**: Script evaluation errors (syntax errors, runtime panics, type mismatches) must be
   mapped to structured Rust errors with line/column metadata.

---

## 5. Acceptance Criteria

- [ ] `RuntimeEngine` and `ExecutionContext` traits defined in `core/engine`.
- [ ] `RhaiEngine` implementation in `core/runtime/rhai` passing trait compliance tests.
- [ ] Registered Rust domain struct (`DomNode`) readable and mutable from Rhai script.
- [ ] Execution limit test: an infinite loop in Rhai is aborted with `EngineError::ExecutionLimitExceeded`.
- [ ] Trait-mocking test verifying engine can be replaced without modifying domain crates.

---

## Domain Concept Identification

### Existing Concepts (from codebase)

- `engine` (`core/engine`): Workspace crate designated by ADR-0002, ADR-0006 e ADR-0010 como a camada de abstração pura
  com zero dependências externas de interpretador.
- `alloy` (`alloy/`): Crate binário executável inicializado na F0 com CLI `clap`, pronto para orquestrar execução de
  scripts na v0.1.

### New Concepts Required

- `RuntimeEngine`: Trait mestre de execução que abstrai a compilação, instanciação de contexto sandbox e avaliação de
  scripts.
- `ExecutionContext`: Trait do ambiente de execução isolado que gerencia registro de tipos customizados, registro de
  funções nativas do host, variáveis e chamada de métodos.
- `CapabilitySet` & `Capability`: Conjunto de flags de autorização em nível de bit (`DOM_READ`, `DOM_MUTATE`,
  `NETWORK_FETCH`, etc.) exigidas na criação de cada contexto de execução (PRD-003).
- `EngineValue`: Enum canônico que representa valores dinâmicos do script (Null, Bool, Int, Float, String, Array, Map,
  Custom) sem vazar tipos específicos de um motor (como `rhai::Dynamic`).
- `IntoEngineValue` & `FromEngineValue`: Traits de conversão bidirecional segura entre estruturas de dados do Rust e
  `EngineValue`.
- `EngineError`: Enum tipado de erros do runtime (`ExecutionLimitExceeded`, `PermissionDenied`, `TypeMismatch`,
  `SyntaxError`, `RuntimeError`, `ScopeError`).
- `MockEngine` & `MockContext`: Implementação concreta em memória para testes unitários e de integração, provando
  desacoplamento de domínio (C-05).

### Key Business Rules

- **Pureza Estrutural (ADR-0002 & ADR-0010)**: `core/engine` não pode depender de interpretador algum (Rhai, Boa ou
  Wasm). Deve conter apenas definições de portas e tipos de valor de domínio.
- **Sandboxing por Princípio do Menor Privilégio (ADR-0004 & PRD-003)**: Todo `ExecutionContext` nasce com um
  `CapabilitySet` explícito. Funções nativas registradas devem ter acesso a essas permissões.
- **Sem Estado Global (PRD-002:77)**: Cada subsistema do navegador possui instâncias independentes de
  `ExecutionContext`.
- **Object Calisthenics Aplicado**: Newtypes para nomes de variáveis/funções (`Identifier`), erros fortemente tipados e
  zero mutabilidade pública descontrolada.

---

## Strategic Approach

### Solution Direction

- Implementar a arquitetura limpa em `core/engine`:
    - `src/domain/`: Entidades de valor imutáveis (`EngineValue`, `Capability`, `CapabilitySet`, `EngineError`,
      `Identifier`).
    - `src/application/`: Portas e interfaces de execução (`RuntimeEngine`, `ExecutionContext`, `FromEngineValue`,
      `IntoEngineValue`, `EngineFunction`).
    - `src/infrastructure/`: Adaptador de teste `MockEngine` e `MockContext` demonstrando substituição sem tocar no
      domínio.
    - `src/lib.rs`: Fachada pública re-exportando a API pública e garantindo `#![forbid(unsafe_code)]`.
- Habilitar `bitflags = { workspace = true }` em `core/engine/Cargo.toml` para modelagem segura de `CapabilitySet`.

### Key Design Decisions

- **`bitflags` para `CapabilitySet`**: Adotar o crate padrão da comunidade `bitflags` centralizado em
  `[workspace.dependencies]` para garantir operações binárias type-safe e sem `unsafe`.
- **Modelagem de `EngineFunction`**: Suportar registro de funções nativas através de trait de função padronizada com
  assinatura `Fn(&mut dyn ExecutionContext, &[EngineValue]) -> Result<EngineValue, EngineError>`, permitindo que
  closures do Rust atuem como callbacks do script.
- **`MockEngine` como primeira implementação**: Antes de implementar o backend do Rhai na F2, implementar um
  `MockEngine` completo dentro dos testes de `core/engine` para fechar C-01 e C-05 imediatamente com 100% de cobertura.

### Alternatives Considered

- _Implementar Rhai diretamente dentro de `core/engine`_: Rejeitado categoricamente pelo ADR-0002 para evitar
  acoplamento do domínio ao Rhai.
- _Usar `Box<dyn Any>` em vez de `EngineValue`_: Rejeitado porque impede serialização, inspeção em runtime e
  interoperabilidade transparente com futuros motores de script.

---

## Risk & Gap Analysis

### Requirement Ambiguities

- A assinatura de `register_fn` em `PRD-002:49-51` sugere generics `F, Args, Ret`. Em Rust puro sem macros de variádicos
  complexos, a abstração mais robusta e idiomática para trait de porta é receber um slice `&[EngineValue]` ou traits
  auxiliares de aridade controlada.

### Edge Cases

- Conversões de tipos numéricos (`i64` vs `f64`): Scripts podem passar inteiros onde floats são esperados; a conversão
  em `FromEngineValue` deve suportar coerção segura ou retornar `EngineError::TypeMismatch` estruturado.
- `reset_scope()`: Deve limpar variáveis e escopo léxico mantendo funções nativas registradas intactas.

### Technical Risks

- Overhead de conversão (N-01: `< 10μs`): `EngineValue` deve evitar alocações excessivas para tipos escalares
  (primitivos inline).
- Tratamento de panics em funções do host: Funções registradas não devem entrar em pânico no host; devem retornar
  `Result<EngineValue, EngineError>`.

### Acceptance Criteria Coverage

| AC#      | Descrição                                                              | Endereçável nesta Fase (F1)? | Notas                                           |
| :------- | :--------------------------------------------------------------------- | :--------------------------- | :---------------------------------------------- |
| **C-01** | Traits `RuntimeEngine` e `ExecutionContext` definidas em `core/engine` | Sim                          | Entregável central da Fase F1.                  |
| **C-02** | `RhaiEngine` passando testes de conformidade                           | Não (Fase F2)                | Escopo de `core/runtime/rhai` na F2.            |
| **C-03** | `DomNode` mutável por script                                           | Não (Fase F3/I1)             | Depende de `core/dom` (F3) e Rhai (F2).         |
| **C-04** | Aborto de loop infinito com `ExecutionLimitExceeded`                   | Não (Fase F2)                | Implementado no backend Rhai na F2.             |
| **C-05** | Teste com engine mockado provando troca sem tocar crates de domínio    | Sim                          | Implementado via `MockEngine` em `core/engine`. |
