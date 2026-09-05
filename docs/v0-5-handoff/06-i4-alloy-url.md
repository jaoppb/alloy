# I4 — `alloy <url>`: janela nativa

## Contexto

O objetivo final da campanha inteira: `alloy https://example.com` abrindo uma janela nativa e renderizando a página
real. Depende de **I2** (pipeline), **C1** (rede/TLS), **C2** (janela), **M** (política scriptada) e **X** (imagens) — é
a fase que amarra tudo. **Segundo checkpoint de integração**: _push_ + PR draft ao final.

## Estado atual

Nada desta fase existe — depende inteiramente das seis fases anteriores estarem prontas primeiro. Não comece este
arquivo antes de I2, C1, C2, M e X estarem todos commitados.

## Passos

1. **`alloy/src/application/event_loop.rs`** — o laço único dono da _thread_ principal (ADR-0019).
   `WindowSystem::pump_events` o dirige; I/O bloqueante (`HttpTransport::execute`, DNS) roda num _pool_ de
   `std::thread`, resultado volta por `std::sync::mpsc` como um evento do laço. **Sem runtime assíncrono** — decisão já
   tomada e registrada em ADR-0019.

2. **`alloy/src/application/navigation.rs`** — `URL → HttpTransport` (via `RequestPolicy`) → decodificação de _charset_
   (`network::charset`, já existe) → `core/html` → `DomTree` → `pipeline.rs` (de I2).

3. **`alloy/src/application/subresource.rs`** — fila de `<link rel=stylesheet>` e `<img>` com coalescência: cada
   conclusão agenda no máximo **uma** re-cascata + relayout por quadro, nunca uma por recurso.

4. **Resize** — `WindowEvent::Resized` → re-cascata → relayout → repintura, coalescido por quadro (mesma disciplina de
   coalescência do item anterior).

5. **CLI** — `alloy <url>` abre uma janela nativa via `WindowSystem`/`Presenter` (de C2).

6. **`alloy/tests/e2e_golden.rs`** — página servida por `MockTransport` (de C1) +
   `HeadlessWindowSystem`/`RecordingPresenter` (de C2), renderizada pixel-idêntica com CSS de autor + texto + imagem.
   Casos específicos a provar:
    - resize 800→1024 dispara **um** relayout, não mais;
    - 50 imagens chegando disparam um número **limitado** de relayouts, não 50 (a coalescência do item 3 está
      funcionando).

7. **Congelamento** — `network::PORT_SCHEMA_VERSION`, `window::PORT_SCHEMA_VERSION` e os agregados de `TreeSink` de
   `core/html` congelam aqui. Qualquer mudança de forma depois disso exige nota de migração nos respectivos PRDs (009,
   010, 008).

## Crates de referência

- `core/network/src/infrastructure/mock.rs` — `MockTransport` já existe e é o que o golden e2e consome.
- `core/window/src/infrastructure/headless.rs` — `HeadlessWindowSystem` + `RecordingPresenter`, idem.

## Definition of Done

- [ ] `alloy https://example.com` renderiza a página real numa janela nativa (verificação manual — display e rede reais
      não são automatizáveis em CI, e esta sessão rodou num sandbox sem display; **não verificado manualmente**, só por
      leitura de código — `alloy/src/main.rs`'s `run_browse_command`).
- [x] Golden e2e sobre mocks verde em CI — `alloy/tests/e2e_golden.rs`
      (`navigated_page_with_stylesheet_and_image_matches_golden_reference`), `MockTransport` +
      `HeadlessWindowSystem`/`RecordingPresenter`, CSS de autor via `<link rel=stylesheet>` (não só inline) + texto +
      imagem.
- [x] Resize coalescing: N `WindowEvent::Resized` num só `pump_events` custam **1** relayout, e o viewport reflete o
      último (`alloy/src/application/event_loop.rs`'s `tests::multiple_resizes_in_one_pump_coalesce_to_one_relayout`).
      Prova direta sobre `pump_once` (thread-free), não sobre `run_browser` inteiro — ver o comentário do módulo de
      teste para o porquê.
- [x] 50 imagens = relayouts limitados, não 50 — mesma técnica
      (`tests::fifty_image_arrivals_in_one_pump_coalesce_to_one_relayout`, assert `relayouts == 1`).
- [x] `network::PORT_SCHEMA_VERSION`, `window::PORT_SCHEMA_VERSION` congelados e documentados
      (`docs/architecture/{http-transport,window-system}-port-contract.md` item 7 → ✅, `PRD-009`/`PRD-010` §6). Como
      bônus desta fase, `html::PORT_SCHEMA_VERSION` (que nem existia) também foi introduzido e documentado
      (`docs/architecture/html-tree-sink-port-contract.md`, que também registra um gap real encontrado: `HtmlError` não
      carrega `SourceLocation`).
- [x] `cargo test --workspace --all-features` — todo verde (confirmado nesta sessão).
- [ ] **Checkpoint:** `git push -u origin feat/v0-5` e abrir um PR draft `` `v0.5 · I4 alloy <url>` `` via `gh` — **não
      feito nesta sessão**: múltiplas sessões estavam mexendo em `feat/v0-5` concorrentemente, e um `push`/PR é uma ação
      compartilhada que exige confirmação explícita do usuário, não uma chamada unilateral do agente.

## Estado atual (verificado nesta sessão, v0.5 Fase I4)

Implementado: `alloy/src/application/{navigation,subresource,event_loop}.rs`, `alloy/src/main.rs`'s `alloy <url>`.
`event_loop.rs` é o laço único (`ADR-0019`) — thread pool via `std::thread::spawn` por fetch, resultado por
`std::sync::mpsc`, sem runtime assíncrono. Coalescência: um `pump_once` drena todo evento de janela e toda mensagem de
fetch já disponíveis antes de decidir relayout, no máximo um por ciclo. `core/css`'s `collect_style_sheets` só via
`<style>`/`style=`; o `<link rel=stylesheet>` externo é buscado por `subresource.rs` e absorvido via
`StyleSheetSet::absorb` em `Origin::Author`. Imagem: `IntrinsicSize::Pending` é atribuído a todo `<img>` desde B4/X,
carregado ou não — por isso todo id descoberto ganha um placeholder 1×1 transparente
(`subresource::placeholder_framebuffer`) assim que descoberto, e a imagem real substitui esse placeholder quando o fetch
termina. `cargo clippy --workspace --all-targets --all-features -- -D warnings` e `cargo fmt --all -- --check` limpos.

## Convenção de commit

```text
feat(alloy): native window rendering via `alloy <url>` (v0.5 I4)

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
```
