# Diagnóstico — `alloy <url>` abre a janela mas não exibe nada (tela branca)

| Campo              | Valor                                                                                                                                                                                                                      |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Sintoma**        | `alloy https://html.duckduckgo.com/html/` (Arch Linux / Wayland) abre a janela nativa, mas a tela fica **branca e continua branca** — nem a página, nem o card vermelho de "Navigation Error"                              |
| **Veredito**       | Duas falhas estruturais independentes no caminho I4 (`run_browser` → `run_loop` → `pump_once`), ambas compatíveis com o sintoma                                                                                            |
| **Correções**      | `core/window` (porta `WindowSystem::request_redraw`, `PORT_SCHEMA_VERSION` 1→2) + `alloy` (split relayout/repaint no event loop; navegação tipa status/corpo-vazio/corpo-não-UTF-8)                                        |
| **Fora de escopo** | Largura do corte de CSS da v0.5 (ver [`DIAGNOSTICO-CSS-EM-SITES-REAIS.md`](DIAGNOSTICO-CSS-EM-SITES-REAIS.md)); brotli / HTTP/2 / `Accept-Language` / cookie jar em `core/network`; abort de `dom::TagName` em tag com `:` |

---

## 1. Causa 1 — não havia loop de repaint; `RedrawRequested` era ignorado

`alloy/src/application/event_loop.rs` `pump_once` só apresentava um frame quando `session.dirty` estava marcado
(navegação, resize, imagem, stylesheet). O `match` de eventos tratava `CloseRequested`/`Resized`/`PointerMoved`/
`PointerButton` e jogava o resto em `_ => {}` — inclusive `WindowEvent::RedrawRequested`, que a porta **já produz**
(`core/window/src/infrastructure/event_map.rs:48`). `WinitSystem` nunca chamava `window.request_redraw()`.

Efeito: `present_if_ready` rodava um punhado de vezes logo após o launch e depois nunca mais. No Wayland (backend padrão
do winit 0.30) o primeiro `buffer.present()` costuma cair numa surface ainda não configurada e o compositor o descarta;
sem tratamento de `RedrawRequested` e sem `request_redraw()`, a janela nunca recebe outro frame. Em alguns compositores
nem o resize recupera, porque a surface do `softbuffer` já entrou em estado ruim sem o ciclo de damage/redraw.

`present_if_ready` também re-executava **todo** o pipeline (`render_dom_with_links` → cascade → layout → paint → raster
→ readback) a cada chamada, então "apresentar todo pump cycle" não era opção (relayout a cada 4 ms).

## 2. Causa 2 — a navegação renderizava um documento vazio em silêncio

`alloy/src/application/navigation.rs` fazia `response.body().as_str().unwrap_or_default()` e `html::parse(body)` **sem
checar o status HTTP**. Um `2xx` com corpo vazio, um `204`, um `3xx` não-seguível, ou um corpo que chegou como bytes
crus não-UTF-8 (sem `Content-Type` textual → o transcode de `core/network` é pulado) viravam `html::parse("")` → DOM
vazio → **janela branca**, logada apenas como `navigation complete`. É o único caminho que produz uma tela genuinamente
vazia **sem** o card de erro — exatamente o sintoma. (O caminho de subrecurso já checava status via `ensure_success`; a
navegação não.)

## 3. O que mudou

1. **`core/window` — porta `WindowSystem::request_redraw(&mut self)`** (comando CQS, sem retorno, no-op antes de haver
   janela). `WinitSystem` encaminha para `winit::Window::request_redraw`; `HeadlessWindowSystem` enfileira um
   `RedrawRequested`. `window::PORT_SCHEMA_VERSION` 1→2; tabela de migração em
   [`window-system-port-contract.md`](../architecture/window-system-port-contract.md) §4; adendo em
   [`ADR-0019`](../adr/0019-single-event-loop-owns-the-main-thread.md).

2. **`alloy/src/application/event_loop.rs` — split relayout / repaint.** `present_if_ready` virou:
    - `relayout_and_present` — reconstrói a display list, apresenta, **cacheia os pixels** em `Session.last_frame`, e é
      o **único** ponto que incrementa `stats.relayouts` (a prova de coalescing do I4 depende disso).
    - `repaint` — re-blita `Session.last_frame` sem pipeline, sem tocar `relayouts`. Serve `RedrawRequested`.

    `pump_once` agora: novo arm `RedrawRequested => needs_repaint = true`; após um relayout chama
    `system.request_redraw()` (re-arma o redraw da plataforma, cobrindo o primeiro present perdido no Wayland); se houve
    `needs_repaint` e **não** houve relayout neste ciclo, chama `repaint`. Rajada de `RedrawRequested` num pump colapsa
    para um `repaint`.

3. **`alloy/src/application/navigation.rs` — `navigate` valida a resposta.** Ordem: status (`ensure_success`, agora
   compartilhado com os fetches de subrecurso, emitindo `AlloyError::HttpStatus`) → corpo vazio
   (`AlloyError::EmptyDocument`) → corpo não-UTF-8 (`AlloyError::NonTextualDocument`). Qualquer um desses `Err` já vira
   o card vermelho visível em `Session::apply` — com a Causa 1 corrigida, o card chega à tela.
   `AlloyError::SubresourceStatus` foi renomeado para `HttpStatus` (usado pelos dois caminhos). `LoopStats` ganhou
   `navigation_errors`.

