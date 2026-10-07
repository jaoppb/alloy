# BARRA DE PESQUISA — proposta técnica e decomposição de issues para o omnibox

| Campo | Valor |
|---|---|
| **Status** | 🟡 Parcial — portas de janela e renderização operacionais; ausência de captura de texto e de UI de barra |
| **Cobertura** | ~0 % (0 de 7 tarefas iniciadas) |
| **Esforço [modelado]** | 6–9 dias-dev no escopo completo; 3,5–5 d no mínimo viável |
| **Depende de** | `core/window` (I4), `core/network` (`Url`), `core/graphics` (`DisplayListBuilder`) |

---

## 1. Estado atual — evidências

O navegador Alloy alcançou no marco v0.5 I4 a capacidade de abrir uma janela nativa do sistema operacional
e apresentar documentos HTML reais via pipeline completo (`alloy <url>`, implementado em
`alloy/src/application/event_loop/mod.rs:56-87` e `main.rs:99-105`). Contudo, toda a interação
interativa hoje restringe-se ao redimensionamento de janela e ao clique em links (`<a>`), conforme
comprovado em `alloy/src/application/event_loop/mod.rs:200-205` e `:250-260`.

Abaixo estão as evidências pontuais que demonstram as lacunas estruturais que impedem a existência
imediata de uma barra de pesquisa e endereços (*omnibox*):

1. **Eventos de teclado ignorados no loop principal:** Em `alloy/src/application/event_loop/mod.rs:247-260`,
   o método `PumpEvents::observe` consome apenas `CloseRequested`, `Resized`, `RedrawRequested`, `PointerMoved`
   e o clique esquerdo (`PointerButton::Left`). Quaisquer eventos de teclado caem no braço curinga `_ => {}`
   (`:258`), sendo descartados em silêncio.
2. **Ausência de texto e modificadores na porta de janela:** A camada de infraestrutura em
   `core/window/src/infrastructure/event_map.rs:43-46` mapeia teclas físicas para `domain::key::KeyCode`.
   Nenhum texto decodificado é propagado (`event.text` de `winit::event::KeyEvent` [F1] não é lido).
   Adicionalmente, `core/window/src/infrastructure/event_map.rs:60-61` descarta explicitamente
   `WindowEvent::ModifiersChanged(_)` e `WindowEvent::Ime(_)`, inviabilizando digitação de caracteres,
   reconhecimento de `Ctrl+L` e composição de acentuação (*dead keys*).
3. **Tipo `WindowEvent::Key` restrito:** Em `core/window/src/domain/event.rs:32-33`, a definição
   `Key { code: KeyCode, pressed: bool }` não carrega identificador de tecla lógica (como `Enter`,
   `Backspace`, `Escape`), nem texto digitado, nem estado de repetição.
4. **Ocupação total do viewport pelo documento web:** Em `alloy/src/application/event_loop/session/mod.rs:156-165`,
   a sessão repassa a totalidade da área da janela (`self.viewport`) para `render_dom_with_links`.
   Não existe conceito de área de interface (*chrome*) reservada no topo da janela; o conteúdo web
   é desenhado a partir da coordenada `(0, 0)`.
5. **Rigidez no parser de URLs:** Em `core/network/src/domain/url.rs:48`, o método `Url::parse_parts`
   exige obrigatoriamente a presença de `://` (`raw.split_once("://").ok_or(UrlDefect::MissingScheme)?`).
   Se um usuário digitar um domínio direto como `github.com` ou um termo de busca como `rust lang`,
   o parser rejeita com `NetworkError::InvalidUrl`, sem mecanismo de desambiguação para pesquisa.

A ausência de componentes de entrada de texto e de controle de barra no código atual é comprovada
pela busca de resultado zero no repositório:

```bash
grep -rn "TextInput\|text_input\|omnibox\|search_bar\|address_bar" core/ alloy/
# → 0 resultados
```

### Precedentes no código

- `core/graphics/src/application/builder.rs:120-159`: `DisplayListBuilder` já implementa primitivas
  robustas para retângulos com cantos arredondados (`draw_rounded_rect`) e texto posicionado (`draw_text`).
  **Reutilizar** integralmente essas operações para a pintura da barra, evitando adicionar motores
  externos de UI.
