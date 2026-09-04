# SPDD Analysis — v0.5 C1 (`core/network`): HTTP transport and request-policy ports

| Campo        | Valor                                                                                                                                                                          |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Fase         | C1 (F8a do `docs/reports/ROADMAP-IMPLEMENTACAO-V1.md:251-253`, trilha C), spec em `~/.claude/plans/…-fancy-dijkstra.md` "## Fase C1"                                           |
| Fecha        | os critérios da trilha de rede — `run_transport_suite` + `run_policy_suite` verdes, resposta HTTP maliciosa → erro tipado (tabela de despacho, linha **C1**)                   |
| Port         | `HttpTransport` (mecanismo) + `RequestPolicy` (política) — seam PRD **PRD-009** (`docs/requirements/README.md:33`, **arquivo ainda não existe** — criado na Fase P)            |
| Depende de   | **C0** (spike TLS, este documento par) · fundação (`bitflags`, `thiserror`) · **nada** de `engine`/`rhai`/`dom` (`arch-lint.toml`, job `layering`)                             |
| Estado atual | `core/network/src/lib.rs` tem 8 linhas — doc-comment e `#![forbid(unsafe_code)]`, zero funções (`core/network/src/lib.rs:1-8`); `core/network/Cargo.toml` sem `[dependencies]` |
| Precede      | **EE** (`SubsystemName::Network` precisa do domínio de C1), **X** (`inflate` re-exportado de `network`), **M** (`NETWORK_BINDINGS`), **I4** (`alloy <url>`)                    |

## Original Business Requirement

Spec da fase, verbatim de `~/.claude/plans/verifique-o-docs-reports-implementacao-d-fancy-dijkstra.md` "## Fase C1"
(linhas 517-556):

```text
- Domain: core/network/src/domain/{url.rs, header_map.rs (first-class collection), request.rs,
  response.rs, status.rs, method.rs, media_type.rs, error.rs, mod.rs}. NetworkError
  #[non_exhaustive], Display à mão, localização = ProtocolPhase { Dns, Connect, Handshake,
  Header, Body, Redirect, Decode }.
- Application: ports.rs, conformance.rs:
      pub trait HttpTransport: Send + Sync {
          fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError>;
      }
      pub trait RequestPolicy: Send + Sync {
          fn decide(&self, request: &HttpRequest) -> Result<PolicyVerdict, NetworkError>;
      }
      pub enum PolicyVerdict { Allow, Rewrite(HttpRequest), Deny { reason: String } }
  Ambas object-safe; nenhum tipo rustls/std::net em assinatura.
  run_transport_suite(&dyn HttpTransport) + run_policy_suite(&dyn RequestPolicy).
- Infrastructure: http1/{mod.rs, chunked.rs, pool.rs}, tls.rs, dns.rs, charset.rs, inflate.rs,
  redirect.rs, mock.rs, mod.rs:
  - HTTP/1.1 sobre std::net::TcpStream: request line, headers, Content-Length,
    Transfer-Encoding: chunked, pool keep-alive por (host, porta), redirect (limite 20 +
    detecção de ciclo), timeout por fase.
  - DNS = wrapper fino de std::net::ToSocketAddrs (sem cliente DNS).
  - TLS = rustls + provider (C0) + webpki-roots embarcado.
  - charset.rs — UTF-8 (std, U+FFFD em inválido) + windows-1252 (tabela de 256). Ordem: BOM →
    charset do Content-Type → <meta charset> nos primeiros 1024 bytes → windows-1252.
  - inflate.rs — RFC 1951 à mão (~300 linhas), compartilhado com a Fase X.
  - MockTransport — serve HttpResponse de fixture; AllowAllPolicy.
- Feature: default = ["real-transport"]; no-transport (portas + MockTransport + domain).
- Ciclo de vida (ADR-0019): trait síncrona; event loop roda execute num worker de pool
  std::thread, resultado por std::sync::mpsc.
- Deps de workspace: rustls, provider, webpki-roots (todas =-pinadas).
```

Entregável, verbatim (linhas 553-556):

```text
run_transport_suite verde para o cliente real (manual) e MockTransport (CI); resposta HTTP
maliciosa (Content-Length mentiroso, chunk inválido, header gigante, redirect em ciclo) →
NetworkError tipado, nunca hang/pânico; cargo tree -p network sem engine/rhai;
--no-default-features verde.
```