## 4. Testes

- `core/window`: `conformance.rs` ganhou `check_request_redraw_before_create_window_is_silent` e
  `check_request_redraw_is_reusable`; `RecordingPresenter::present_count()` distingue repaint de relayout.
- `alloy/src/application/event_loop.rs` (`mod tests`): `a_redraw_request_repaints_the_cached_frame_without_a_relayout`,
  `a_redraw_request_before_the_first_frame_is_a_silent_noop`, `a_relayout_arms_a_following_repaint`,
  `many_redraw_requests_in_one_pump_coalesce_to_one_repaint`. Os dois testes de coalescing do I4 seguem verdes
  (`relayouts == 1`).
- `alloy/src/application/navigation.rs` (`mod tests`): 200 normal, 200 vazio, 204, 304, 404, corpo não-UTF-8.
- `alloy/tests/navigation_integration.rs`: corpo vazio / 204 / não-UTF-8 → `navigation_errors == 1` e um frame (o card)
  apresentado.

## 5. Verificação manual

```bash
cargo build -p alloy

# O bug reportado. Antes: janela branca. Depois: a página (ou o card de erro) aparece < 1 s.
ALLOY_LOG=alloy=debug,network=debug cargo run -p alloy -- https://html.duckduckgo.com/html/
#   Minimizar+restaurar / arrastar entre monitores: o conteúdo permanece (repaint em RedrawRequested),
#   sem novas linhas de relayout.

# Causa 1 isolada: o backend X11 já renderizava antes da correção; o Wayland padrão não.
WINIT_UNIX_BACKEND=x11 cargo run -p alloy -- https://html.duckduckgo.com/html/

# Causa 2: documento vazio / não-textual -> card de erro visível, não tela branca.
ALLOY_LOG=alloy=debug,network=debug cargo run -p alloy -- https://httpbin.org/status/204
ALLOY_LOG=alloy=debug,network=debug cargo run -p alloy -- https://www.google.com/favicon.ico
```

## 6. Follow-up de CSS — lote focado para a `html.duckduckgo.com/html/`

Com a janela já aparecendo, a página seguia **sem estilo**: a aparência da DDG/html vem toda de uma folha externa que
usa propriedades fora do corte da v0.5. Primeiro passo, trazido do território v0.7:

- **`core/css` — shorthands `background` e `border`** (`SUPPORTED_PROPERTIES` 34 → 36; `css::PORT_SCHEMA_VERSION` 4 → 5;
  nota de migração em [`PRD-007`](../requirements/PRD-007-style-cascade-and-layout-engine-ports.md) §6 e na tabela de
  [`style-cascade-port-contract.md`](../architecture/style-cascade-port-contract.md) §4). Cada um **estreitado ao único
  componente que o corte já resolve**: `background` → a **cor** (dobra em `background_color`), `border` → a **largura**
  (dobra nas arestas `border`, como `border-width`). `url()`, gradientes, `no-repeat`/`center`/`/100%`, e o
  `<line-style>` e a cor de uma borda são varridos; `none` / `0` zeram. Sem campo novo em `ComputedStyle` — só mais
  entradas para campos que já existem. `parser/values.rs` ganhou `parse_background_shorthand` /
  `parse_border_shorthand`; `cascade/values.rs`, `MANIFEST.md` e `manifest_runner.rs` acompanham.
- **Efeito na DDG/html**: o fundo `#f7f7f7` (`.body--home { background:#f7f7f7 }`) e a caixa de busca com borda passam a
  renderizar — antes a página era 100% branca porque a DDG só usa `background:` shorthand, que era descartado inteiro.

### Ainda pendente (não neste lote)

- **`margin: auto` (centralização)** — a logo e a caixa de busca ficam à esquerda em vez de centradas. Precisa de
  `Length::Auto` (ou representação de margem específica) na caixa de bloco → outro bump de schema.
- **`background-image: url(...)`** — a logo da DDG e o botão de lupa. Precisa: campo novo em `ComputedStyle`, descoberta
  da URL na cascata, novo fluxo de subrecurso em `alloy` e pintura do fundo. Só PNG (a DDG tem fallback `.png` no padrão
  `background: <png>; background: <svg>`).
- **`position` / `max-width` / `border-radius` / `box-shadow`** — fora do escopo escolhido; território v0.7 pleno
  ([`DIAGNOSTICO-CSS-EM-SITES-REAIS.md`](DIAGNOSTICO-CSS-EM-SITES-REAIS.md)).

### Verificação do lote de CSS

```bash
# Render headless da DDG com a folha real embutida (determinístico):
#   fundo cinza + caixa de busca com borda visíveis; logo/centralização ainda não.
alloy render <ddg-com-style-inline>.html -o out.png --width 1000 --height 760
cargo test -p css   # manifest_runner + value_objects pinam 36 propriedades / PORT_SCHEMA_VERSION 5
```