- `scripts/default_ui.rhai:9-17` e `core/runtime/rhai-bindings/src/window_bindings.rs:23-28`: já existem
  manifestos de capacidades e bindings para atalhos (`key_shortcut`) e títulos de janela (`title`).
  **Reutilizar** o padrão de arquitetura "Skeleton and Muscle" (ADR-0003) para permitir que scripts
  Rhai configurem o mecanismo de pesquisa padrão e interceptem eventos de navegação do omnibox.
- `core/window/src/domain/key.rs:24-60`: `KeyCode` fornece mapeamento para posições de teclas físicas
  conforme UI Events. **Reutilizar** para gatilhos de atalho físico (`Ctrl+L`, `Alt+D`), mas **divergir**
  na captura de caracteres editáveis, criando um evento semântico de entrada textual no domínio.

### Não-objetivos

- **Não implementar autocompletar via requisições de rede assíncronas** nesta fase: sugestões em tempo
  real via API de busca adicionariam complexidade de concorrência e latência desnecessárias ao MVP.
- **Não criar sistema de abas múltiplas (*tab-bar*) ou histórico persistido em disco**: o foco exclusivo
  deste relatório é a barra de navegação/pesquisa de documento único ativo.
- **Não incluir dependências pesadas de GUI de terceiros** (como `egui`, `slint` ou `iced`): a interface
  da barra deve ser renderizada pelas primitivas nativas do `core/graphics`, preservando a soberania
  do engine (ADR-0006 e ADR-0010).

---

## 2. As 5 decisões de design

### 2.1 Mapeamento de eventos de teclado na porta de janela

A digitação de termos de busca exige capturar tanto caracteres normais quanto teclas de controle
(`Backspace`, `Delete`, `Enter`, `Escape`, setas). No `winit 0.30.13` [F1], essas informações residem
em `winit::event::KeyEvent` (campos `logical_key`, `text` e `state`) e em `winit::event::Ime`.

| Opção | O que é | Custo [modelado] | Base | Veredito |
|---|---|---|---|---|
| **A (Recomendada)** | Adicionar `WindowEvent::TextInput(String)` e `WindowEvent::ControlKey` no domínio `core/window` | 1 d | [F1]; `core/window/src/domain/event.rs:18` | **Recomendada** — desacopla o engine de tipos do winit e suporta IME e layouts internacionais |
| **B** | Expor `winit::keyboard::Key` diretamente nos tipos públicos de `core/window` | 0,5 d | `Cargo.toml:73` | Rejeitada: violação frontal do contrato de portas (ADR-0011 item 2) |
| **C** | Tentar sintetizar texto a partir de `domain::key::KeyCode` | 2 d | `core/window/src/domain/key.rs:24` | Rejeitada: impraticável; ignora layouts de teclado (ABNT2, Dvorak, AZERTY) e acentuação |

### 2.2 Localização e tecnologia de renderização da barra

A barra de pesquisa precisa ser desenhada na janela do navegador sobrepondo o viewport.

| Opção | O que é | Custo [modelado] | Base | Veredito |
|---|---|---|---|---|
| **A (Recomendada)** | Renderização nativa declarativa via `DisplayListBuilder` no topo do frame | 1,5 d | `core/graphics/src/application/builder.rs:129` | **Recomendada** — latência zero, reusa o rasterizador CPU e independe de parsing HTML/CSS |
| **B** | Renderizar um documento HTML/CSS interno (*web-chrome*) para a barra | 3,5 d | `alloy/src/application/pipeline.rs:188` | Rejeitada: exige instanciar segundo pipeline de cascade/layout a cada tecla digitada |
| **C** | Integrar biblioteca imediata (ex.: `egui`) | 3 d | [modelado: ecossistema Rust] | Rejeitada: fere a independência do engine e cria duplicação de backends gráficos |

### 2.3 Desambiguação de URL vs Query de pesquisa (`OmniboxClassifier`)

O usuário pode digitar uma URL completa, um domínio sem esquema ou palavras-chave de busca.

| Opção | O que é | Custo [modelado] | Base | Veredito |
|---|---|---|---|---|
| **A (Recomendada)** | Classificador heurístico em `core/network` com normalização e fallback para motor de busca | 0,75 d | [F2]; [F3]; `core/network/src/domain/url.rs:48` | **Recomendada** — padrão da indústria (Chromium/Firefox), encaminha URLs implícitas e buscas |
| **B** | Exigir estritamente `http://` ou `https://` | 0,2 d | `core/network/src/domain/url.rs:48` | Rejeitada: péssima experiência de usabilidade, impede buscas diretas |
| **C** | Consulta DNS prévia para verificar se termo sem ponto é host local | 2 d | `core/network/src/domain/authority.rs` | Rejeitada: bloqueia o main thread e gera vazamento de privacidade para a rede |