Correções ao spec que se aplicam a C1 (`~/.claude/plans/…-fancy-dijkstra.md:36-38`):

```text
NetworkError usa #[derive(thiserror::Error)] + #[error("…")] — o Display à mão é carve-out SÓ
de core/engine (ADR-0015). core/dom e core/graphics já usam thiserror; os crates novos seguem
esse caminho. (O texto antigo da fase C1 dizia "Display à mão" — ignorar.)
```

Contrato de porta substituível — os 7 itens de `ADR-0011` para `core/network`, verbatim de
`~/.claude/plans/…-fancy-dijkstra.md:737-747`:

```text
1. Seam PRD-009; ameaça = servidor hostil; unsafe proibido em byte de socket.
2. Traits object-safe; sem tipo rustls/TcpStream/webpki em assinatura.
3. Url/HeaderMap/HttpRequest/HttpResponse/StatusCode/Method/MediaType #[non_exhaustive];
   network::PORT_SCHEMA_VERSION = 1.
4. NetworkError #[non_exhaustive], localização = ProtocolPhase.
5. http-transport-port-contract.md: trait síncrona; execute em worker de pool; HttpResponse por
   mpsc; pool (host,porta), timeouts por fase, redirect 20 + ciclo.
6. run_transport_suite + run_policy_suite; referência = cliente HTTP/1.1 à mão; mock =
   MockTransport + AllowAllPolicy; feature no-transport.
7. Congela em I4.
```

## Domain Concept Identification

### Existing Concepts (from codebase)

- **Padrão de porta substituível `ADR-0011`** — `core/graphics` é o exemplar completo: `Cargo.toml` com `[features]`
  (`default`/`no-backend`), `lib.rs` com `pub const PORT_SCHEMA_VERSION: u32 = 1`
  (`spdd/prompt/202609031011-[Feat]-graphics-display-list-and-render-backend-v0-3-f4.md:172`), `application/ports.rs`
  object-safe, `application/conformance.rs::run_backend_suite` como código `pub` de biblioteca (não `#[cfg(test)]`),
  adaptador de referência in-repo. `core/network` copia essa forma inteira.
- **`thiserror` fora de `core/engine`** (`ADR-0015`, `core/dom/src/domain/error.rs`) — `NetworkError` usa
  `#[derive(thiserror::Error)]` + `#[error("…")]`, **não** o `Display` à mão do `core/engine`
  (`~/.claude/plans/…-fancy-dijkstra.md:36`).
- **`ProtocolPhase` ≈ `SourceLocation`** — o modelo "erro tipado com metadado de localização" do `ADR-0011` item 4; em
  `core/graphics` a "linha" é `CommandIndex`
  (`spdd/analysis/202609031011-[Analysis]-graphics-display-list-and-render-backend-v0-3-f4.md:86`), em `core/network` é
  a fase de protocolo (`Dns`/`Connect`/`Handshake`/`Header`/`Body`/`Redirect`/`Decode`).
- **First-class collections** (`ADR-0010:131-133`, `Children`/`AttributeMap` em `core/dom`) — `HeaderMap` segue o mesmo
  molde: sem `Vec`/`HashMap` público, ordem de inserção preservada, acesso por método validador.
- **Feature `no-<adapter>` + adaptador de referência + mock** (`ADR-0011` item 6; `core/graphics` estreou `[features]`
  no workspace, `spdd/prompt/202609031011-…-f4.md:250`) — `core/network` ganha `real-transport` (default) e
  `no-transport` (`MockTransport` + `AllowAllPolicy` + domínio).
- **Job `layering`** (`justfile` `no-engine`, `.github/workflows/ci.yml`) — já roda `cargo tree -p network` e
  `cargo test -p network --no-default-features` (Fase 0 os adicionou ao spec do job); C1 tem de mantê-los verdes.
- **`ADR-0019` — event loop único dono da thread principal**
  (`docs/adr/0019-single-event-loop-owns-the-main-thread.md:43-49`) — I/O bloqueante (DNS, TCP connect, handshake TLS,
  leitura de corpo) roda num worker de pool `std::thread`, resultado por `std::sync::mpsc`; `HttpTransport` é
  **síncrona**, sem runtime assíncrono.
