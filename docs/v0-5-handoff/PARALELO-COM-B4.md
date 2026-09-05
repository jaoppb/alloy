# O que pode rodar agora, em paralelo com B4

> [!NOTE] **Status da Estratégia de Despacho**: **Superado e Concluído**. Todas as frentes que estavam bloqueadas ou
> paralelizadas em torno de B4 foram entregues e verificadas:
>
> - **B4** entregue em `1353647`
> - **B5** entregue em `fc7d55e` (Faixa 1)
> - **X** entregue em `3558e4c` (Faixa 2)
> - **I2** entregue em `9bb8ae3` (paralelo I2 + M executado com sucesso)
> - **M integral** entregue em `067e87b` (Faixa 3 integral, incluindo `css_bindings.rs`)
>
> O gargalo foi inteiramente superado. A campanha avança para a Fase **I4** (`06-i4-alloy-url.md`), seguida de **P**
> (`07-p-final-gates.md`). O texto abaixo é mantido como registro histórico do plano de despacho paralelo original.

`01-b4-box-model-inline-flexbox.md` é o único arquivo desta pasta que pede uma IA mais forte dedicada só a ele — o
algoritmo de Flexbox, os testes de retângulo e a correção do `pipeline.rs` são exatamente as três tarefas 🔴/🟡 da
[triagem](00-triagem-despacho.md). Enquanto isso roda, quatro pedaços do backlog **não tocam nenhum arquivo do WIP de
B4** (`core/css/**`) e podem ir para sessões separadas — a mesma máquina/_working tree_, ou uma isolada, tanto faz,
porque os conjuntos de arquivo são disjuntos.

Isto não é o mesmo recorte da triagem: lá, "🟢 leve" significa "mecânico o bastante para conferir só pelo DoD". Aqui o
critério é mais estrito — **zero dependência de arquivo com `core/css`**, porque é isso que garante que rodar em
paralelo com B4 não gera conflito de `git`. Só quatro das sete fases passam nesse corte, e duas delas só passam
**parcialmente**.

## As quatro faixas seguras

| #   | O quê                                                                                                                                                                                                                          | Arquivo-fonte                                                                       | Toca `core/css`?                                                                                                                                                                                                                                              | Depende de B4?                                                                        |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| 1   | **B5 inteira** — tokenizer HTML5 + tree sink                                                                                                                                                                                   | [`02-b5-html-tokenizer.md`](02-b5-html-tokenizer.md)                                | Não — só `core/html/**`                                                                                                                                                                                                                                       | Não, só de B0 (entregue)                                                              |
| 2   | **X inteira** — decodificador PNG + `DrawImage`                                                                                                                                                                                | [`03-x-image-support.md`](03-x-image-support.md)                                    | Não — só `core/graphics/**` + `core/network/**` (reexport de `inflate`)                                                                                                                                                                                       | Não, só de C1 (entregue). O gatilho de _intrinsic sizing_ fica para I2/I4, não para X |
| 3   | **M, todos os itens exceto `css_bindings.rs`** — `css_cascade()`, `NETWORK_BINDINGS`, `WINDOW_BINDINGS`, `default_ui.rhai`/`default_network.rhai`, a matriz de pânico das duas tabelas novas, o _benchmark_ `hook_overhead.rs` | [`05-m-muscle-scripting.md`](05-m-muscle-scripting.md), itens 1–3, 5 (parcial), 6–7 | Não — `core/engine/src/domain/capability.rs` + `core/runtime/rhai-bindings/src/{net_bindings.rs, window_bindings.rs}` + `scripts/{default_ui,default_network}.rhai` + `rhai-bindings/tests/fault_injection.rs` + `core/runtime/rhai/benches/hook_overhead.rs` | Não, só de EE + C1 + C2 (todas entregues)                                             |
| 4   | **P, só os itens de `network`/`window`** — ADR-0018/0019 → `Accepted`, N-02, PRD-009, PRD-010, os dois _contract records_ de rede/janela                                                                                       | [`07-p-final-gates.md`](07-p-final-gates.md), itens 1–4 (parcial)                   | Não — só `docs/adr/`, `docs/requirements/`, `docs/architecture/{http-transport,window-system}-port-contract.md`                                                                                                                                               | Não, só de C0 + C1 + C2 (todas entregues)                                             |

Confirmado por busca antes de listar aqui — `css_cascade()` (item 3) ainda não existe:

```bash
grep -n "pub fn css_cascade\|pub fn network_interceptor\|pub fn ui_window" core/engine/src/domain/capability.rs
# → 98:    pub fn network_interceptor() -> CapabilitySet {
# → 104:    pub fn ui_window() -> CapabilitySet {
# (css_cascade não aparece — é trabalho real do item 1 de M, não uma checagem vazia)
```

## O que **não** entra nessa lista, mesmo parecendo leve

- **M, item 4** (`css_bindings.rs`, o adaptador de cascata scriptável) **e a parte de `cascade.rhai` do item 5** —
  registram `DomSnapshot`/`StyledTree` como `rhai::CustomType`. `StyledTree` é exatamente o agregado que B4 está mudando
  de forma agora (`ComputedStyle` ganhando `border`/`width`/`height`/`box-sizing`/`text-align`/`white-space`/`flex`).
  Ligar esse _binding_ contra um `StyledTree` que ainda está mudando de forma é trabalho perdido garantido.
- **P, o resto** — `style-cascade-port-contract.md` é o próprio freeze I3, que é **parte do DoD de B4**, não de P (já
  está listado assim em `01-b4-*.md`, item 10). A reescrita de "Current State" do `CLAUDE.md` e os portões de CI
  `css-conformance`/`layering` pressupõem saber o estado final de `core/css`/`core/html` — não dá para escrever isso de
  verdade antes de B4 e B5 estarem prontos.
- **I2 e I4** — dependem de B4 (e, no caso de I2, também de B5) estarem **compilando e testando verde**, não só de não
  colidir em arquivo. `LayoutBoxTree`/`CascadeResolver`/`LayoutEngine` são o meio da cadeia que I2 encadeia — não tem
  como escrever o `pipeline.rs` contra uma porta que ainda não compila.

## Regra de segurança ao rodar em paralelo

Cada sessão despachada deve fazer _stage_ e commit **só do seu próprio conjunto de arquivos** — nunca `git add -A`. O
WIP de B4 fica o tempo todo solto no _working tree_ (não commitado, propositalmente — ver `01-b4-*.md`), então um
`git add -A` de qualquer uma das quatro faixas acima commitaria acidentalmente o Flexbox pela metade junto com uma
tarefa que nada tem a ver com ele. É a mesma disciplina já usada nesta sessão para os pares B0/C0, B1/C1, B2/C2 e EE/B3
— despachados em paralelo, cada um commitado com `git add <caminhos exatos>` depois de conferido, nunca com um `add`
genérico.

Antes de cada commit dessas quatro faixas, rode `git status --short` e confirme que a lista de arquivos
modificados/novos bate exatamente com a coluna "Toca" da tabela acima — se aparecer qualquer coisa em `core/css/`, pare
e investigue antes de commitar.

## Ganho real

As quatro faixas tiram do caminho crítico ~23–35 dias-dev `[modelado]` (B5: 10–15 d; X: 5–8 d; M itens 1–3: ~4–6 d dos
8–12 d totais de M; P itens 1–4: ~3–5 d dos 5–8 d totais de P) que, de outro modo, só começariam depois de B4 fechar. O
que **não** muda é o gargalo: I2 continua esperando B4 (e agora B5, que com isso já estará pronta) antes de poder
começar — paralelizar aqui encurta a fila depois de B4, não a fase B4 em si.