### 2.4 Modelo de buffer e edição de texto com segurança UTF-8

A edição interativa da barra exige inserir caracteres, apagar (*backspace*), mover cursor e selecionar texto.

| Opção | O que é | Custo [modelado] | Base | Veredito |
|---|---|---|---|---|
| **A (Recomendada)** | Struct de domínio `OmniboxBuffer` manipulando caracteres via iteradores seguros de limite escalar | 1 d | `Cargo.toml:98`; ADR-0010 §3 | **Recomendada** — respeita o lint `string_slice = "deny"` e previne pânicos de corte no meio de código UTF-8 |
| **B** | Fatiamento direto de strings por índices numéricos brutos (`&s[..i]`) | 0,5 d | `Cargo.toml:92,98` | Rejeitada: terminantemente proibida pelos lints estritos de compilação do repositório |

### 2.5 Particionamento de Viewport (Chrome vs Conteúdo)

A janela do Alloy precisa coexistir entre a barra superior e o conteúdo do documento web.

| Opção | O que é | Custo [modelado] | Base | Veredito |
|---|---|---|---|---|
| **A (Recomendada)** | Particionamento vertical na `Session`: topo para a barra (`40 px`), restante para o documento | 1 d | `alloy/src/application/event_loop/session/mod.rs:156` | **Recomendada** — simples, transparente para `core/css`, requer apenas ajuste de *offset* no hit-testing |
| **B** | Conteúdo sob a barra com transparência (*floating overlay*) | 2 d | `core/graphics/src/domain/command.rs:57` | Rejeitada: complexidade visual e oculta o topo dos sites |

### 2.6 O que não fazer

- **Não criar threads em segundo plano para o buffer de texto**: a edição de texto do omnibox é
  estritamente síncrona e pertence à thread principal da janela; I/O de rede só ocorre após o `Enter`.
- **Não modificar `core/css` para estilizar a barra de pesquisa**: estilização do chrome por CSS
  é meta de longo prazo (v0.9+ com DevTools); nesta fase, cores e dimensões são tokens de domínio.
- **Não usar tipos primitivos nus (*naked primitives*)**: índices de cursor e comprimentos devem
  adotar *newtypes* fortemente tipados (`CursorPosition`, `ChromeHeight`), em cumprimento irrestrito
  ao Object Calisthenics (`CLAUDE.md:231-237`).

---

## 3. Plano de implementação e especificação das issues

A implementação é estruturada em **7 issues técnicas sequenciais**, modeladas para execução via
*git worktrees* ou fluxo SPDD:

### Issue 1: `feat(window): mapear entrada de texto e teclas de controle no WindowEvent`
- **Contexto:** `core/window/src/infrastructure/event_map.rs:60-61` descarta `Ime` e texto digitado.
- **Escopo:**
  - Estender `WindowEvent` em `core/window/src/domain/event.rs` com `TextInput { text: String }` e
    `ControlKey { key: LogicalKey, state: KeyState }`.
  - Mapear em `infrastructure/event_map.rs` os campos `event.text` e `event.logical_key` de
    `winit::event::KeyEvent` [F1], além de tratar `winit::event::Ime::Commit(text)`.
  - Incrementar `window::PORT_SCHEMA_VERSION` de 2 para 3, atualizando os testes de conformidade em
    `core/window/src/application/conformance.rs`.
- **Critério de aceite:** Testes unitários comprovando que pressionar teclas alfanuméricas gera
  `WindowEvent::TextInput` e teclas de controle (`Enter`, `Backspace`, `Escape`) geram `WindowEvent::ControlKey`.
- **Esforço [modelado]:** 1–1,5 dias-dev.

### Issue 2: `feat(network): classificador de omnibox para desambiguação de URL vs query de busca`
- **Contexto:** `core/network/src/domain/url.rs:48` falha se não houver `://`, impedindo buscas e domínios simples.
- **Escopo:**
  - Criar o serviço de domínio `OmniboxClassifier` em `core/network/src/domain/omnibox.rs`.
  - Se a entrada começar com esquema (`http://` ou `https://`), analisar diretamente com `Url::parse`.
  - Se não possuir esquema e parecer um domínio ou endereço local válido (sem espaços e contendo ponto
    ou `localhost`), prefixar com `https://` (ou `http://` para `localhost`).
  - Se contiver espaços ou for um termo livre, formatar como query para o motor de busca padrão
    (ex.: `https://duckduckgo.com/?q={searchTerms}` [F2, F3]) aplicando *percent-encoding*.