- **`ADR-0018` — `unsafe` por superfície de ameaça** (`docs/adr/0018-unsafe-by-threat-surface.md:62-67`) — bytes de
  socket TLS/HTTP = linha 1 = `unsafe` de terceiros **proibido**; a carve-out RustCrypto/`ring` é a exceção explícita
  (`docs/adr/0018-unsafe-by-threat-surface.md:78-82`).
- **`engine::capability::profiles::network_interceptor()`** já existe (`~/.claude/plans/…-fancy-dijkstra.md:40`) — a
  Fase M usa esse perfil; C1 **não** cria capability nova (e `core/network` nem referencia `engine`).

### New Concepts Required

- **`HttpTransport`** — a porta de **mecanismo**: `HttpRequest` entra, `HttpResponse` sai ou `NetworkError`. Não conhece
  `rustls` nem `std::net` na assinatura (`ADR-0011` item 2).
- **`RequestPolicy` + `PolicyVerdict`** — a porta de **política**: `Allow` / `Rewrite(HttpRequest)` / `Deny { reason }`.
  É o ponto onde o muscle script da Fase M intercepta (`network_interceptor()`), sem que C1 dependa de `engine`.
- **`Url`** — VO validado (esquema, host, porta, caminho, query); a fronteira que impede um caminho/host malformado de
  virar `TcpStream::connect`.
- **`HeaderMap`** — first-class collection, ordem de inserção, nomes case-insensitive, valor sem CR/LF (defesa contra
  response splitting).
- **`HttpRequest` / `HttpResponse`** — agregados de fronteira `#[non_exhaustive]`, versionados por
  `network::PORT_SCHEMA_VERSION = 1`. `HttpResponse` carrega bytes de corpo **já decodificados** (charset +
  content-encoding).
- **`StatusCode` / `Method` / `MediaType`** — VOs `#[non_exhaustive]`; `MediaType` separa `type/subtype` + parâmetro
  `charset`.
- **`NetworkError` / `ProtocolPhase`** — o erro único da porta e seu metadado de fase. Toda resposta hostil vira uma
  variante tipada, nunca hang nem pânico.
- **`ProtocolPhase`** — `{ Dns, Connect, Handshake, Header, Body, Redirect, Decode }`.
- **`Http1Transport`** — o adaptador de referência: HTTP/1.1 sobre `std::net::TcpStream` + `rustls` (`ring`, C0) +
  `webpki-roots`, com pool `keep-alive` por `(host, porta)`, redirect (limite 20 + ciclo), timeout por fase.
- **`MockTransport` / `AllowAllPolicy`** — o adaptador in-repo de `ADR-0011` item 6; `MockTransport` serve
  `HttpResponse` de fixture (`core/network/tests/fixtures/`).
- **`Inflate`** (RFC 1951 à mão) — descompressor `deflate`/`gzip`; re-exportado pela Fase X para o decodificador PNG
  (`~/.claude/plans/…-fancy-dijkstra.md:632-636`), então nasce como módulo público reutilizável.
- **`Charset`** — decodificador UTF-8 (`U+FFFD` em inválido) + windows-1252 (tabela de 256); ordem de detecção BOM →
  `Content-Type` → `<meta charset>` (primeiros 1024 bytes) → windows-1252.
- **`ConnectionPool`** — cache de `TcpStream` por `(host, porta)`; nunca altera o resultado, só evita re-handshake.

### Key Business Rules

- **Nenhum `unsafe` de terceiros no caminho de bytes de socket** (`ADR-0018:64`, linha 1). Consequência do spike C0: não
  existe pilha TLS `unsafe`-free; `ring` entra como **exceção linha-1 registrada** em `unsafe-allowlist.toml` +
  `ADR-0018` + PRD-009. Governa `tls.rs`, `Cargo.toml`, `unsafe-allowlist.toml`.
- **Uma resposta hostil nunca trava nem entra em pânico — sempre vira `NetworkError` tipado.** `Content-Length`
  mentiroso, chunk inválido, header gigante, redirect em ciclo, corpo infinito: cada um tem uma variante e uma
  `ProtocolPhase`. Governa `http1/`, `redirect.rs`, `NetworkError`.
- **Nenhum tipo `rustls` / `std::net` / `webpki` cruza a assinatura da porta** (`ADR-0011` item 2). Governa `ports.rs`,
  `HttpRequest`, `HttpResponse`.
