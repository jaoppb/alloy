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
      não são automatizáveis em CI).
- [ ] Golden e2e sobre mocks verde em CI.
- [ ] Resize 800→1024 = exatamente **1** relayout (contador de relayout instrumentado no teste).
- [ ] 50 imagens = relayouts limitados, não 50.
- [ ] `network::PORT_SCHEMA_VERSION`, `window::PORT_SCHEMA_VERSION`, agregados `TreeSink` congelados e documentados.
- [ ] `cargo test --workspace` continua todo verde.
- [ ] **Checkpoint:** depois do commit, `git push -u origin feat/v0-5` e abrir um PR draft chamado
      `` `v0.5 · I4 alloy <url>` `` via `gh`.

## Convenção de commit

```text
feat(alloy): native window rendering via `alloy <url>` (v0.5 I4)

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
```