- **Critério de aceite:** Testes exaustivos para `github.com` → `https://github.com/`, `localhost:8080` →
  `http://localhost:8080/` e `termo de busca` → `https://duckduckgo.com/?q=termo+de+busca`.
- **Esforço [modelado]:** 0,5–1 dia-dev.

### Issue 3: `feat(ui): modelo de domínio e máquina de estados para edição de texto (OmniboxState)`
- **Contexto:** Ausência de estrutura para manter texto digitado, cursor e estado de foco.
- **Escopo:**
  - Criar `OmniboxState` e `OmniboxBuffer` em `alloy/src/application/omnibox/state.rs`.
  - Encapsular estado de foco (`is_focused: bool`), texto atual, posição do cursor e seleção.
  - Implementar métodos de mutação CQS: `insert_text(&mut self, text: &str)`, `backspace(&mut self)`,
    `delete(&mut self)`, `move_cursor_left(&mut self)`, `move_cursor_right(&mut self)`, `select_all(&mut self)`.
  - Garantir navegação estritamente alinhada a caracteres UTF-8, sem uso de indexação crua por colchetes.
- **Critério de aceite:** Testes com caracteres ASCII e multibyte (acentos, emojis) validando inserção,
  remoção e movimentação de cursor sem causar pânico.
- **Esforço [modelado]:** 1 dia-dev.

### Issue 4: `feat(graphics): renderização declarativa da barra de pesquisa em DisplayList`
- **Contexto:** Necessidade de desenhar a barra de pesquisa na janela usando as primitivas existentes.
- **Escopo:**
  - Criar `OmniboxPainter` em `alloy/src/application/omnibox/paint.rs`.
  - Emitir comandos para `DisplayListBuilder`: fundo do chrome, retângulo do campo com cantos
    arredondados (`corner_radius = 6 px`), borda de foco quando ativo, texto do buffer ou URL atual via
    `draw_text`, e linha vertical representando o cursor (*caret*) quando focado.
  - Utilizar fontes e métricas providas pelo `FontProvider` já instanciado no pipeline.
- **Critério de aceite:** Teste de renderização comprovando emissão ordenada dos comandos no
  `DisplayList` e teste visual gerando snapshot PNG determinístico da barra.
- **Esforço [modelado]:** 1–1,5 dias-dev.

### Issue 5: `feat(alloy): particionamento de viewport e roteamento de cliques`
- **Contexto:** `alloy/src/application/event_loop/session/mod.rs:156` aloca 100 % da janela para a web.
- **Escopo:**
  - Definir `CHROME_HEIGHT = 42 px`.
  - Subtrair `CHROME_HEIGHT` da altura enviada ao layout do documento web em `Session::relayout`.
  - Ajustar o hit-testing de links em `alloy/src/application/event_loop/hit_test.rs`: cliques com
    `y < CHROME_HEIGHT` não acionam links da página; cliques com `y >= CHROME_HEIGHT` têm a coordenada `y`
    deslocada em `-CHROME_HEIGHT` antes de testar contra a árvore de caixas.
  - Compor a barra de pesquisa e o conteúdo web no mesmo framebuffer final apresentado pelo `Presenter`.
- **Critério de aceite:** Clicar na barra superior não aciona links que estariam no topo do documento;
  links da página continuam clicáveis em suas posições visuais corretas.
- **Esforço [modelado]:** 1 dia-dev.

### Issue 6: `feat(alloy): atalhos de teclado (Ctrl+L, Enter, Esc) e integração ao event loop`
- **Contexto:** Conectar a digitação, atalhos globais e navegação ao ciclo do `run_loop`.
- **Escopo:**
  - Em `alloy/src/application/event_loop/mod.rs`, capturar `WindowEvent::TextInput` e `WindowEvent::ControlKey`.
  - Atalho `Ctrl+L` ou clique no campo de texto: foca o omnibox e seleciona o texto existente.
  - Tecla `Escape`: cancela a edição, restaura a URL do documento atual e remove o foco.
  - Tecla `Enter`: obtém o texto do buffer, executa `OmniboxClassifier::classify`, dispara
    `session.navigate(url)` e remove o foco do campo.
  - Redesenhar a interface (solicitando `request_redraw`) a cada alteração no buffer da barra.