- **`HttpTransport` é síncrona; o bloqueio mora num worker de pool** (`ADR-0019:45-48`). A porta não é `async`, não puxa
  `tokio`. Governa `ports.rs` e o contract record.
- **`HttpResponse` entrega bytes decodificados uma vez só.** Charset e `Content-Encoding` são resolvidos dentro do
  transporte; o consumidor (`core/html`) recebe UTF-8. Governa `charset.rs`, `inflate.rs`.
- **O pool nunca muda o resultado.** Uma resposta servida de conexão reusada é byte-idêntica a uma de conexão nova.
  Governa `ConnectionPool`.
- **Política e mecanismo são portas distintas.** `RequestPolicy::decide` roda **antes** de `HttpTransport::execute`; um
  `Deny` nunca abre socket. Governa `ports.rs` e o laço de `alloy` (I4).
- **`--no-default-features` compila e testa** só com domínio + `MockTransport` + `AllowAllPolicy` (`ADR-0011` item 6).
  Governa `[features]` e o job `layering`.

## Strategic Approach

### Solution Direction

`core/network` é um crate de domínio puro em três camadas (`ADR-0010:54-74`) que recebe um `HttpRequest` e devolve um
`HttpResponse` decodificado, sem conhecer `engine`, `rhai` ou `dom`. O fluxo é:

```text
RequestPolicy::decide → (Allow) → HttpTransport::execute → DNS → TCP
  → TLS (rustls + ring) → HTTP/1.1 → dechunk → inflate → charset → HttpResponse
```

As duas portas são object-safe desde a assinatura; `Http1Transport` é a referência contra a qual um transporte
alternativo (proxy, HTTP/2, cache) se provará; `MockTransport` mantém a CI determinística sem rede. O bloqueio de I/O é
empurrado para um worker de pool `std::thread` pelo consumidor (`alloy`, Fase I4), nunca pela porta.

### Key Design Decisions

- **TLS: `rustls =0.23.43` + `ring =0.17.14` + `webpki-roots =1.0.9`, provider `ring` sob carve-out `ADR-0018`
  linha 1.** Trade-off — `ring` traz assembly + C `unsafe` no caminho que descriptografa registros TLS; é exatamente o
  que a linha 1 proíbe para terceiros. → **Decisão do spike C0** (`docs/reports/SPIKE-C0-TLS-PROVIDER.md` §6): o
  provider RustCrypto puro (`rustls-rustcrypto`) é **NO-GO** — só existe `0.0.2-alpha` (sem release estável), força um
  `rustls-webpki` duplicado que o `deny.toml` (`bans.multiple-versions = "deny"`, `deny.toml:42`) proíbe, e **não é**
  `unsafe`-free (intrínsecos AES-NI/SHA/dalek). `ring` concentra o `unsafe` numa crate de linhagem FIPS, amplamente
  auditada, com 26 crates na árvore contra 105. `rustls-webpki` e `untrusted` (os que parseiam bytes do certificado)
  foram verificados **sem `unsafe`** no `src/`.
- **`webpki-roots` embarcado, não o trust store do SO.** Trade-off — o conjunto de CAs é atualizado por bump de
  dependência, não pelo SO. → Adotado (`~/.claude/plans/…-fancy-dijkstra.md:124`): determinismo cross-OS e nenhuma
  dependência de FFI de plataforma para ler o trust store; o preço é revisar `webpki-roots` a cada release.
- **HTTP/1.1 à mão sobre `std::net`, sem `hyper`/`reqwest`.** Trade-off — ~800 linhas de parsing de framing escritas à
  mão, superfície de teste grande. → Adotado: `hyper` puxa `tokio` (contra `ADR-0019:49`, "sem runtime assíncrono") e
  `h2`; o framing HTTP/1.1 é pequeno e o portão de fuzz (`~/.claude/plans/…-fancy-dijkstra.md:771`) cobre o parser.
- **`inflate` RFC 1951 à mão, módulo público.** Trade-off — reimplementar Huffman + LZ77 (~300 linhas) em vez de
  `flate2`/`miniz_oxide`. → Adotado: a Fase X re-exporta `network::inflate` para o decodificador PNG
  (`~/.claude/plans/…-fancy-dijkstra.md:632`), evitando uma crate nova; `flate2` default puxa `miniz_oxide` (que tem
  `unsafe`) ou `zlib` (C) — ambos caem na linha 1 do `ADR-0018`.
