# Implementação da v0.5 — plano detalhado de F8 + F9 · I4

| Campo               | Valor                                                                                                                                |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| **Status**          | ❌ Não iniciado — plano. `core/css`, `core/window` e `core/network` são o stub `add()` de 16 linhas                                  |
| **Cobertura**       | Fecha **0** dos 18 critérios numerados — e é a versão que mais fecha requisito: `PRD-007` integral, `PRD-003` §3.2 pela primeira vez |
| **Esforço**         | 84–126 dias-dev `[modelado]`. `ROADMAP-IMPLEMENTACAO-V1.md:218` orça 50–75 — a diferença é escopo escolhido, aberta em §1.3          |
| **Depende de**      | v0.1 + v0.2 + **v0.3 inteira**. I4 exige F8 **e** I2 (`ROADMAP-IMPLEMENTACAO-V1.md:281`)                                             |
| **Atenção**         | ⚠️ A v0.5 escreve **dois** ports novos (`core/network`, `core/window`) e é a primeira a executar política em `.rhai` — §2.4          |
| **Fecha requisito** | `PRD-007` integral · `PRD-003` §3.2 (perfis de rede e de UI) · N-01 (`<10μs`) vira medida · `PRD-009`/`PRD-010` **novos**            |

> ⚠️ **Base de referência das citações.** Toda referência `arquivo:linha` deste relatório foi conferida contra
> `feat/v0-2-implementation` (commit `6536bbc`, PR #5), **não** contra `main`. Em `main` os números de linha do
> `ROADMAP-IMPLEMENTACAO-V1.md` diferem (largura de reflow anterior), e `PRD-006`/`PRD-007`/`PRD-008`, `ADR-0011`,
> `ADR-0013`, `core/dom` e `core/engine` sequer existem. Ler este documento contra `main` produz citação pendurada; ele
> só é verificável sobre as PRs #4 e #5.

---

Este relatório cobre **apenas a v0.5** do `ROADMAP-IMPLEMENTACAO-V1.md` — as fases **F8** (`core/window` +
`core/network`, `:265`), **F9** (`core/css`, `:266`) e o ponto de integração **I4** (`:281`), que o roadmap §3.1 agrupa
sob "Browser de verdade" (`:218`). É a primeira versão apresentável a quem não é do time (`:238`). Nada aqui foi
implementado.

Quatro decisões de escopo foram tomadas com o solicitante antes deste plano e valem como premissa em todo o documento:

1. **TLS com provider de cripto em Rust puro**, sem `unsafe` em nenhum byte que venha da rede — e não o provider padrão
   do `rustls`. O custo é maturidade e desempenho, e está assumido em §2.5.
2. **A política vai para o muscle de verdade.** Roteamento de evento de janela e política de rede rodam em `.rhai`
   embarcado, sob o ciclo `on_init`/`on_event`/`on_process` de `PRD-001:87-90` — §2.4.
3. **O framebuffer chega à janela por `softbuffer`**, não por um `OpenGLBackend` antecipado da F12 — §2.7.
4. **Entram três fatias além do mínimo**: `<img>` com decodificador próprio de PNG, o portão `criterion` de `<10μs`, e a
   quebra de `rhai-runtime` em `rhai-bindings`.

---

## 1. Estado assumido e o que a v0.5 acrescenta

### 1.1 A premissa — e a prova de que ela ainda não é o estado do repositório

Este plano assume a v0.3 entregue como especificada em `IMPLEMENTACAO-DETALHADA-V0-3.md`. **Ela não está.** No branch
`feat/v0-2-implementation` (commit `6536bbc`), os seis crates que a v0.3 e a v0.5 preenchem continuam idênticos ao que
`cargo new` gerou:

```bash
wc -l core/{css,html,graphics,window,network,js}/src/lib.rs
# → 16 linhas cada, todas com o mesmo `add(left, right)` e o mesmo `it_works()`
```

E o grafo de dependências externas do workspace tem exatamente dois crates — `bitflags` e `rhai` (`Cargo.toml:33-40`).
Nada de rede, nada de janela:

```bash
grep -rn "winit\|rustls\|softbuffer\|reqwest\|hyper\|ureq\|tokio\|http" --include=Cargo.toml .
# → 1 resultado: Cargo.toml:24, a URL do repositório. Zero dependências.
```

O que a v0.2 **de fato** entregou e a v0.5 usa como fundação: o port do `RuntimeEngine` com os sete itens do `ADR-0011`
verdes e `PORT_SCHEMA_VERSION = 2` (`core/engine/src/lib.rs:65`); os nove bitflags de `Capability`
(`core/engine/src/domain/capability.rs:16-26`); e o chokepoint de capability, que hoje está **vazio de carga de
produção** — o próprio arquivo declara isso:

```rust
// core/runtime/rhai/src/infrastructure/sandbox.rs:8-10
//! v0.2 ships **no** production guarded bindings (all DOM access is through
//! `NodeHandle` methods, which self-guard). The mechanism is here, tested, and
//! ready for the first scripted policy port.
```

A v0.5 **é** esse primeiro port de política scriptada. É a versão em que o mecanismo de sandbox construído na v0.2 passa
a segurar peso real.

### 1.2 O que a v0.5 acrescenta

Nenhum dos 18 critérios numerados do roadmap (`:115-134`) cai na v0.5 — C-10 a C-13 são hot-reload (F11), C-15/C-16 são
GPU (F12). Isso torna fácil subestimar a versão, e é exatamente o contrário: a v0.5 fecha o PRD que hoje mais promete e
menos entrega, e instancia metade da tabela de perfis de capability que existe desde a v0.1 sem nunca ter tido código.

| Entrega                                                         | Origem            | Como fecha                                                                    |
| --------------------------------------------------------------- | ----------------- | ----------------------------------------------------------------------------- |
| `CascadeResolver` / `LayoutEngine` com adaptadores **reais**    | `PRD-007:92-101`  | F9 integral: parser, seletores, cascata, box model, IFC e Flexbox             |
| Adaptador de cascata em `.rhai` sob `DOM_READ \| GRAPHICS_DRAW` | `PRD-007:96-98`   | Fatia M: o contrato do `PRD-007` §3.4 deixa de ser hipótese                   |
| Perfil **Network Interceptor** com código                       | `PRD-003:57`      | `default_network.rhai` sob `NETWORK_FETCH \| FS_WRITE_CACHE`                  |
| Perfil **UI & Window Manager** com código                       | `PRD-003:58`      | `default_ui.rhai` sob `WINDOW_MANAGE \| GRAPHICS_DRAW \| DOM_READ`            |
| N-01 (`<10μs` por hook) vira número aferido                     | `PRD-001:96`      | Portão `criterion`, que o roadmap liga exatamente aqui (`:358`)               |
| Dois ports novos sob o contrato do `ADR-0011`                   | `ADR-0011:79-105` | `PRD-009` (transporte + política de rede) e `PRD-010` (janela + apresentação) |

**Micro-entregáveis da versão** (`ROADMAP-IMPLEMENTACAO-V1.md:238`): `alloy https://example.com` abre janela nativa e
renderiza a página real; redimensionar a janela refaz o layout.

### 1.3 Por que 84–126 d e não os 50–75 d do roadmap

O intervalo do roadmap (`:218`) soma F8 (20–30) + F9 (30–45) e não orça a fatia de muscle, o `<img>`, a quebra de crate
nem a papelada dos dois ports novos. O delta é escopo **escolhido**, não estouro:

| Bloco                                                        | Esforço `[modelado]` | Estava no roadmap?              |
| ------------------------------------------------------------ | -------------------- | ------------------------------- |
| F9a — parser CSS, seletores, especificidade                  | 10–15 d              | Sim (parte dos 30–45 de F9)     |
| F9b — cascata, herança, valores computados, unidades, cores  | 8–12 d               | Sim                             |
| F9c — box model, IFC, fluxo normal, Flexbox                  | 16–24 d              | Sim                             |
| F8a — `core/network`: HTTP/1.1, TLS, charset                 | 14–20 d              | Sim (parte dos 20–30 de F8)     |
| F8b — `core/window`: `winit`, eventos, apresentador          | 8–12 d               | Sim                             |
| I4 — integração, navegação, resize, subrecursos              | 8–12 d               | Sim (implícito no ponto)        |
| **M** — política em `.rhai`, bindings, perfis                | 8–12 d               | Não — decisão de escopo 2       |
| **X** — `<img>`, inflate, decodificador PNG, `DrawImage`     | 5–8 d                | Não — decisão de escopo 4       |
| **R** — quebra de `rhai-runtime` em `rhai-bindings`          | 2–3 d                | Não — risco §6.4 da v0.3        |
| **P** — portões, ADRs, `PRD-009`/`PRD-010`, contract records | 5–8 d                | Parcial (`:358` liga um portão) |

Alavanca de alívio, se F9c estourar: entregar Flexbox de linha única (sem `flex-wrap`) e adiar o _wrap_ para a v0.7. É a
única fatia de F9c que sai sem tornar o layout inútil para páginas reais — §2.9.

### 1.4 ⚠️ Divergências de documentação a corrigir nesta entrega

`docs/architecture/overview.md:92-93` declara `window → graphics, engine` e `network → engine`. As duas linhas
contrariam a decisão 2.1 da v0.3 (`IMPLEMENTACAO-DETALHADA-V0-3.md:94-107`), que generalizou a regra da v0.2: **nenhum
crate de domínio nomeia `engine`**. E `window → graphics` é evitável sem custo — §2.7. As linhas viram `window → nada` e
`network → nada`, e a correção é entregável desta versão, não dívida.

O índice de ADRs (`docs/adr/README.md:12-23`) pula o **ADR-0012**. Não é lacuna: `ADR-0011:128` o reserva para a escolha
do motor de JS de conteúdo, que é decisão da v0.7. Os ADRs novos da v0.5 são o **0016** e o **0017** — a v0.3 já
reivindicou 0014 e 0015 (`IMPLEMENTACAO-DETALHADA-V0-3.md:590`).

---

## 2. As decisões de design

### 2.1 `unsafe` é decidido por superfície de ameaça, não por conveniência (ADR-0016)

A escolha de um provider de cripto em Rust puro (§2.5) só é coerente se a regra que a motiva for escrita — porque a
mesma versão precisa de `winit` e `softbuffer`, e chamar a API de janela do sistema operacional é FFI.

### ⚠️ Defeito encontrado: N-02 já está violado hoje, no lugar exato que ele protege

Antes de escrever a regra, o estado real. O `rhai` fixado em `Cargo.toml:40` contém quatro blocos `unsafe`, e **três
deles estão no caminho de registro e de chamada de função nativa** — a costura por onde todo binding guardado passa:

```bash
grep -rn "unsafe " --include="*.rs" ~/.cargo/registry/src/*/rhai-1.26.0/src
# src/reify.rs:19,56          — transmute_copy
# src/func/register.rs:60     — transmute_copy no registro de função nativa
# src/func/call.rs:87         — transmute_copy no despacho da chamada
```

N-02 (`PRD-001:97`) exige _"zero unsafe memory operations **exposed to script runtimes**"_. `rhai::func::call` **é** o
runtime de script, e `func/register.rs` é o caminho que `register_guarded_binding` usa. O requisito não é violado por
nada que a v0.5 introduza: ele está violado desde a v0.1, pela escolha de motor do `ADR-0002`.

Isso não condena o `rhai` — `transmute_copy` atrás de checagem de `TypeId` é o padrão de qualquer despacho dinâmico em
Rust, e o `bitflags` da outra dependência é limpo (`forbid(unsafe_code)` em `src/lib.rs:273`). O que o achado condena é
a formulação de N-02: **"zero unsafe" nunca foi verdade e nunca foi verificado.** Rejeitar `ring` por trazer `unsafe`
enquanto se despacha toda chamada de binding por `transmute_copy` não é rigor, é inconsistência.

A regra honesta é por superfície de ameaça, e ela descreve o que o projeto **já faz**:

| Superfície                                                         | `unsafe` de terceiros  | Por quê                                                                                                         |
| ------------------------------------------------------------------ | ---------------------- | --------------------------------------------------------------------------------------------------------------- |
| Bytes controlados pelo atacante — TLS, HTTP, HTML, CSS, PNG, fonte | **Proibido**           | É a superfície que N-02 (`PRD-001:97`) existe para proteger; um _overflow_ aqui é executado por qualquer página |
| Script **confiável** (muscle) — despacho de binding do `rhai`      | Permitido, **nominal** | `PRD-003:21-24` modela o script de muscle como _bugado_, não adversário; o autor é o usuário                    |
| FFI de plataforma sem alternativa — janela, superfície, event loop | Permitido, **nominal** | Não processa entrada hostil; é a fronteira com o SO, e o SO já é confiança pressuposta                          |
| Conveniência — SIMD, alocação, otimização                          | **Proibido**           | Foi o critério que a v0.3 usou para rejeitar `simd-adler32` (`IMPLEMENTACAO-DETALHADA-V0-3.md:239-241`)         |

A linha 2 tem prazo de validade: na v0.7, `core/js` executa script **adversário**, e o motor de conteúdo cai na linha 1,
não na 2. O `ADR-0016` deve escrever isso agora, para que a escolha do motor de JS (reservada ao `ADR-0012`,
`ADR-0011:128`) nasça sabendo que o critério de `unsafe` dela é o estrito.

`#![forbid(unsafe_code)]` continua em **todo** crate nosso, sem exceção. O que a regra governa é a árvore de
dependências, que `cargo-deny` não inspeciona — ele audita CVE e licença, não blocos `unsafe`. Portanto **portão novo**:
job de CI `unsafe-audit` que roda `cargo-geiger` sobre o workspace e falha se aparecer `unsafe` em crate fora de uma
_allowlist_ nominal e comentada — que nasce com `rhai` dentro, e com o comentário dizendo por quê. A allowlist é o
registro revisável das linhas 2 e 3 da tabela; ela cresce por revisão, nunca por acidente. Isso vira o **ADR-0016**, e
`PRD-001:97` é **reescrito** na mesma entrega: um requisito que nunca foi verdade corrói os outros quatro.

### As alternativas sem `unsafe`, e o que cada uma custa

A pergunta "existe caminho sem `unsafe`?" tem resposta diferente para cada superfície, e só uma delas é real:

| Alternativa                                    | Elimina `unsafe`?                                                | Custo                                                                                             | Veredito                                         |
| ---------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `breadx` no lugar de `winit` + `softbuffer`    | Sim para janela — tem `forbid(unsafe_code)`                      | Fala X11 por socket: **só Linux/X11**. Sem Wayland, sem Windows, sem macOS. Blit próprio, +8–12 d | Rejeitada: custa a matriz de 3 SOs (`:344`)      |
| `x11rb` com `rust-connection`                  | **Não** — mantém `unsafe` para FFI de libxcb                     | Mesma perda de plataforma, sem o ganho                                                            | Rejeitada: paga o custo e não entrega o objetivo |
| `wayland-client` com backend Rust              | **Não** — backend puro, mas _"little unsafe"_, não zero          | Idem                                                                                              | Rejeitada: idem                                  |
| Adiar a janela: v0.5 headless, F8b para a v0.7 | Sim — nenhuma dependência de plataforma entra                    | A v0.5 deixa de ser a primeira versão apresentável (`:238`); a entrega vira `alloy render <url>`  | **A única alternativa honesta** — §6.7           |
| Processo separado só para janela e blit        | Não — confina, não elimina; memória compartilhada exige `unsafe` | +10–15 d de IPC para proteger uma superfície que não recebe bytes do atacante                     | Rejeitada: desproporcional                       |

Nenhuma delas toca o `rhai`, e é por isso que **nenhuma produz uma árvore sem `unsafe`**: só trocar o motor de muscle
faria isso, e trocá-lo contradiz o `ADR-0002`. A escolha entre a regra acima e a v0.5 headless é uma decisão de produto,
não de engenharia, e está registrada como risco §6.7.

### 2.2 Dois ports novos, e por que rede e janela merecem o contrato inteiro

`ADR-0011:118-124` lista cinco ports governados; nem `core/network` nem `core/window` estão lá. Poderia-se ler isso como
dispensa. É o contrário: `PRD-001:27` promete explicitamente _"intercepting requests"_ como caso de uso de
malleabilidade, e `PRD-003:57` já reserva um perfil de capability para o interceptador de rede. O seam existe no
requisito desde a v0.1 — falta o contrato.

| Port                              | Crate          | Espécie              | PRD novo  | Congela em |
| --------------------------------- | -------------- | -------------------- | --------- | ---------- |
| `HttpTransport` / `RequestPolicy` | `core/network` | Mecanismo + política | `PRD-009` | I4         |
| `WindowSystem` / `Presenter`      | `core/window`  | Mecanismo            | `PRD-010` | I4         |

O ganho não é burocrático, é de testabilidade, e é grande: as _features_ `no-transport` e `no-window` que o item 6 do
contrato (`ADR-0011:99-102`) obriga são exatamente o que torna **I4 verificável em CI**. Com um `MockTransport` servindo
respostas HTTP de fixture e um `HeadlessWindowSystem`, a frase "abre uma URL e renderiza" vira golden image
determinística num runner sem rede e sem display. Sem os ports, I4 só seria testável manualmente — e um ponto de
integração que só se verifica à mão regride em silêncio.

`RequestPolicy` é o port de **política** no sentido de `ADR-0011:107-113`: pode ser dirigido em tempo de execução por
`.rhai` (§2.4). `HttpTransport` e `WindowSystem` são mecanismo, trocados em tempo de compilação.

### 2.3 Um único event loop dono da thread principal (ADR-0017)

O roadmap marca isso como armadilha de I5 (`:318`) — _"dois laços disputando a thread principal"_ — e I5 é v0.9. Mas a
v0.5 é a primeira versão a **ter** um laço: `winit` toma posse da thread principal, e é irreversível depois que rede,
hot-reload e event loop de JS chegarem. Resolver na v0.5 custa desenho; resolver na v0.9 custa reescrita.

A regra: **o event loop de janela é o único dono da thread principal.** Todo I/O bloqueante — DNS, TCP, handshake TLS,
leitura de corpo — roda em _worker_ de um pool de threads `std`, e o resultado volta por `std::sync::mpsc` como um
evento do mesmo laço. Sem runtime assíncrono, sem `tokio`: a assinatura do `RuntimeEngine` não é `async`, e introduzir
um executor para depois ter de atravessá-lo a cada hook é comprar um problema que ninguém pediu.

Consequências que já ficam pagas para I5: o _watcher_ de hot-reload da F11 é mais um produtor no mesmo canal; o event
loop de JS da F10 é mais um consumidor no mesmo laço. Vira **ADR-0017**.

### 2.4 A política vai para o muscle — e é aqui que a tese do ADR-0003 é provada ou não

Até a v0.3 o Alloy roda scripts, mas nenhum script **decide** nada: `--script` executa um `.rhai` que constrói um DOM e
sai. A v0.5 é a primeira com eventos reais — clique, tecla, resize, navegação, requisição — e portanto a primeira em que
a distinção Skeleton/Muscle do `ADR-0003` é verificável em vez de declarada.

Dois scripts embarcados por `include_str!`, cada um num `ExecutionContext` isolado com o perfil exato de
`PRD-003:53-58`:

| Script                 | Capabilities                                 | Decide                                                                  |
| ---------------------- | -------------------------------------------- | ----------------------------------------------------------------------- |
| `default_ui.rhai`      | `WINDOW_MANAGE \| GRAPHICS_DRAW \| DOM_READ` | Roteamento de evento de janela, atalhos, quando repintar, título da aba |
| `default_network.rhai` | `NETWORK_FETCH \| FS_WRITE_CACHE`            | Permitir/negar/reescrever requisição, política de redirect, cabeçalhos  |

O ciclo é o de `PRD-001:87-90` — `on_init()`, `on_event(event)`, `on_process(state)` — e cada chamada passa pelo
`run_with_fallback` que a v0.2 já entrega: script que falha escreve diagnóstico, o host cai no adaptador Rust embutido,
e **a página continua renderizando**, que é literalmente o requisito 3 de `PRD-007:82` e o §4 de `PRD-003:62-70`.

O que a decisão **não** é: mover mecanismo para script. O socket, o parser, a árvore e o rasterizador continuam em Rust.
O que atravessa a costura é decisão — e o volume de travessias é o que o portão de `<10μs` (§2.13) passa a medir.

Custo aceito e registrado: `GuardedBinding` (`core/runtime/rhai/src/infrastructure/sandbox.rs:16-21`) deixa de ter zero
entradas de produção e passa a ter duas tabelas. As duas entram na varredura de C-06 e na matriz de injeção de pânico de
C-09 que já existem — o sandbox da v0.2 é reaproveitado inteiro, não reescrito.

### 2.5 `core/network`: HTTP/1.1 escrito à mão, TLS com provider RustCrypto

`domain/` — `Url` (esquema, host, porta, caminho, query, com validação e normalização), `HttpRequest`/`HttpResponse`,
`HeaderMap` como _first-class collection_ (`ADR-0010` regra 2, já nomeada em `CLAUDE.md`), `StatusCode`, `Method`,
`MediaType`, e um `NetworkError` `#[non_exhaustive]` com localização — aqui a "linha" é a fase do protocolo
(`Dns`/`Connect`/`Handshake`/`Header`/`Body`), o análogo do `SourceLocation` que o item 4 do contrato exige
(`ADR-0011:93-95`).

`infrastructure/` — cliente HTTP/1.1 escrito à mão sobre `std::net::TcpStream`: _request line_, cabeçalhos,
`Content-Length` e `Transfer-Encoding: chunked`, `keep-alive` com pool por `(host, porta)`, redirect com limite de 20 e
detecção de ciclo, e _timeout_ em cada fase — rede hostil sem timeout é _hang_, não erro.

A resolução de nome usa `std::net::ToSocketAddrs`, ou seja, o resolvedor do sistema. É bloqueante, e é por isso que ela
mora num _worker_ (§2.3). **Não** escrevemos cliente DNS na v0.5.

**TLS**: `rustls` com `CryptoProvider` montado a partir dos crates RustCrypto, em vez do provider padrão
(`aws-lc-rs`/`ring`, ambos com assembly e `unsafe`). A leitura literal de N-02 (`PRD-001:97`) — _"unsafe exposto a
runtimes de script"_ — permitiria o provider padrão; a decisão do solicitante foi a leitura estrita, e ela é defensável
pela regra de §2.1: cripto processa bytes que o atacante escolhe.

| Opção                           | O que é                             | Custo                                                                                      | Veredito                                             |
| ------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------ | ---------------------------------------------------- |
| `rustls` + provider RustCrypto  | Handshake e AEAD em Rust puro       | Provider menos maduro que o padrão; handshake e _bulk crypto_ mais lentos; menos auditoria | **Escolhida** — decisão de escopo 1                  |
| `rustls` + `aws-lc-rs` / `ring` | Provider padrão, assembly otimizado | `unsafe` na superfície que decifra bytes do atacante                                       | Rejeitada pela regra de §2.1                         |
| `reqwest` / `hyper`             | Cliente HTTP completo               | Puxa `tokio` e um executor assíncrono que colide com o laço único de §2.3                  | Rejeitada: contradiz o ADR-0017 antes de ele existir |
| TLS próprio                     | Handshake e cifra do zero           | Meses, e uma superfície criptográfica não auditada protegendo o usuário                    | Rejeitada: irresponsável                             |

Raízes de confiança por `webpki-roots` (conjunto embarcado, determinístico) e **não** pelo _trust store_ do sistema — o
mesmo raciocínio do provedor sintético de fontes da v0.3 (`ROADMAP-IMPLEMENTACAO-V1.md:315`): teste que depende do
estado da máquina não é teste.

Fora da v0.5, declarado: HTTP/2, HTTP/3, cookies, cache em disco, autenticação, proxy, Brotli. **Dentro**, porque sai
quase de graça: `Content-Encoding: gzip`/`deflate`, já que o `<img>` (§2.11) obriga um _inflate_ de qualquer forma.

### 2.6 Decodificação de charset: UTF-8 e windows-1252, escritos à mão

A v0.3 empurrou isso explicitamente para cá (`IMPLEMENTACAO-DETALHADA-V0-3.md:371-372`). A ordem de decisão é a do HTML:
BOM → `charset` do `Content-Type` → `<meta charset>` nos primeiros 1024 bytes → _fallback_ windows-1252.

Só dois decodificadores entram: UTF-8 (pela `std`, com substituição de sequência inválida por `U+FFFD`) e windows-1252
(uma tabela de 256 entradas — 30 linhas). `encoding_rs` seria o caminho fácil e é rejeitado pela regra de §2.1: ele
decodifica bytes vindos da rede e contém `unsafe`. Qualquer outro rótulo de charset devolve `NetworkError` com o rótulo
não suportado, visível, em vez de renderizar lixo em silêncio.

### 2.7 `core/window`: `winit`, apresentador `softbuffer`, e zero dependência de `graphics`

`domain/` — `WindowEvent` `#[non_exhaustive]` (`Resized`, `CloseRequested`, `PointerMoved`, `PointerButton`, `Key`,
`Scroll`, `RedrawRequested`), `SurfaceSize`, `PhysicalPosition`, `ScaleFactor`, `WindowError`. **Nenhum tipo de `winit`
aparece em assinatura pública** — `infrastructure/winit_system.rs` mapeia os eventos do `winit` para os nossos, que é a
mesma disciplina de mapeamento explícito do item 3 do contrato (`ADR-0011:90-92`).

`application/ports.rs` — `WindowSystem` (criar janela, bombear eventos) e `Presenter`:

```rust
// core/window/src/application/ports.rs — assinatura proposta
pub trait Presenter: Send {
    fn present(&mut self, frame: FrameView<'_>) -> Result<(), WindowError>;
}
```

`FrameView<'a>` é um _value object_ emprestado — largura, altura e `&'a [u32]` em RGBA8 pré-multiplicado. É o que
**elimina a aresta `window → graphics`** que `overview.md:92` documenta: `core/window` nunca nomeia
`graphics::Framebuffer`; quem constrói a `FrameView` a partir dele é o `alloy`, em uma linha, sem cópia. Custo zero, uma
aresta a menos no grafo, e o item 3 do `ADR-0011` respeitado ao pé da letra.

A apresentação em si é `softbuffer`: _blit_ dos pixels para a superfície da janela, sem GPU. O `SoftwareCpuBackend` da
v0.3 continua sendo o **único** produtor de pixels, o que preserva intacto o portão de golden image e o determinismo do
ADR-0014 — a janela mostra exatamente o que o PNG de referência contém. Antecipar o `OpenGLBackend` da F12 daria janela
acelerada e quebraria as duas coisas, além de inverter a ordem que o roadmap declara não-negociável (`:293`).

`HeadlessWindowSystem` é o adaptador de referência que o item 6 exige, e o que faz I4 rodar em CI.

### 2.8 `core/css`: o parser, os seletores e a cascata que a v0.3 deliberadamente não escreveu

A v0.3 entrega os agregados de fronteira e as portas do `PRD-007` com adaptadores UA-only, **sem parser**
(`IMPLEMENTACAO-DETALHADA-V0-3.md:243-274`). A v0.5 troca os miolos atrás das mesmas traits — que é a prova de que a
costura valeu a pena, e o teste real do `ADR-0011`.

`infrastructure/parser/` — tokenizador de CSS Syntax Level 3 (o suficiente para folhas reais: _at-rules_, blocos,
funções, `url()`, strings, escapes, comentários) e o parser de regras que popula o `StyleSheetSet` que já existe. Entram
`<style>`, `style=` e `<link rel=stylesheet>` (subrecurso, via §2.11).

Seletores, com especificidade de três componentes e ordem de origem/documento como desempate:

| Dentro                                                                       | Fora, e por quê                                            |
| ---------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Tipo, universal, `.classe`, `#id`, `[attr]` e `[attr=v]`, listas             | `:has()` — exige _matching_ reverso, custo desproporcional |
| Combinadores descendente, `>`, `+`, `~`                                      | Namespaces — sem conteúdo estrangeiro até a v1.0           |
| `:hover`, `:active`, `:focus`, `:first-child`, `:last-child`, `:nth-child()` | `::before`/`::after` — geram caixa sem nó; v0.7            |
| `@media` com `min-width`/`max-width`                                         | `@supports`, `@font-face`, `@import`, `@keyframes`         |

`!important`, as três origens (`UserAgent`/`User`/`Author`) e herança entram integralmente — sem eles não existe
cascata, só ordem de regra. Valores computados cobrem `px`/`em`/`rem`/`%`/`pt`, cores
nomeadas/`#rgb`/`#rrggbb`/`rgb()`/`rgba()`, e as propriedades que F9c precisa resolver.

O recorte acima é **declarado por manifesto**, no molde do recorte html5lib da v0.3
(`IMPLEMENTACAO-DETALHADA-V0-3.md:299-309`): `core/css/tests/data/MANIFEST.md` lista propriedade a propriedade e seletor
a seletor o que a v0.5 suporta, e o runner falha se o código suportar algo não listado ou listar algo não suportado. É o
que impede o recorte de encolher em silêncio para o CI ficar verde.

### 2.9 Layout: box model, contexto inline de verdade e Flexbox

`block_layout.rs` da v0.3 é substituído, não estendido. O que entra:

1. **Box model completo** — `margin`/`border`/`padding`, `box-sizing`, colapso de margem vertical entre irmãos e entre
   pai e primeiro/último filho. Colapso de margem é a regra que mais parece detalhe e mais quebra página quando falta.
2. **Contexto de formatação inline** — caixas de linha de verdade: `white-space` (`normal`/`pre`/`nowrap`), colapso de
   espaço, oportunidades de quebra suave (espaço e após hífen — UAX #14 simplificado), `text-align`
   (`left`/`right`/`center`/`justify`), inlines aninhados, e alinhamento por _baseline_.
3. **Flexbox** — eixo por `flex-direction`, `flex-wrap`, `justify-content`, `align-items`, `align-content`,
   `align-self`, e a resolução de `flex-grow`/`flex-shrink`/`flex-basis`. É o que `ROADMAP-IMPLEMENTACAO-V1.md:266`
   pede, e é a fatia com maior variância da versão.

Fora, declarado: `float`, `position: absolute/fixed/sticky`, Grid, `writing-mode`, BiDi, tabelas com algoritmo de
largura automática, `z-index` com contexto de empilhamento completo.

Toda geometria continua em `Au(i32)` (ADR-0014 da v0.3), sem exceção: é o que mantém a golden image idêntica nos três
SOs depois de o layout ficar dez vezes mais complexo.

**A alavanca de alívio de §1.3 é aqui**: `flex-wrap` é a única sub-fatia que sai sem tornar Flexbox inútil, porque a
esmagadora maioria das páginas usa flex de linha única. Sai por decisão registrada no `MANIFEST.md`, nunca por
descoberta na véspera.

### 2.10 Fontes: resolução de `font-family` com fallback para fontes do sistema

`font-family` no CSS ganha suporte completo à lista de famílias e mapeamento para as categorias genéricas (`sans-serif`,
`serif`, `monospace`). Em runtime, o `SystemFontProvider` (introduzido na v0.3) resolve nomes de família através do
catálogo do sistema (`FontCatalog`), utilizando a tabela de mapeamento por SO (ex.: `sans-serif` → DejaVu Sans/Ubuntu no
Linux, SF Pro/Helvetica no macOS, Segoe UI/Arial no Windows; `serif` → DejaVu Serif/Times New Roman; `monospace` →
DejaVu Sans Mono/Menlo/Consolas) combinada com inspeção de tabelas OpenType via `ttf-parser`.

A separação introduzida na v0.3 é mantida: os testes de golden e conformidade continuam utilizando o `FontProvider`
sintético/mock para garantir 100% de determinismo entre plataformas nos testes automatizados, enquanto o runtime do
navegador resolve fontes reais do sistema operacional.

### 2.11 `<img>`, o _inflate_ que ele obriga, e o relayout que quase ninguém orça

`DrawImage` existe na `DisplayList` desde a v0.3 e é recusado pelo backend (`IMPLEMENTACAO-DETALHADA-V0-3.md:148-149`).
A v0.5 o implementa. O trabalho tem três partes, e a terceira é a que costuma faltar no orçamento:

1. **Decodificador** — PNG apenas. O _inflate_ (RFC 1951) é escrito por nós, ~300 linhas, e é o mesmo usado pelo
   `Content-Encoding: gzip` de §2.5, o que paga a conta duas vezes. `png`/`miniz_oxide` seriam o caminho fácil e ficam
   condicionados ao portão de §2.1: entram **se** o `unsafe-audit` passar; caso contrário o decodificador próprio é a
   alavanca declarada (+4–6 d `[modelado]`). JPEG fica fora da v0.5, declarado.
2. **Rasterização** — `DrawImage` no `SoftwareCpuBackend`, com escala por amostragem de caixa em inteiros. Nada de
   filtro em ponto flutuante: o determinismo do ADR-0014 vale para pixel de imagem como vale para glifo.
3. **Sizing intrínseco e o segundo layout** — a largura e a altura da imagem só são conhecidas depois do download. O
   layout roda primeiro com a caixa vazia (ou com `width`/`height` do autor, se houver), e a chegada de cada imagem
   dispara **re-cascata e re-layout da árvore inteira**, porque `PRD-007:51` manda granularidade grossa. Isso é correto
   e é caro; §2.13 é o que impede que fique caro em silêncio.

Alvo de _fuzz_ obrigatório: o decodificador PNG e o _inflate_. Formato de imagem é entrada hostil por definição, e um
pânico no decodificador é DoS (`ROADMAP-IMPLEMENTACAO-V1.md:338`).

### 2.12 A quebra de `rhai-runtime` em `rhai-bindings`

A v0.3 registrou como risco §6.4 (`IMPLEMENTACAO-DETALHADA-V0-3.md:546-548`): decidir **antes** do terceiro bridge. A
v0.5 traz o terceiro, o quarto e o quinto de uma vez — cascata scriptável, política de rede, política de UI. Hoje o
crate tem duas dependências de domínio (`core/runtime/rhai/Cargo.toml:10-16`); sem a quebra, teria cinco, e
`rhai-runtime` deixaria de ser _"the only place a rhai type is named"_ (`:8`) para virar o lugar onde todo crate de
domínio é nomeado.

| Crate                        | Fica com                                                                 | Depende de                                            |
| ---------------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------- |
| `core/runtime/rhai`          | `RhaiEngine`, `RhaiContext`, marshaling, sandbox, fallback, error map    | `engine`, `rhai` — **e nenhum crate de domínio**      |
| `core/runtime/rhai-bindings` | `dom_bindings`, `display_list_bindings`, `css_bindings`, `net`, `window` | `rhai-runtime`, `engine`, `dom`, `graphics`, `css`, … |

O glob `core/runtime/*` de `Cargo.toml:12` já inclui o crate novo sem nenhuma edição no manifesto do workspace — a
decisão de membros explícitos + glob da F0 paga dividendo aqui. `rhai-runtime` volta a ser um backend puro, e o portão
`no-engine` da CI ganha uma asserção nova: `cargo tree -p rhai-runtime` não mostra `dom`.

Custo real: 2–3 d `[modelado]`, quase todo em mover arquivo e reajustar `use`. Feito depois, com cinco bridges no lugar,
é o dobro — e é por isso que **R é o primeiro passo do plano**, não o último.

### 2.13 O portão `<10μs`, e a generalização do `EngineError` que ele expõe

`ROADMAP-IMPLEMENTACAO-V1.md:358` liga o portão `criterion` exatamente na v0.5, e a decisão de §2.4 é o que lhe dá o que
medir: sem hook, não há travessia; sem travessia, o portão é ruído verde. A medida é o _round-trip_ completo de
`on_event` — construir `EngineValue`, atravessar a guarda de capability, executar o corpo, converter o retorno — com p99
< 10μs (`PRD-001:96`).

A armadilha que o roadmap aponta em `:309` — checagem de capability em todo binding × orçamento de 10μs — já está
resolvida pela v0.2: `register_guarded_binding` captura o `CapabilitySet` por valor e a guarda é um `and` de bits mais
um desvio. Não há resolução de permissão por chamada. O benchmark existe para provar isso e detectar regressão, não para
descobrir.

**E ele expõe um defeito de desenho que vale corrigir agora.** `EngineError` ganhou `Dom` na v0.2
(`core/engine/src/domain/error.rs:65`), ganha `Graphics` na v0.3, e ganharia `Network` e `Window` aqui — quatro
variantes que diferem só no nome do subsistema, num enum que o `ADR-0011` congelou e cuja mudança exige bump de schema e
nota de migração a cada vez. A recomendação é **generalizar**: uma variante
`Subsystem { subsystem: SubsystemName, operation, reason }` que absorve as quatro, com `Dom` e `Graphics` mantidas como
`#[deprecated]` durante a v0.5. Custo: um bump (`PORT_SCHEMA_VERSION` 3 → 4), uma nota de migração em `PRD-002` §4.2 e a
travessia dos _call sites_ — meio dia. Não fazer custa uma variante nova a cada subsistema até a v1.0, e a v0.7 traz
mais dois.

### 2.14 Onde `paint.rs` fica — a promessa da v0.3, revisitada

A v0.3 pôs o mapeamento `LayoutBoxTree → DisplayList` em `alloy/src/application/paint.rs` e prometeu promovê-lo a crate
próprio _"quando aparecer o segundo consumidor (F8, `core/window`)"_ (`IMPLEMENTACAO-DETALHADA-V0-3.md:134-135`).

**O segundo consumidor não apareceu, e a promessa não se cumpre.** `core/window` consome pixels, não caixas — é
exatamente o que a `FrameView` de §2.7 formaliza. Os dois consumidores de `paint` continuam sendo `alloy render`
(headless) e `alloy <url>` (com janela), ambos dentro do `alloy`. Promover agora criaria um crate com um único
dependente para satisfazer uma previsão que os fatos não confirmaram. `paint.rs` fica onde está; a promessa é
**reavaliada na F10**, quando `core/js` puder pintar fora do ciclo de navegação.

Registrar isso importa mais do que a decisão em si: uma promessa de plano que ninguém revisita vira ficção de
documentação, que é o risco §6 do próprio roadmap (`:425-427`).

### 2.15 Os dois ports novos contra os sete itens do `ADR-0011:79-105`

| Item                             | `HttpTransport` / `RequestPolicy`                             | `WindowSystem` / `Presenter`                              |
| -------------------------------- | ------------------------------------------------------------- | --------------------------------------------------------- |
| 1 Seam PRD + variação + ameaça   | `PRD-009` **novo** — ameaça: servidor hostil                  | `PRD-010` **novo** — ameaça: nenhuma; é FFI de SO         |
| 2 Traits sem tipo de adaptador   | Object-safe; nenhum tipo de `rustls` em assinatura            | Object-safe; nenhum tipo de `winit` em assinatura         |
| 3 Agregados versionados          | `HttpRequest`/`HttpResponse`/`Url`; `PORT_SCHEMA_VERSION = 1` | `WindowEvent`/`SurfaceSize`/`FrameView`; `= 1`            |
| 4 Um erro tipado com localização | `NetworkError` + fase do protocolo                            | `WindowError` + identificador de janela                   |
| 5 Ciclo de vida e concorrência   | `http-transport-port-contract.md` — worker + canal (§2.3)     | `window-system-port-contract.md` — dono da thread         |
| 6 Conformidade + ref + `no-*`    | `run_transport_suite` · `MockTransport` · `no-transport`      | `run_window_suite` · `HeadlessWindowSystem` · `no-window` |
| 7 Congelamento                   | Congela em **I4**                                             | Congela em **I4**                                         |

O item 5 do port de janela é o mais carregado: é onde a regra "um único event loop dono da thread principal" do ADR-0017
fica escrita como contrato, e não como convenção que a F10 pode desconhecer.

### 2.16 O que NÃO fazer na v0.5

- **Não** escrever `VulkanBackend` nem `OpenGLBackend` — F12/v0.9. A janela recebe pixels do rasterizador de software
  (§2.7).
- **Não** implementar hot-reload, _watcher_ nem `on_reload()` — F11/v0.9. Os scripts são embarcados por `include_str!`;
  substituí-los ainda exige recompilar.
- **Não** tocar `core/js`, `devtools` nem `extension`. `<script>` continua sendo suspensão de tokenizer sem consumidor.
- **Não** implementar `Origin`, perfil `WEB_CONTENT` nem isolamento por aba — é F7/v0.7, e o roadmap é explícito em
  `:296` que a fronteira de conteúdo hostil vem antes do motor de JS, não antes da rede.
- **Não** implementar múltiplas abas. A v0.5 tem uma janela e um documento.
- **Não** adicionar HTTP/2, cookies, cache em disco, proxy ou autenticação (§2.5).
- **Não** adicionar `float`, `position`, Grid ou `::before`/`::after` (§2.8, §2.9).
- **Não** carregar _trust store_ do sistema (§2.5), mantendo raízes TLS via `webpki-roots` para reprodutibilidade.
- **Não** introduzir runtime assíncrono (§2.3).

---

## 3. Plano de implementação

| Fase    | Conteúdo                                                                                 | Entregável verificável                                         | Esforço `[modelado]` |
| ------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------- | -------------------- |
| **R**   | Quebra de `rhai-runtime` em `rhai-bindings`                                              | `cargo tree -p rhai-runtime` sem `dom`; suítes da v0.2 verdes  | 2–3 d                |
| **F9a** | `core/css`: tokenizador, parser de regras, seletores, especificidade                     | Recorte do `MANIFEST.md` verde; `<style>` e `style=` aplicados | 10–15 d              |
| **F9b** | Cascata real: origens, `!important`, herança, valores computados, unidades, cores        | `MockCascadeResolver` ainda troca; goldens da v0.3 mudam       | 8–12 d               |
| **F9c** | Box model, colapso de margem, contexto inline, `text-align`, Flexbox                     | Asserções de retângulo; goldens de página com CSS de autor     | 16–24 d              |
| **F8a** | `core/network`: `Url`, HTTP/1.1, chunked, pool, redirect, TLS, charset, gzip             | `run_transport_suite`; `MockTransport` serve fixture em CI     | 14–20 d              |
| **F8b** | `core/window`: eventos, `WindowSystem`/`Presenter`, `winit`, `softbuffer`, headless      | Janela abre/redimensiona/fecha nos 3 SOs; headless roda em CI  | 8–12 d               |
| **M**   | Muscle: `default_ui.rhai`, `default_network.rhai`, bindings guardados, perfis            | Script sem `NETWORK_FETCH` recebe `PermissionDenied`           | 8–12 d               |
| **X**   | `<img>`: _inflate_, decodificador PNG, `DrawImage`, sizing intrínseco, relayout          | Golden com imagem; _fuzz_ do decodificador verde               | 5–8 d                |
| **I4**  | `alloy <url>`, navegação, resize → relayout, subrecursos, `<link rel=stylesheet>`        | `alloy https://example.com` renderiza; golden fim a fim em CI  | 8–12 d               |
| **P**   | Portões: `criterion`, `unsafe-audit`, _fuzz_ de CSS, ADR-0016/0017, PRD-009/010, records | Três jobs de CI novos, bloqueantes                             | 5–8 d                |

**Ordem, e por que ela é assim.** R → (F9a → F9b → F9c ‖ F8a → F8b) → M → X → I4 → P. As trilhas B e C do roadmap
(`:251-253`) rodam em paralelo; R e P são compartilhados.

Três pontos não óbvios:

- **R é o primeiro passo, não o último.** Mover dois bridges custa 2–3 d; mover cinco custa o dobro, e a v0.5 cria os
  três restantes (§2.12). Fazer a quebra depois de M é pagar para desfazer o que M acabou de escrever no lugar errado.
- **F9 antes de M.** A cascata scriptável de `PRD-007:96-98` precisa de uma cascata Rust funcionando para ter contra o
  que ser comparada e para onde cair no _fallback_. Escrever o adaptador `.rhai` primeiro é escrever contra um alvo que
  ainda não existe.
- **X antes de I4.** O relayout disparado por chegada de imagem (§2.11) é a interação mais frágil entre rede, layout e
  pintura. Descobri-la dentro de I4, junto com navegação e resize, é misturar três fontes de defeito num só ponto.

O SPDD roda antes de cada fase, não depois (`PRD-001:100`, `ADR-0007`): `/spdd-analysis` + `/spdd-reasons-canvas` para
F9 (um canvas de parser/cascata e um de layout), F8 (um de rede e um de janela), M e I4.

**F9a — passos (10–15 d), em `core/css/`:** tokenizador de CSS Syntax L3 (3–4 d); parser de regras, declarações e
`@media` (2–3 d); `Selector` como _value object_ + parser de seletor (2–3 d); _matching_ contra o `DomSnapshot` +
especificidade + ordem de documento (2–3 d); `MANIFEST.md` do recorte e runner que falha na divergência (1–2 d).

**F9b — passos (8–12 d):** valores computados por propriedade, com `initial`/`inherit` (2–3 d); resolução de unidades
(`px`/`em`/`rem`/`%`/`pt`) e de cor (2–3 d); as três origens + `!important` + herança substituindo `ua_cascade` (3–4 d);
folha UA escrita agora em CSS de verdade, embarcada por `include_str!` (1–2 d).

**F9c — passos (16–24 d):** box model e colapso de margem (4–6 d); contexto inline — caixas de linha, `white-space`,
quebra suave, `text-align`, _baseline_ (6–8 d); Flexbox: eixos, `justify`/`align`, `grow`/`shrink`/`basis` (5–8 d);
`flex-wrap` (1–2 d, **a alavanca de §1.3**).

**F8a — passos (14–20 d):** `Url` + `HeaderMap` + `HttpRequest`/`HttpResponse` + `NetworkError` (3–4 d); `HttpTransport`

- `run_transport_suite` + `MockTransport` + feature `no-transport` (2–3 d); HTTP/1.1 sobre TCP: chunked, `keep-alive`,
  pool, redirect, timeout (4–5 d); TLS por `rustls` + provider RustCrypto + `webpki-roots` (3–5 d); charset e
  `Content-Encoding` (2–3 d).

**F8b — passos (8–12 d):** `WindowEvent` e os _value objects_ (1–2 d); `WindowSystem`/`Presenter` + `run_window_suite` +
`HeadlessWindowSystem` + feature `no-window` (2–3 d); adaptador `winit` com o mapeamento de eventos (3–4 d);
apresentador `softbuffer` + `FrameView` (2–3 d).

**M — passos (8–12 d):** perfis de capability por subsistema, um `ExecutionContext` cada (1–2 d); tabelas
`WINDOW_BINDINGS` e `NETWORK_BINDINGS` via `install_guarded_table`, na varredura de C-06 e na matriz de C-09 (3–4 d);
`default_ui.rhai` e `default_network.rhai` + o ciclo `on_init`/`on_event`/`on_process` sob `run_with_fallback` (3–4 d);
adaptador de cascata em `.rhai` de `PRD-007:96-98`, com teste de pânico caindo no Rust (1–2 d).

**X — passos (5–8 d):** _inflate_ RFC 1951 + testes de vetor (2–3 d); decodificador PNG sobre ele (1–2 d); `DrawImage`
no rasterizador com amostragem inteira (1–2 d); sizing intrínseco e o relayout disparado por chegada (1 d).

**I4 — passos (8–12 d):** `alloy <url>` e a máquina de navegação (2–3 d); busca de subrecursos — `<link rel=stylesheet>`
e `<img>` — com fila e coalescência (2–3 d); resize → re-cascata → relayout → repintura, com coalescência por quadro
(2–3 d); goldens fim a fim sobre `MockTransport` + `HeadlessWindowSystem` e o job de CI (2–3 d).

**P — passos (5–8 d):** benchmark `criterion` do hook + linha de base + portão bloqueante (1–1,5 d); job `unsafe-audit`
com `cargo-geiger` e allowlist nominal (1–1,5 d); alvos de _fuzz_ de CSS e do decodificador PNG (1 d); generalização do
`EngineError` para `Subsystem` + bump 3 → 4 + nota de migração (0,5 d); ADR-0016, ADR-0017, `PRD-009`, `PRD-010`, dois
contract records, `overview.md:92-93`, `deny.toml`, `CLAUDE.md` (1,5–2,5 d).

**Mínimo viável** (só o que o roadmap literalmente pede — F9 + F8 + I4, sem muscle, sem `<img>`, sem quebra de crate): ≈
**64–95 d**. **Escopo completo:** ≈ **84–126 dias-dev `[modelado]`**.

---

## 4. Armadilhas

| Armadilha                                                                                           | Mitigação                                                                                                                              |
| --------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Provider RustCrypto do `rustls` não existe na forma esperada, ou é incompatível com a versão fixada | Verificar **no primeiro dia da F8a**, antes de qualquer código de HTTP; a decisão de §2.5 tem um segundo lugar declarado e um custo    |
| `winit` e `softbuffer` trazem `unsafe` e contradizem a escolha de §2.5 se a regra não for escrita   | ADR-0016 primeiro, código depois: a allowlist nominal do `unsafe-audit` é o que torna a exceção revisável em vez de precedente         |
| `PRD-001:97` exige "zero unsafe" enquanto o `rhai` transmuta na costura de binding desde a v0.1     | Reescrever N-02 na fase P junto com o ADR-0016, e rodar o `unsafe-audit` na árvore **de hoje** antes de fixar qualquer dep nova (§2.1) |
| `tokio` entra pela porta dos fundos junto com um cliente HTTP pronto                                | HTTP/1.1 escrito à mão sobre `std::net`; o job `unsafe-audit` e o `cargo tree` do portão de rede tornam a entrada visível no PR        |
| Segundo event loop aparece para "tratar rede" e disputa a thread principal                          | ADR-0017 escrito na F8b, antes da F11 e da F10 (`ROADMAP-IMPLEMENTACAO-V1.md:318`); I/O em worker, resultado por canal                 |
| Colapso de margem tratado como detalhe e implementado por último                                    | É passo 1 da F9c, com asserção de retângulo própria; sem ele nenhuma página real bate                                                  |
| Golden image quebra em massa quando a cascata real substitui a folha UA da v0.3                     | Regerar as goldens é esperado e deve ser **um commit isolado**, revisável imagem a imagem, nunca misturado com mudança de layout       |
| Variação de fontes do SO diverge os testes de renderização                                          | Testes automatizados usam `FontProvider` sintético; resolução de `font-family` do sistema opera no runtime (§2.10)                     |
| Raízes de confiança do sistema tornam o teste dependente da máquina                                 | `webpki-roots` embarcado; nenhum teste toca o _trust store_ do SO                                                                      |
| I4 só verificável com internet, e o CI passa a depender de `example.com`                            | `MockTransport` com fixtures + `HeadlessWindowSystem`; a rede real fica em teste manual declarado, nunca em portão                     |
| Relayout por chegada de imagem entra em laço com a busca de subrecursos                             | Coalescência por quadro no event loop + teste com 50 imagens que exige um número **limitado** de relayouts                             |
| Decodificador PNG com pânico alcançável por página                                                  | _Fuzz_ bloqueante do `inflate` e do decodificador; `DomError`-style typed error, nenhum `unwrap` em caminho alcançável                 |
| `EngineError` ganha mais duas variantes de subsistema e o enum vira lista de crates                 | Generalizar para `Subsystem { … }` **nesta** versão, com bump 3 → 4 e nota de migração (§2.13); depois são quatro, e a v0.7 traz mais  |
| Quebra de `rhai-runtime` adiada "para não atrapalhar a F9"                                          | R é o passo 1 do plano; adiar dobra o custo e joga o retrabalho em cima de M (§2.12)                                                   |
| Recorte de CSS encolhe em silêncio para o CI ficar verde                                            | `MANIFEST.md` + runner que falha nos dois sentidos, no molde do recorte html5lib da v0.3                                               |
| `PRD-009` e `PRD-010` nunca escritos, e os ports nascem fora do contrato                            | São entregável da fase P, com os sete itens de `ADR-0011:79-105` conferidos um a um na revisão                                         |
| Repositório ainda sem `LICENSE` (`Cargo.toml:25-27`) e agora com pilha TLS                          | Levantar com os mantenedores nesta entrega; bloqueia distribuição, não código — e a v0.5 é a primeira versão distribuível              |
| `spdd/` sem canvas para F8/F9/M/I4 enquanto `PRD-001:100` os exige                                  | `/spdd-analysis` + `/spdd-reasons-canvas` antes do primeiro `/spdd-generate` de cada fase                                              |

---

## 5. Verificação

Nada aqui foi executado. Nenhum item nasce marcado.

**Automatizável em CI, nos 3 SOs (`pnpm check` + `cargo test --workspace`):**

- [ ] `cargo test -p css -p network -p window` verde; `fmt --check` e `clippy -D warnings` continuam exit 0.
- [ ] `cargo tree -p css`, `-p network` e `-p window` não mostram `engine` nem `rhai` — o portão N-04 estendido aos três
      crates novos (`PRD-001:99`).
- [ ] `cargo tree -p rhai-runtime` **não** mostra `dom` — guarda a quebra de §2.12 contra regressão.
- [ ] `cargo test -p network --no-default-features` (`no-transport`) e `-p window --no-default-features` (`no-window`) —
      os ports compilam e passam sem adaptador real linkado (`ADR-0011:99-102`).
- [ ] `run_transport_suite` passa para o cliente real **e** para `MockTransport`; `run_window_suite` para `winit` e para
      `HeadlessWindowSystem`.
- [ ] `cargo-geiger` não encontra `unsafe` em nenhum crate fora da allowlist nominal — é o portão que impede
      `encoding_rs`, `ring` ou um SIMD de conveniência de entrar sem revisão (§2.1).
- [ ] A allowlist do `unsafe-audit` contém `rhai` com comentário citando `PRD-003:21-24`, e **não** contém nenhum crate
      que decodifique bytes vindos da rede — o teste falha se alguém classificar um decodificador como "FFI de
      plataforma" para passar pelo portão.
- [ ] Benchmark `criterion` do `on_event`: p99 < 10μs (`PRD-001:96`), com a primeira execução virando linha de base.
- [ ] O recorte declarado no `MANIFEST.md` de CSS fica 100% verde, e o runner falha se código e manifesto divergirem em
      qualquer um dos dois sentidos.
- [ ] Asserções de retângulo cobrem colapso de margem, `box-sizing`, quebra de linha e cada propriedade de Flexbox — o
      teste falha se alguém "arredondar" a especificação por conveniência.
- [ ] Um adaptador de cascata em `.rhai` altera uma propriedade computada e a golden muda, com capability limitada a
      `DOM_READ | GRAPHICS_DRAW` — fecha `PRD-007:96-97`.
- [ ] Esse mesmo adaptador, ao entrar em pânico, cai no resolvedor Rust embutido e a página **ainda renderiza** —
      `PRD-007:98` e `PRD-003:62-70`.
- [ ] Script de UI sem `NETWORK_FETCH` recebe `EngineError::PermissionDenied` ao tentar buscar — estende C-06/C-07 ao
      terceiro e quarto subsistema.
- [ ] A matriz de injeção de pânico cobre `WINDOW_BINDINGS` e `NETWORK_BINDINGS` (C-09 fora do DOM e do display list).
- [ ] Golden fim a fim: uma página servida por `MockTransport` renderiza pixel a pixel idêntica nos três SOs, com CSS de
      autor, texto e imagem.
- [ ] Redimensionar a superfície headless de 800 para 1024 refaz o layout e a golden de 1024 bate — e o contador de
      relayouts mostra **um**, não um por evento de resize.
- [ ] 100 renderizações da mesma entrada produzem `LayoutBoxTree` e framebuffer idênticos (`PRD-007:100`).
- [ ] Chegada de 50 imagens dispara um número limitado de relayouts, não 50 — guarda a coalescência de §2.11.
- [ ] `cargo-fuzz` no parser de CSS, no `inflate` e no decodificador PNG: zero pânicos em 10 min por alvo, bloqueante.
- [ ] Resposta HTTP maliciosa — `Content-Length` mentiroso, chunk inválido, cabeçalho gigante, redirect em ciclo —
      devolve `NetworkError` tipado e nunca _hang_ nem pânico.
- [ ] Cobertura de `domain/` ≥ 85% para `css`, `network` e `window`.

**Só com display e rede reais (não verificável no runner comum de CI):**

- [ ] `alloy https://example.com` abre janela nativa e renderiza a página real, em Linux, macOS e Windows.
- [ ] Handshake TLS com o provider RustCrypto completa contra um conjunto de sites reais — é a validação que o portão de
      `MockTransport` **não** faz, por construção.
- [ ] Redimensionar a janela com o mouse refaz o layout sem piscar e sem _tearing_ visível.

**Não verificável nesta fase (declarado):**

- [ ] Hot-reload de `default_ui.rhai` sem recompilar — F11/v0.9; os scripts são embarcados por `include_str!`.
- [ ] `VulkanBackend`/`OpenGLBackend` na janela (C-15/C-16) — F12/v0.9.
- [ ] Qualquer coisa de `<script>`, `Origin` ou isolamento por aba — F7 e F10, v0.7.

---

## 6. Riscos

1. **A pilha TLS é o risco de maior variância da v0.5, e é um risco de disponibilidade, não de esforço.** O provider
   RustCrypto do `rustls` é o caminho menos trilhado da decisão 1, e **este relatório não verificou que ele existe na
   forma e na versão necessárias** — a checagem é o primeiro passo da F8a, deliberadamente, para que a alternativa (o
   provider padrão sob exceção registrada de §2.1) possa ser retomada com dias de custo, não semanas.

2. **F9c é a fase mais longa e a que mais convida a "quase certo".** Colapso de margem, contexto inline e Flexbox são
   três fontes independentes de caso de borda, e nenhuma admite aproximação: uma margem que não colapsa desloca a página
   inteira. A alavanca declarada é `flex-wrap` (§2.9), e ela sai por decisão registrada no `MANIFEST.md`.

3. **A v0.5 é a primeira versão que a tese do produto pode falhar em público.** Até aqui, "política em script" é
   afirmação de ADR. A partir de M, ela é um número: se o _round-trip_ de `on_event` não couber em 10μs com o volume de
   eventos de uma janela real, a resposta correta é reduzir a granularidade das travessias — não afrouxar o portão. O
   portão existe para forçar essa conversa cedo.

4. **Quatro dependências externas novas numa árvore que hoje tem duas.** `Cargo.toml:33-40` lista `bitflags` e `rhai`; a
   v0.5 acrescenta `winit`, `softbuffer`, `rustls` e o provider, mais `webpki-roots` — e a v0.3 já traz `ttf-parser`. É
   o salto que `ROADMAP-IMPLEMENTACAO-V1.md:334` antecipa como razão de existir do `cargo-deny`, e a primeira vez que a
   auditoria de supply-chain protege algo real.

5. **`overview.md` já documenta o que a v0.5 vai contradizer.** `:92-93` diz `window → graphics, engine` e
   `network → engine`; as duas ficam falsas por decisão consciente (§1.4, §2.7). Corrigir dentro da fase que as
   contradiz é a única disciplina que impede a documentação de virar ficção — é o risco §6 do roadmap (`:425-427`), e a
   v0.5 é a terceira versão seguida a herdá-lo.

6. **A papelada de dois ports novos é subestimável exatamente como foi na v0.3.** `PRD-009`, `PRD-010`, dois contract
   records, duas suítes de conformidade, dois adaptadores de referência e duas _features_ `no-*`. O risco não é o prazo
   da fase P: é os contract records nunca serem escritos e o `ADR-0011` perder autoridade por precedente.

7. **A regra de `unsafe` do §2.1 é uma decisão de produto disfarçada de decisão técnica.** Se a resposta for "o Alloy
   não embarca `unsafe` de terceiros, ponto", então a v0.5 **não tem janela** — a única alternativa que de fato elimina
   `unsafe` sem custar Windows e macOS é ficar headless e empurrar F8b para a v0.7 (§2.1). Isso é defensável: a v0.5
   entregaria rede + CSS real + `alloy render <url> -o out.png`, que já é a maior parte do valor, e a janela viria com o
   hot-reload da v0.9 quando houver mais razão para uma. O que **não** é defensável é manter `PRD-001:97` escrito como
   está, com `rhai` transmutando na costura de binding desde a v0.1 e ninguém tendo verificado. A decisão precisa ser
   tomada antes da F8b, não durante.

---

## 7. Arquivos tocados

| Arquivo                                                                                                  | Mudança                                                                                                     |
| -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `Cargo.toml:33-40`                                                                                       | `[workspace.dependencies]` += `winit`, `softbuffer`, `rustls`, provider RustCrypto, `webpki-roots`          |
| `core/runtime/rhai/Cargo.toml:10-16`                                                                     | **Remove** `dom`; volta a depender só de `engine` + `rhai`                                                  |
| `core/runtime/rhai/src/infrastructure/dom_bindings.rs`                                                   | **movido** para o crate novo                                                                                |
| `core/runtime/rhai-bindings/`                                                                            | **novo crate** — `dom_bindings`, `display_list_bindings`, `css_bindings`, `net_bindings`, `window_bindings` |
| `core/css/Cargo.toml`                                                                                    | Features `builtin-adapters` (default) e `no-script`; sem `engine`                                           |
| `core/css/src/infrastructure/parser/`                                                                    | **novo** — tokenizador CSS L3, parser de regras, `@media`                                                   |
| `core/css/src/domain/selector.rs`, `specificity.rs`                                                      | **novo** — `Selector`, `Specificity`, _matching_ contra `DomSnapshot`                                       |
| `core/css/src/infrastructure/cascade.rs`                                                                 | **novo** — origens, `!important`, herança; substitui o `ua_cascade` da v0.3                                 |
| `core/css/src/infrastructure/layout/` (`block.rs`, `inline.rs`, `flex.rs`)                               | **novo** — box model, contexto inline, Flexbox; substitui `block_layout.rs`                                 |
| `core/css/assets/ua.css`, `core/css/tests/data/MANIFEST.md`                                              | **novo** — folha UA em CSS real; recorte declarado + runner                                                 |
| `core/network/Cargo.toml`                                                                                | + `rustls`, provider, `webpki-roots`; features `real-transport` (default) e `no-transport`                  |
| `core/network/src/domain/`                                                                               | **novo** — `Url`, `HeaderMap`, `HttpRequest`/`HttpResponse`, `StatusCode`, `NetworkError`                   |
| `core/network/src/application/` (`ports.rs`, `conformance.rs`)                                           | **novo** — `HttpTransport`, `RequestPolicy`, `run_transport_suite`                                          |
| `core/network/src/infrastructure/` (`http1.rs`, `tls.rs`, `pool.rs`, `charset.rs`, `inflate.rs`)         | **novo** — cliente HTTP/1.1, TLS, pool, charset, _inflate_ RFC 1951                                         |
| `core/window/Cargo.toml`                                                                                 | + `winit`, `softbuffer`; features `winit-system` (default) e `no-window`. **Sem** `graphics`                |
| `core/window/src/domain/`                                                                                | **novo** — `WindowEvent`, `SurfaceSize`, `FrameView`, `ScaleFactor`, `WindowError`                          |
| `core/window/src/application/` (`ports.rs`, `conformance.rs`)                                            | **novo** — `WindowSystem`, `Presenter`, `run_window_suite`, `HeadlessWindowSystem`                          |
| `core/window/src/infrastructure/` (`winit_system.rs`, `softbuffer_presenter.rs`)                         | **novo** — adaptadores; nenhum tipo de `winit` em assinatura pública                                        |
| `core/graphics/src/infrastructure/software/image.rs`                                                     | **novo** — `DrawImage` com amostragem de caixa em inteiros                                                  |
| `core/graphics/src/infrastructure/png_decode.rs`                                                         | **novo** — decodificador PNG sobre o _inflate_                                                              |
| `core/engine/src/domain/error.rs:65`                                                                     | `Subsystem { subsystem, operation, reason }` generalizando `Dom`/`Graphics`; schema 3 → 4                   |
| `alloy/src/application/` (`navigation.rs`, `subresource.rs`, `event_loop.rs`)                            | **novo** — máquina de navegação, fila de subrecursos, laço único da thread principal                        |
| `alloy/src/application/paint.rs`                                                                         | Inalterado — a promoção prometida na v0.3 é **reavaliada na F10** (§2.14)                                   |
| `scripts/default_ui.rhai`, `scripts/default_network.rhai`, `scripts/cascade.rhai`                        | **novo** — os três adaptadores de política embarcados                                                       |
| `alloy/tests/`, `core/css/tests/`, `core/network/tests/`, `core/window/tests/`                           | **novo** — goldens fim a fim, asserções de retângulo, conformidade, resposta HTTP maliciosa                 |
| `core/runtime/rhai/benches/hook_overhead.rs`                                                             | **novo** — `criterion`, p99 do `on_event`                                                                   |
| `fuzz/fuzz_targets/` (`css_parse.rs`, `inflate.rs`, `png_decode.rs`)                                     | **novo** — três alvos                                                                                       |
| `.github/workflows/ci.yml`                                                                               | **novo** — jobs `hook-benchmark`, `unsafe-audit`, `css-conformance`; `no-engine` estendido                  |
| `deny.toml:19-23`                                                                                        | Licenças da pilha TLS e de `winit`/`softbuffer`                                                             |
| `docs/adr/0016-…` (`unsafe` por superfície de ameaça), `docs/adr/0017-…` (event loop único), `README.md` | **novo** — dois MADRs + linhas no índice (0012 segue reservado, `ADR-0011:128`)                             |
| `docs/requirements/PRD-009-…`, `PRD-010-…`                                                               | **novo** — os dois seam PRDs dos ports de rede e de janela                                                  |
| `docs/requirements/PRD-002-…`                                                                            | Nota de migração do `PORT_SCHEMA_VERSION` 3 → 4 (`EngineError::Subsystem`)                                  |
| `docs/requirements/PRD-001-…:97`                                                                         | **Reescrita de N-02** — "zero unsafe" nunca foi verdade (§2.1); vira o critério por superfície de ameaça    |
| `docs/architecture/http-transport-port-contract.md`, `window-system-port-contract.md`                    | **novo** — os dois contract records dos sete itens                                                          |
| `docs/architecture/overview.md:92-93`                                                                    | `window` e `network` sem `engine`; `window` sem `graphics` (§1.4, §2.7)                                     |
| `CLAUDE.md`, `docs/README.md:25-29`                                                                      | "Current State" reescrito; linha da v0.5 na árvore de `reports/`                                            |
| `spdd/analysis/`, `spdd/prompt/`                                                                         | **novo** — canvases de F8, F9, M e I4 (`PRD-001:100`)                                                       |

---

> Nenhuma linha deste plano foi implementada, e a sua premissa — a v0.3 entregue — **não é o estado do repositório**:
> `core/css`, `core/window` e `core/network` continuam com as 16 linhas do stub `add()` no commit `6536bbc` do branch
> `feat/v0-2-implementation`, como a busca de §1.1 mostra. O que **foi** feito nesta rodada: leitura de
> `ROADMAP-IMPLEMENTACAO-V1.md`, `IMPLEMENTACAO-DETALHADA-V0-3.md`, `PRD-001`, `PRD-003`, `PRD-007`, `ADR-0011`,
> `docs/architecture/overview.md`, `docs/adr/README.md`, e inspeção direta de `Cargo.toml`,
> `core/runtime/rhai/Cargo.toml`, `alloy/Cargo.toml`, `core/engine/src/domain/capability.rs`,
> `core/engine/src/domain/error.rs`, `core/engine/src/lib.rs`, `core/runtime/rhai/src/infrastructure/sandbox.rs`,
> `deny.toml` e `.github/workflows/ci.yml`. O achado de §2.1 (quatro `unsafe` no `rhai`, três deles na costura de
> binding) **foi verificado** por `grep` sobre `rhai-1.26.0` no registry local, e `bitflags-2.13.1:273` foi conferido
> como `forbid(unsafe_code)`; as propriedades de `breadx`, `x11rb` e `wayland-client` vêm de busca na documentação
> pública desses projetos, não de inspeção do código. **Não verificado**: a existência, a versão e a completude de um
> `CryptoProvider` RustCrypto para `rustls` (risco §6.1); as versões a fixar de `winit`, `softbuffer` e `webpki-roots`;
> se `png`/`miniz_oxide` passariam no portão `unsafe-audit` de §2.1; se `cargo-geiger` roda sob a toolchain fixada em
> 1.97.1; e o `unsafe` das demais dependências transitivas da árvore atual, que só o próprio portão resolve. Os esforços
> em dias-dev são `[modelado]`; os blocos que existem no roadmap reaproveitam `ROADMAP-IMPLEMENTACAO-V1.md:265-266`, e
> os demais não têm velocidade histórica para calibrá-los.