- **Critério de aceite:** Teste de integração ponta a ponta: focar a barra via `Ctrl+L`, digitar
  `example.com`, teclar `Enter` e verificar o disparo correto de `session.navigate("https://example.com/")`.
- **Esforço [modelado]:** 1–1,5 dias-dev.

### Issue 7: `feat(runtime): bindings do Rhai para configuração e políticas do omnibox`
- **Contexto:** Respeito ao ADR-0003 ("Skeleton and Muscle"): políticas de usuário devem residir em scripts Rhai.
- **Escopo:**
  - Registrar bindings em `core/runtime/rhai-bindings/src/window_bindings.rs`: `set_search_engine(template_url)`
    e hook de evento `on_omnibox_navigate(query)`.
  - Adicionar script padrão em `scripts/default_ui.rhai` configurando DuckDuckGo como motor padrão.
- **Critério de aceite:** Script Rhai consegue alterar o motor de busca em tempo de execução sem recompilar o binário.
- **Esforço [modelado]:** 0,5–1 dia-dev.

---

**Mínimo viável (MVP funcional para navegação e busca, Issues 1 a 6):** 5,0–7,5 dias-dev.
**Escopo completo (incluindo extensibilidade Rhai, Issues 1 a 7):** 5,5–8,5 dias-dev.

Ordem recomendada: **Issue 1 → Issue 2 → Issue 3 → Issue 4 → Issue 5 → Issue 6 → Issue 7**.
A Issue 1 destrava a entrada de dados antes que a lógica de aplicação seja acoplada, evitando retrabalho.

---

## 4. Armadilhas

| Armadilha | Mitigação |
|---|---|
| Quebra de limites UTF-8 ao calcular cursor ou apagar caracteres com *backspace* | Operar estritamente sobre `char_indices()` e iteradores reversos de caracteres; rejeitar fatiamento indexado direto por colchetes em conformidade com o lint `string_slice = "deny"` |
| Dessincronização do hit-testing de links no documento após o deslocamento do viewport | Subtrair rigidamente `CHROME_HEIGHT` de todas as coordenadas de ponteiro antes de passá-las para `hit_test(&self.links, position)` |
| Relayout desnecessário do documento web inteiro a cada caractere digitado na barra de pesquisa | Separar a renderização do chrome: digitar altera apenas o buffer do omnibox, emitindo novo `DisplayList` do topo e repintando o frame em cache sem refazer cascade/layout do DOM |
| Conflito entre atalhos do navegador (`Ctrl+L`, `Ctrl+R`) e entrada de texto | Interceptar teclas de controle com prioridade quando o omnibox estiver focado, repassando eventos não consumidos para a página |
| Digitação duplicada em sistemas operacionais com IME ativo (ex.: Linux IBus/Fcitx, Windows) [F1] | Processar texto exclusivamente via evento de commit do IME ou evento unificado de texto, ignorando eventos de tecla física crua para propósitos de inserção de glifos |

---

## 5. Verificação

**Automatizável em testes unitários e de integração (`cargo test`):**
- [ ] `core/window`: `map_window_event` converte eventos `KeyEvent` do winit com texto para `WindowEvent::TextInput`.
- [ ] `core/window`: teclas `Enter`, `Backspace` e `Escape` são convertidas em `WindowEvent::ControlKey`.
- [ ] `core/network`: `OmniboxClassifier` identifica URLs completas (`https://site.org`), domínios (`site.org`),
      endereços locais (`localhost:3000`) e termos de busca com codificação de caracteres [F2, F3].
- [ ] `alloy`: `OmniboxBuffer` manipula strings com caracteres multibyte e acentuação sem pânico e respeita limites.
- [ ] `alloy`: `hit_test` em `y < CHROME_HEIGHT` retorna `None` mesmo que haja um link desenhado no topo do DOM.
- [ ] `alloy`: alteração no texto da barra não incrementa o contador `stats.relayouts` da sessão (prova de coalescência).
- [ ] `rhai-bindings`: chamada a `set_search_engine` altera a URL de modelo usada pelo classificador.