- **Duas portas (`HttpTransport` mecanismo, `RequestPolicy` política), não uma.** Trade-off — mais superfície de trait.
  → Adotado: espelha `RenderBackend` (mecanismo) vs. o muscle script (política) do `ADR-0003` "Skeleton and Muscle"; a
  Fase M pluga um `RequestPolicy` `.rhai` sem tocar o transporte.
- **`NetworkError` com `thiserror`, não `Display` à mão.** Trade-off — diverge do texto antigo da fase. → Corrigido em
  `~/.claude/plans/…-fancy-dijkstra.md:36`: `Display` à mão é carve-out só de `core/engine` (`ADR-0015`); `core/dom` e
  `core/graphics` já usam `thiserror`.
- **`ProtocolPhase` como enum fechado-por-agora `#[non_exhaustive]`.** Trade-off — HTTP/2 (F8 later) adiciona fases. →
  Adotado: `#[non_exhaustive]` permite crescer sem bump de schema maior; congela em I4
  (`~/.claude/plans/…-fancy-dijkstra.md:747`).
- **Adaptador `.rhai` de política mora em `rhai-bindings`, nunca em `core/network`.** → `no-transport` é trivialmente
  satisfeito (mesma lógica que `core/css` `no-script`, `~/.claude/plans/…-fancy-dijkstra.md:391-394`).

### Alternatives Considered

- **`rustls-rustcrypto` (provider RustCrypto puro)** — rejeitado pelo spike C0. `0.0.2-alpha`, sem release estável desde
  2024; conflito `rustls-webpki 0.102` ↔ `0.103` proibido pelo `deny.toml`; `aes`/`sha2`/`curve25519-dalek` com `unsafe`
  de intrínseco. Ver `docs/reports/SPIKE-C0-TLS-PROVIDER.md` §2.1.
- **`aws-lc-rs` (provider default do `rustls`)** — viável (pré-autorizado no `ADR-0018:81`), mas build mais pesado
  (cmake; NASM no Windows) para a matriz de 3 SOs, sem vantagem de `unsafe` sobre `ring`. Reservado para se FIPS virar
  requisito.
- **`native-tls` / SChannel / Secure Transport** — rejeitado: FFI de TLS de plataforma processa bytes hostis (linha 1 do
  `ADR-0018`, pior que `ring`) e quebra o determinismo cross-OS do handshake.
- **`ureq` / `attohttpc` (clientes HTTP/1.1 síncronos prontos)** — rejeitado: trazem sua própria pilha `rustls` e
  política de redirect/charset, o oposto de uma porta substituível; `core/network` **é** o cliente, não o embrulha.
- **Runtime assíncrono (`tokio` + `hyper`)** — rejeitado por `ADR-0019`
  (`docs/adr/0019-single-event-loop-owns-the-main-thread.md:33-37`): briga com a posse da thread principal pelo `winit`
  e arrasta um executor por cada hook síncrono.
- **Cliente DNS próprio (`hickory-dns`)** — rejeitado: `~/.claude/plans/…-fancy-dijkstra.md:540` manda usar
  `std::net::ToSocketAddrs` (wrapper fino, sem cliente DNS) na v0.5.

## Risk & Gap Analysis

### Requirement Ambiguities

- **PRD-009 não existe.** `docs/requirements/README.md:33` reserva o número para "Porta de transporte HTTP + política de
  requisição (`core/network`)", mas o arquivo só é escrito na **Fase P** (`~/.claude/plans/…-fancy-dijkstra.md:728`). C1
  escreve o código contra a spec da fase e os 7 itens do `ADR-0011` transcritos acima; a análise formal de
  requisitos/ameaça entra no PRD-009 depois. **Colisão de número** com o branch `docs/benchmark-harness-prd-009`
  resolvida: rede fica com PRD-009, benchmark renumera para PRD-011 (`docs/requirements/README.md:40-46`).
- **"provider (C0)" na spec** — resolvido pelo spike: é `ring =0.17.14` via `rustls::crypto::ring::default_provider()`,
  **não** um `CryptoProvider` RustCrypto. Ver `docs/reports/SPIKE-C0-TLS-PROVIDER.md` §6.
