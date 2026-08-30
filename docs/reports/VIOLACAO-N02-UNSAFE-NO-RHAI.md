# Análise Técnica — N-02 (`PRD-001:97`) × o `unsafe` do `rhai`

**Data:** 2026-08-30 · **Branch:** `docs/n02-unsafe-audit-and-v0-3-v0-5-plans` · **Commit base:** `cd9631b` (`main`)
**Escopo:** o requisito não-funcional N-02 (`PRD-001:97`) contra a árvore de dependências que a PR #4 introduz
**Evidência de código coletada em:** `6536bbc` (`feat/v0-2-implementation`, PR #5) e no registry local do Cargo
**Relatórios irmãos:** [`IMPLEMENTACAO-DETALHADA-V0-5.md`](./IMPLEMENTACAO-DETALHADA-V0-5.md) §2.1

---

## 1. Sumário executivo

`PRD-001:97` declara como requisito não-funcional: _"**Memory Safety**: Zero unsafe memory operations exposed to script
runtimes."_ Em `main` o requisito é verdadeiro por vacuidade — não há motor de script na árvore. A **PR #4** o torna
falso ao introduzir `rhai`, e a **PR #5** constrói o chokepoint de capability exatamente sobre o caminho que contém o
`unsafe`.

O achado não é um defeito de memória. `transmute_copy` atrás de checagem de `TypeId` é o padrão de qualquer despacho
dinâmico em Rust, e o `rhai` não está fazendo nada anormal. O defeito é de **contrato**: um requisito escrito em termos
absolutos, aplicado com rigor a candidatos rejeitados, e nunca verificado contra o que foi aceito.

| #        | Problema                                                                            | Alcance                  | Severidade |
| -------- | ----------------------------------------------------------------------------------- | ------------------------ | ---------- |
| **S-01** | `rhai` executa `transmute_copy` no registro e no despacho de função nativa          | Todo binding guardado    | **Alta**   |
| **S-02** | N-02 nunca teve portão: `cargo-deny` audita CVE e licença, não blocos `unsafe`      | Todos os NFRs por tabela | **Alta**   |
| **S-03** | O critério de `unsafe` foi aplicado a candidatos rejeitados, nunca ao motor adotado | Decisões de dependência  | **Média**  |

**Veredicto:** o requisito está falso na forma escrita e não tem instrumento que o afira. A correção não é trocar o
`rhai` — é reescrever N-02 em termos de superfície de ameaça e ligar o portão que faltava, antes que a v0.5 traga uma
pilha TLS e a v0.7 traga um motor de JavaScript de conteúdo.

---

## 2. Metodologia e limites

### 2.1 O que foi feito

Leitura de `PRD-001`, `PRD-003`, `ADR-0002` e `ADR-0011`; inspeção do fonte do `rhai` 1.26.0 e do `bitflags` 2.13.1 no
registry local do Cargo; inspeção de `Cargo.toml` e `core/runtime/rhai/Cargo.toml` em `main` e em
`feat/v0-2-implementation`; e leitura de `core/runtime/rhai/src/infrastructure/sandbox.rs` na PR #5.

### 2.2 O que NÃO foi feito — limites desta análise

> **Nenhuma auditoria de dependência transitiva foi executada.** `cargo-geiger` não está instalado nesta máquina e não
> foi rodado. As quatro ocorrências de S-01 vêm de `grep` sobre o fonte do `rhai`; **as demais dependências transitivas
> da árvore não foram inspecionadas**, e a contagem real de `unsafe` no grafo completo é desconhecida. Este relatório
> estabelece que N-02 é falso, não o quanto é falso.

Também não foi feita análise de exploitabilidade. Nenhuma das ocorrências foi avaliada como vulnerabilidade; a
classificação de severidade abaixo mede violação de contrato e ausência de portão, não risco de execução de código.

### 2.3 O texto que serve de limiar

| Critério                             | Texto                                                                   | Fonte           |
| ------------------------------------ | ----------------------------------------------------------------------- | --------------- |
| N-02                                 | "Zero unsafe memory operations exposed to script runtimes"              | `PRD-001:97`    |
| Modelo de ameaça do script de muscle | Script _bugado_, em laço infinito, ou escalando privilégio — do usuário | `PRD-003:21-24` |
| Portão de memória previsto           | `#![forbid(unsafe_code)]` por crate, exceção só em `infrastructure/`    | `roadmap:333`   |
| Portão de supply-chain previsto      | `cargo-deny check` — CVE e licença                                      | `roadmap:334`   |

---

## 3. Evidências medidas

### 3.1 Em `main`, N-02 é verdadeiro por vacuidade

```bash
grep -n "rhai\|bitflags\|workspace.dependencies" Cargo.toml   # em main
# → 0 resultados
```

`core/runtime/rhai/Cargo.toml` em `main` declara uma única dependência, `engine = { path = "../../engine" }`. Não há
motor de script na árvore, logo não há `unsafe` exposto a um. **A violação é introduzida pela PR #4, não herdada.**

### 3.2 O `rhai` 1.26.0 tem quatro ocorrências, três na costura de binding

```bash
grep -rn "unsafe " --include="*.rs" ~/.cargo/registry/src/*/rhai-1.26.0/src
```

| Arquivo                | Linha | Operação                                                  |
| ---------------------- | ----- | --------------------------------------------------------- |
| `src/reify.rs`         | 19    | `transmute_copy` sobre `ManuallyDrop`                     |
| `src/reify.rs`         | 56    | `transmute_copy` sobre `ManuallyDrop`                     |
| `src/func/register.rs` | 60    | `transmute_copy::<_, T>` no **registro** de função nativa |
| `src/func/call.rs`     | 87    | `transmute_copy` no **despacho** da chamada               |

### 3.3 Verificação negativa: o `bitflags` está limpo

```bash
grep -rn "forbid(unsafe" ~/.cargo/registry/src/*/bitflags-2.13.1/src/lib.rs
# → src/lib.rs:273: #![cfg_attr(not(test), forbid(unsafe_code))]
```

As duas ocorrências de `unsafe impl` em `src/external.rs:236,243` são `Pod`/`Zeroable` do `bytemuck`, dentro de macro
atrás de feature opcional que o projeto não habilita (`Cargo.toml:36` fixa `bitflags = "=2.13.1"` sem features). **A
hipótese de que o `bitflags` também violasse N-02 está refutada e fica registrada para não voltar a ser levantada.**

### 3.4 Os crates do próprio projeto estão corretos

`#![forbid(unsafe_code)]` está na primeira linha dos onze crates. Nenhum achado deste relatório é sobre código escrito
pelo time — é sobre a árvore de dependências, que nenhum portão inspeciona.

---

## 4. Achados

### S-01 · `rhai` executa `transmute_copy` no registro e no despacho de função nativa — **Alta**

`~/.cargo/registry/src/*/rhai-1.26.0/src/func/register.rs:60` + `src/func/call.rs:87`

```rust
// rhai-1.26.0/src/func/register.rs:60
return unsafe { mem::transmute_copy::<_, T>(&ref_str) };

// rhai-1.26.0/src/func/call.rs:87
self.orig_mut = Some(mem::replace(&mut args[0], unsafe {
```

N-02 fala em `unsafe` _"exposed to script runtimes"_. `rhai::func::call` **é** o runtime de script: é o despacho que
executa toda chamada de função nativa a partir de um script. E `func/register.rs` é o caminho percorrido por
`RhaiContext::register_guarded_binding`, que a PR #5 estabelece como o **chokepoint único** de verificação de capability
(`core/runtime/rhai/src/infrastructure/sandbox.rs:3-10`). Não é uma dependência periférica: é exatamente a costura que o
requisito nomeia.

O mecanismo é conhecido e legítimo. O `rhai` guarda valores como `Dynamic`, confere o `TypeId` do alvo e então
`transmute_copy` para o tipo concreto — porque Rust não oferece _downcast_ seguro para tipos genéricos por valor. A
alternativa segura seria `Box<dyn Any>` + `downcast`, que aloca em cada chamada e destruiria o orçamento de `<10μs` por
hook (`PRD-001:96`).

| Alcance                   | Impacto                                                                                          |
| ------------------------- | ------------------------------------------------------------------------------------------------ |
| Todo binding nativo       | Alto para o **contrato**: o requisito que nomeia essa fronteira é falso exatamente nela          |
| Segurança de memória real | Baixo — `transmute_copy` atrás de checagem de `TypeId`; nenhuma exploração conhecida ou avaliada |

Trocar de motor não é a correção: `ADR-0002` escolheu o `rhai` deliberadamente, e qualquer interpretador embarcado com
despacho dinâmico terá a mesma construção. A correção é o requisito.

**Correção:** reescrever `PRD-001:97` em termos de superfície de ameaça — `unsafe` de terceiros proibido onde os bytes
são escolhidos pelo atacante (TLS, HTTP, HTML, CSS, imagem, fonte), permitido e **nominalmente enumerado** onde a
entrada é confiável (despacho de script de muscle, FFI de plataforma), proibido por conveniência (SIMD, alocação).
**Critério de aceite:** `PRD-001:97` não contém a palavra "zero"; e existe uma _allowlist_ versionada que nomeia `rhai`
com o comentário citando `PRD-003:21-24`.

---

### S-02 · N-02 nunca teve instrumento: `cargo-deny` não enxerga `unsafe` — **Alta**

`deny.toml:8-53` + `.github/workflows/ci.yml` (job `supply-chain`)

O `roadmap:333` atribui o portão de memória a `#![forbid(unsafe_code)]` por crate. Esse atributo governa **o código do
crate onde está escrito** e nada mais — ele não olha para dependências. O `roadmap:334` atribui o portão de supply-chain
ao `cargo-deny`, que audita CVE e licença, não blocos `unsafe`. O resultado é que N-02, ao contrário dos outros quatro
NFRs, **nunca teve nenhum instrumento**: a tabela de portões da seção 5 do roadmap não tem linha para ele.

Este é o achado transversal — S-01 e S-03 são consequências dele. Um requisito sem instrumento não é verificado por
disciplina; é verificado por sorte, e neste caso a sorte acabou na primeira dependência real.

**Correção:** job de CI `unsafe-audit` rodando `cargo-geiger` sobre o workspace, falhando em qualquer `unsafe` de crate
fora de uma _allowlist_ nominal e comentada, e bloqueante como os demais. **Critério de aceite:** adicionar ao
`Cargo.toml` uma dependência com `unsafe` fora da allowlist faz o job falhar; a mesma dependência com a linha
correspondente adicionada à allowlist faz o job passar.

---

### S-03 · O critério de `unsafe` foi aplicado a candidatos rejeitados, nunca ao motor adotado — **Média**

O critério existe e é usado — sempre para justificar uma rejeição, nunca para auditar uma adoção:

| Decisão                                   | Critério aplicado                                 | Onde                                   |
| ----------------------------------------- | ------------------------------------------------- | -------------------------------------- |
| `rquickjs` rejeitado como motor de JS     | _"`unsafe` na fronteira colide com N-02"_         | `roadmap:169`                          |
| `simd-adler32` evitado no codificador PNG | _"traz `unsafe`, que exigiria exceção ao portão"_ | `IMPLEMENTACAO-DETALHADA-V0-3.md` §2.7 |
| `rhai` adotado como motor de muscle       | **Nenhum**                                        | `ADR-0002`                             |

A assimetria é o problema. Um critério que só filtra candidatos e nunca revisa o incumbente não é um critério — é uma
racionalização disponível. E o custo já é concreto: no plano da v0.5, o provider de cripto padrão do `rustls` foi
rejeitado por trazer `unsafe`, aceitando um provider menos maduro para proteger a conexão, enquanto todo binding do
motor despacha por `transmute_copy`.

**Correção:** a decisão sobre o provider TLS da v0.5 é retomada **depois** de N-02 estar reescrito e o `unsafe-audit`
rodando, com os dois lados avaliados pelo mesmo critério. **Critério de aceite:** o relatório da v0.5 §2.5 registra a
decisão de provider citando a saída real do `unsafe-audit`, não a formulação antiga de N-02.

---

## 5. A consequência que vence na v0.7

A regra proposta em S-01 classifica o despacho do `rhai` como entrada confiável, e isso é defensável **enquanto** o
único motor for o de muscle: `PRD-003:21-24` modela o script de customização como bugado, não adversário — o autor é o
próprio usuário.

Essa classificação tem prazo. Na v0.7, `core/js` executa JavaScript de página: código de terceiros, potencialmente
hostil, a cada navegação. O motor de conteúdo cai na **primeira** linha da regra, não na segunda, e o critério de
`unsafe` para escolhê-lo é o estrito.

O `ADR-0011:128` reserva o **ADR-0012** exatamente para essa escolha. O ADR que reescrever N-02 precisa dizer isso de
forma explícita, para que o ADR-0012 nasça com o critério certo em vez de herdar a classificação do `rhai` por analogia
— que é como o precedente errado normalmente se propaga.

---

## 6. Backlog priorizado

> Ordenado por (dano ao contrato × alcance) ÷ esforço.

| Pri | Item                                                                     | Esforço | Alcance               | Tipo        |
| --- | ------------------------------------------------------------------------ | ------- | --------------------- | ----------- |
| 1   | **S-02** — job `unsafe-audit` com `cargo-geiger` + allowlist nominal     | S       | Todas as dependências | Qualidade   |
| 2   | **S-01** — reescrever `PRD-001:97` por superfície de ameaça              | XS      | Todo o projeto        | Arquitetura |
| 3   | ADR novo registrando a regra, com o prazo de validade da §5 escrito      | S       | v0.7 em diante        | Arquitetura |
| 4   | **S-03** — retomar a decisão de provider TLS da v0.5 sob o critério novo | XS      | v0.5                  | Correção    |
| 5   | Linha de N-02 na tabela de portões do `roadmap` §5, hoje ausente         | XS      | Governança            | Qualidade   |

Os itens 2, 4 e 5 somam menos de uma hora e são edição de documento. O item 1 é o único que exige código, e é o que
transforma os outros quatro de intenção em portão.

---

## 7. Riscos e o que falta verificar

1. **A contagem real de `unsafe` na árvore é desconhecida.** Só `rhai` e `bitflags` foram inspecionados. Rodar
   `cargo-geiger` sobre a árvore de hoje é o primeiro passo, e ele deve acontecer **antes** de a v0.5 fixar `winit`,
   `softbuffer`, `rustls` e o provider de cripto — caso contrário a allowlist nasce descrevendo uma árvore que ninguém
   mediu.

2. **`cargo-geiger` pode não rodar sob a toolchain fixada em 1.97.1.** É a dependência técnica do item 1 do backlog e
   não foi verificada. Se não rodar, a alternativa é um script que percorre `cargo metadata` e faz `grep` no fonte
   baixado — mais tosco, e suficiente para o portão.

3. **A severidade de S-01 é de contrato, não de memória.** Nenhuma das quatro ocorrências foi avaliada quanto a
   exploitabilidade, e este relatório não afirma que sejam exploráveis. Tratar S-01 como vulnerabilidade seria
   sobrerreagir; tratá-lo como irrelevante seria manter um requisito falso no documento que governa os outros quatro.

4. **A PR #4 pode ser mergeada antes da correção.** Isso é aceitável desde que seja decisão consciente: o comentário
   nesta PR existe para que o merge não signifique "ninguém viu". O que não é aceitável é a v0.5 rejeitar dependências
   por `unsafe` citando um requisito que o próprio motor já contradiz.

---

> Nenhum portão foi executado e `cargo-geiger` não foi rodado — ele não está instalado nesta máquina. Toda a análise vem
> da leitura de `PRD-001`, `PRD-003`, `ADR-0002`, `ADR-0011` e `deny.toml` no branch `main` (commit `cd9631b`), da
> inspeção de `core/runtime/rhai/src/infrastructure/sandbox.rs` e `Cargo.toml` no branch `feat/v0-2-implementation`
> (commit `6536bbc`, PR #5), e de `grep` sobre o fonte de `rhai-1.26.0` e `bitflags-2.13.1` no registry local do Cargo.
> A verificação da árvore transitiva completa está listada na seção 7 como pendente.