**Verificação visual e interativa manual (host Linux / Wayland / X11):**
- [ ] Ao abrir `alloy https://html.duckduckgo.com/html/`, a barra exibe a URL carregada no topo da janela.
- [ ] Pressionar `Ctrl+L` foca o campo e seleciona o texto.
- [ ] Digitar `rust language` e teclar `Enter` direciona a navegação para a página de resultados da busca.
- [ ] Digitar uma URL inexistente exibe o card de erro com a URL digitada visível na barra de pesquisa.

---

## 6. Riscos

1. **Incompatibilidade ou duplicidade de eventos de teclado entre plataformas winit no Linux [F1].**
   No Wayland e X11, versões diferentes de compositores podem enviar tanto `KeyEvent` com caractere quanto
   eventos de IME vazios. A implementação deve deduplicar ou tratar commits de texto de maneira idempotente.
2. **Custo de repintura por frame durante a digitação rápida.**
   Mesmo sem relayout do DOM, redesenhar a barra e compor com o frame em cache via software blit
   em telas 4K pode consumir tempo de CPU se o dano (*damage rect*) não for delimitado. A mitigação é manter a
   barra em uma área retangular limitada e acionar apenas o redesenho da faixa superior no `Presenter` [F4].
3. **Escalonamento de versão de portas e quebra de contratos.**
   Adicionar novas variantes ao `WindowEvent` eleva a versão do schema da porta `core/window` (2 → 3).
   Como o marco I4 ainda não foi congelado formalmente no branch `main`, a alteração deve ser consolidada
   em sincronia com a equipe de arquitetura.

---

## 7. Arquivos tocados

| Arquivo | Mudança |
|---|---|
| `core/window/src/domain/event.rs` | Estender `WindowEvent` com variantes de texto e controle semântico |
| `core/window/src/domain/key.rs` | Adicionar identificadores de teclas de navegação e controle |
| `core/window/src/infrastructure/event_map.rs` | Mapear `event.text`, `event.logical_key` e `WindowEvent::Ime` |
| `core/network/src/domain/omnibox.rs` | **Novo** — classificador de URLs e termos de busca |
| `core/network/src/lib.rs` | Exportar serviços do classificador de omnibox |
| `alloy/src/application/omnibox/mod.rs` | **Novo** — módulo agregador do omnibox |
| `alloy/src/application/omnibox/state.rs` | **Novo** — máquina de estados de edição e buffer |
| `alloy/src/application/omnibox/paint.rs` | **Novo** — pintor declarativo em `DisplayList` |
| `alloy/src/application/event_loop/session/mod.rs` | Particionar viewport verticalmente e integrar estado do omnibox |
| `alloy/src/application/event_loop/hit_test.rs` | Ajustar compensação de offset vertical `CHROME_HEIGHT` |
| `alloy/src/application/event_loop/mod.rs` | Capturar eventos de teclado, rotear digitação e tratar atalhos globais |
| `core/runtime/rhai-bindings/src/window_bindings.rs` | Expor bindings de configuração de motor de busca para scripts |
| `scripts/default_ui.rhai` | Configurar atalhos padrão e URL base do DuckDuckGo |

---

## 8. Fontes consultadas

| # | Fonte | Tipo | Versão | Consultada em | Sustenta |
|---|---|---|---|---|---|
| F1 | [Winit — `KeyEvent` and `Ime` Documentation](https://docs.rs/winit/latest/winit/event/struct.KeyEvent.html) | Doc oficial | 0.30.13 | 2026-10-07 | Decisão 2.1; Issue 1; Armadilhas 4 |
| F2 | [WHATWG URL Standard](https://url.spec.whatwg.org/) | Doc oficial | Living Standard | 2026-10-07 | Decisão 2.3; Issue 2 |
| F3 | [Chromium Omnibox Autocomplete & Query Architecture](https://www.chromium.org/developers/design-documents/omnibox/) | Projeto de terceiros | Referência Chromium | 2026-10-07 | Decisão 2.3; Issue 2 |
| F4 | [Softbuffer crate documentation](https://docs.rs/softbuffer/latest/softbuffer/) | Doc oficial | 0.4.8 | 2026-10-07 | Risco 2; Issue 5 |

---

> Nenhum item deste relatório foi executado em ambiente gráfico real com usuário. Toda a
> análise vem da leitura estrita do código no branch `main` (commit `b495fda`), das especificações
> de arquitetura do Alloy e da documentação das versões fixadas das dependências; a validação
> em tempo de execução está listada na seção 5 como pendente.