- **`PolicyVerdict::Deny { reason: String }`** usa `String` cru na spec — checar contra a regra "no naked primitives in
  domain models" (`ADR-0010:129`). Resolução sugerida: `Deny { reason: DenyReason }` com um VO, ou manter `String` e
  documentar como mensagem-humana de diagnóstico (precedente: `EngineError` reason fields).
- **Timeout "por fase"** não tem valores na spec. Resolução: um `TransportConfig` com defaults documentados (connect /
  handshake / header / body-idle), nunca um booleano, nunca hard-coded no laço.
- **`MediaType` vs. `Content-Type` completo** — a spec lista `media_type.rs` mas não diz se guarda parâmetros além de
  `charset`. Resolução: `type`, `subtype`, `charset: Option<Charset>`; outros parâmetros ignorados na v0.5.

### Edge Cases

- **`Content-Length` e `Transfer-Encoding: chunked` ambos presentes** — RFC 9112 manda ignorar `Content-Length`; testar
  que não há confusão de framing (request smuggling).
- **Header sem `:`, header com CR sem LF, header de 1 MB** — cada um → `NetworkError { phase: Header }`, nunca
  realocação sem limite.
- **Redirect para si mesmo / ciclo A→B→A / 21 redirects** — limite 20 + detecção de ciclo por conjunto de URLs visitadas
  → `NetworkError { phase: Redirect }`.
- **`chunked` com tamanho de chunk não-hex, chunk final sem `0\r\n\r\n`, chunk maior que o declarado** → erro tipado.
- **Corpo sem `Content-Length` e sem `chunked` em resposta `keep-alive`** — ambíguo; fechar conexão e ler até EOF só se
  `Connection: close`, senão erro.
- **`gzip` truncado / stream `deflate` inválido / bomba de descompressão (ratio 1000:1)** — `inflate` com teto de saída
  → `NetworkError { phase: Decode }`.
- **`<meta charset>` depois dos primeiros 1024 bytes, ou charset desconhecido (`Shift_JIS`)** — cai para windows-1252
  (spec) ou erro tipado para rótulo não suportado (`~/.claude/plans/…-fancy-dijkstra.md:544`).
- **DNS resolve para `127.0.0.1` / `::1` / IP privado** — a `RequestPolicy` é quem decide bloquear SSRF; o transporte
  não presume. Documentar que `AllowAllPolicy` **não** é segura para conteúdo não confiável.
- **TLS: certificado expirado, hostname que não bate, cadeia incompleta, TLS 1.1** — `rustls` + `webpki-roots` recusa →
  `NetworkError { phase: Handshake }`; testar que a mensagem não vaza detalhe explorável.
- **Conexão do pool morta pelo servidor entre requests** — detectar no primeiro `write`/`read` e reconectar uma vez,
  transparente.

### Technical Risks

- **⚠️ Premissa do relatório derrubada pelo spike C0.** `IMPLEMENTACAO-DETALHADA-V0-5.md` §2.1 apostou num provider
  RustCrypto puro para manter `unsafe` fora dos bytes hostis. **Não existe pilha TLS `unsafe`-free** no ecossistema Rust
  hoje (`docs/reports/SPIKE-C0-TLS-PROVIDER.md` §2.1, §6). Mitigação: carve-out `ring` **explícito e destacado** em
  `unsafe-allowlist.toml` (linha `row = 1`, EXCEPTION) + `ADR-0018` + PRD-009; C1 não reabre a decisão.
- **`ring` exige toolchain C no build nos 3 SOs** (`cc` compila o asm pré-gerado + C). Imagens de CI padrão têm; anotar
  no job de CI de C1 e na seção não-funcional do PRD-009 (`docs/reports/SPIKE-C0-TLS-PROVIDER.md` §6 item 2).
- **`unsafe-audit` (advisory) vai passar a listar `ring` +
  `subtle`/`zeroize`/`once_cell`/`getrandom`/`rustls-pki-types`/ `rustls-webpki`** quando `core/network` depender de
  `rustls`. O job hoje é `--forbid-only` e advisory (`unsafe-allowlist.toml:37-43`); a Fase P decide o mecanismo
  (allowlist por crate vs. baseline contado). C1 só precisa garantir que `ring` aparece sob uma entrada allowlistada e
  re-rodar `just unsafe-audit`.
- **`deny.toml` `multiple-versions = "deny"`** (`deny.toml:42`) — a pilha `ring` foi verificada **sem duplicatas** no
  spike; se C1 adicionar `rustls-pki-types` ou `rustls-webpki` como dep direta, confirmar que a versão bate com a que
  `rustls 0.23.43` puxa (`1.15.1` / `0.103.15`).
- **Portão de clippy proíbe o vocabulário de um parser de bytes** — `arithmetic_side_effects`, `as_conversions`,
  `indexing_slicing`, `string_slice` são `deny` (`Cargo.toml:56-66`). Mitigação: `domain/` mantém o portão integral
  (`checked_*`/`TryFrom`/`.get()`); `#[allow]` por função, comentado citando `ADR-0017`, só em `infrastructure/http1/` e
  `infrastructure/inflate.rs` (precedente: `core/graphics/src/infrastructure/software/`,
  `spdd/prompt/202609031011-…-f4.md:320-322`).
- **Determinismo do `inflate` compartilhado** — a Fase X vai fuzzar `network::inflate` e `png_decode`
  (`~/.claude/plans/…-fancy-dijkstra.md:636`); qualquer `panic`/`unwrap` no caminho de `inflate` quebra os dois. C1
  entrega `inflate` já sem `unwrap` e com teto de saída.
- **`arch-lint.toml` ainda não tem escopo `network`** — a Fase 0 deveria ter adicionado `network_domain`/
  `network_application`/`network` + `deny-scope-dep` negando `engine`/`runtime_rhai`/`runtime_rhai_bindings`/`alloy_cli`
  (`~/.claude/plans/…-fancy-dijkstra.md:290-296`). Se não estiver lá, C1 adiciona (fora do escopo deste spike C0).
- **`HeaderMap` case-insensitive + ordem de inserção** — não pode ser `HashMap` (perde ordem) nem `Vec<(String,String)>`
  público (viola first-class collection, `ADR-0010:131`). Molde: `AttributeMap` de `core/dom`.
- **`std::net::ToSocketAddrs` bloqueia sem timeout** — não há timeout de resolução no `std`. Mitigação: rodar a
  resolução no mesmo worker de pool (`ADR-0019`) e deixar o consumidor cancelar via `mpsc`/drop; documentar no contract
  record.

### Acceptance Criteria Coverage

| Critério (fonte)                                                     | Endereçável? | Notas / lacunas                                                                                    |
| -------------------------------------------------------------------- | ------------ | -------------------------------------------------------------------------------------------------- |
| `run_transport_suite` verde p/ `MockTransport` (CI)                  | Sim          | Código `pub` em `application/conformance.rs`, molde `run_backend_suite`                            |
| `run_transport_suite` verde p/ cliente real (`Http1Transport`)       | Sim (manual) | Handshake já validado no spike C0 contra `example.com`/`github.com`/`www.cloudflare.com` (TLS 1.3) |
| `run_policy_suite` verde p/ `AllowAllPolicy`                         | Sim          | `Allow`/`Rewrite`/`Deny` cobertos                                                                  |
| Resposta HTTP maliciosa → `NetworkError` tipado, nunca hang/pânico   | Sim          | `Content-Length` mentiroso, chunk inválido, header gigante, redirect em ciclo — um teste cada      |
| `cargo tree -p network` sem `engine`/`rhai`                          | Sim          | Job `layering`; `core/network/Cargo.toml` só ganha `rustls`/`webpki-roots` + `thiserror`           |
| `cargo test -p network --no-default-features` (`no-transport`) verde | Sim          | Feature estreada no molde de `core/graphics` `no-backend`                                          |
| `network::PORT_SCHEMA_VERSION = 1`, agregados `#[non_exhaustive]`    | Sim          | Congela em I4 (`~/.claude/plans/…-fancy-dijkstra.md:747`)                                          |
| `unsafe`-free em byte de socket (`ADR-0011` item 1 / `ADR-0018`)     | **Parcial**  | **NO-GO RustCrypto**; `ring` entra como exceção linha-1 registrada — ver spike C0 §6               |
| Contract record `http-transport-port-contract.md` (7 itens)          | Não (Fase P) | `~/.claude/plans/…-fancy-dijkstra.md:729`                                                          |
| Fuzz de `inflate`                                                    | Não (Fase X) | C1 entrega `inflate` fuzz-ready (sem `unwrap`, teto de saída)                                      |
