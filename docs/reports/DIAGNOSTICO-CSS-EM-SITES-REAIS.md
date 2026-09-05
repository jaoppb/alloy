# Diagnóstico — `alloy <url>` renderiza sites reais sem estilo

| Campo              | Valor                                                                                                                                                                                                                              |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Sintoma**        | `alloy <url>` contra sites reais abre a janela e pinta a página, mas **sem nenhum estilo de autor**                                                                                                                                |
| **Veredito**       | Não é regressão nem erro de ligação. O pipeline aplica CSS corretamente; o corte da v0.5 é estreito demais para folhas de estilo reais, e o parser amplificava isso                                                                |
| **Correções**      | 5 commits em `feat/v0-5` (`e2d3acd`, `056e996`, `efa1410`, `adada97`, `f5f0a6f`)                                                                                                                                                   |
| **Fora de escopo** | Alargar `SUPPORTED_PROPERTIES` / `SUPPORTED_SELECTORS` / cores / features de `@media` (corte deliberado da v0.5, `IMPLEMENTACAO-DETALHADA-V0-5.md` §2.8, é território v0.7); `var()`; brotli; HTTP/2; o swap de `rel` via `onload` |

---

## 1. O pipeline está correto

`alloy/src/application/pipeline.rs:198-201` faz o snapshot do DOM, `css::collect_style_sheets` junta `<style>` +
`style=`, o event loop busca `<link rel=stylesheet>` e `Session::absorb_stylesheet`
(`alloy/src/application/event_loop.rs:158`) o funde em `Origin::Author`, e então o resolvedor **real** `UaCascade` roda;
layout e paint leem o `ComputedStyle` já cascateado (`alloy/src/application/paint.rs:54/73/178`). Os testes golden
(`alloy/tests/render_golden.rs`, `e2e_golden.rs`) renderizam páginas **com** CSS de autor — inclusive uma folha `<link>`
externa — e as imagens abençoadas mostram o estilo aplicado. Não há stub nem caminho no-op. O refactor recente do
tokenizer HTML (`acba2bf`) preserva o texto de `<style>`.

## 2. A causa real

| #   | Causa                                                                                                                                                                                                                                                | Evidência                                                                                                                              | Certeza                                                                                                                                       |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Uma lista de seletores separada por vírgula era **descartada inteira** se **um** membro caísse fora do corte de 19 formas (`::before`, `:not()`, `:root`, `[attr^=]`, …)                                                                             | `core/css/src/infrastructure/parser/selectors.rs` (`?` no laço da vírgula, antes desta correção); `rules.rs:252-256` `skip_unreadable` | **Certa** — folhas reais (reset, framework, utilitárias) agrupam seletores e usam essas formas o tempo todo, então a maioria das regras sumia |
| 2   | Regras que sobrevivem perdem quase todas as declarações — só 33 propriedades no corte (`position`, `font-family`, `background`/`border`/`flex` shorthand, `overflow`, `gap`, `grid-*`, `line-height` ficam de fora)                                  | `core/css/src/lib.rs`; `rules.rs:337-343`                                                                                              | **Certa** — compõe com o #1                                                                                                                   |
| 3   | `@media` perdido em dobro: só `(min/max-width)` parseia (`media.rs:38-74`), e mesmo um bloco suportado nunca aplicava porque `alloy` não chamava `StyleSheetSet::matching_viewport`                                                                  | `core/css/src/domain/stylesheet_set.rs:260`; `cascade/author_rules.rs:97-99`                                                           | **Certa** para sites responsivos                                                                                                              |
| 4   | `<link rel>` casado por igualdade exata de string — `rel="preload stylesheet"`, `rel="Stylesheet"` ignorados; `<base href>` nunca consultado                                                                                                         | `subresource.rs:46` (antes desta correção)                                                                                             | Depende do site — comum em sites com tuning de performance                                                                                    |
| 5   | Fetch de subrecurso sem checagem de status: um corpo 404/500 (página HTML de erro) ia direto ao parser CSS, que faz ~zero regras e reporta sucesso; falha de parse de folha era silenciosa; todo `ParseNote` de descarte era computado e jogado fora | `event_loop.rs` `fetch_text` / `absorb_stylesheet` (antes desta correção)                                                              | **Certa** como cegueira de diagnóstico                                                                                                        |

## 3. O que mudou

1. **`fix(css)` `e2d3acd`** — `parse_selector_list` recupera **por seletor**: os membros dentro do corte ficam, cada
   refutado é pulado (cursor ressincronizado ao próximo `,` ou `{`, balanceando `(`/`[`) e registrado como `ParseNote`.
   A regra só cai inteira quando **nenhum** membro parseia. Desvio deliberado de Selectors L4 §3.1, documentado no
   cabeçalho do módulo e em `core/css/tests/data/MANIFEST.md:97`. Testes novos em `core/css/tests/authored_style.rs`.
2. **`fix(alloy)` `056e996`** — `render_dom_internal` calcula o `ViewportConstraints` antes da cascata e passa as folhas
   por `matching_viewport`, então `@media` suportado (`min/max-width`) finalmente aplica. Goldens seguem byte a byte
   idênticos (fixtures sem `@media`). Teste em `alloy/tests/render_golden.rs`.
3. **`fix(alloy)` `efa1410`** — `discover_stylesheet` trata `rel` como o conjunto de tokens case-insensitive que é;
   `discover` resolve o primeiro `<base href>` (WHATWG HTML §4.2.3) como base das referências relativas. Testes em
   `alloy/src/application/subresource.rs`.
4. **`fix(alloy)` `adada97`** — `fetch_text`/`fetch_image` rejeitam status não-2xx com o novo
   `AlloyError::SubresourceStatus`, que o event loop já loga em `warn!`. Teste em `alloy/src/application/event_loop.rs`.
5. **`feat(alloy)` `f5f0a6f`** — `ALLOY_LOG=alloy=debug,network=debug alloy <url>` passa a reportar:
   `stylesheet fetched` (URL, status, bytes), `stylesheet absorbed` (regras × notas), `stylesheet parse failed` (antes
   silencioso), `subresources discovered`, e o `css cascade input` final (regras que sobreviveram × total de descartes).
   Motivo de cada descarte em `alloy=trace`.

## 4. Expectativa após as correções

Sites reais **continuam parecendo quebrados** — o teto de 33 propriedades os esvazia. O que muda: regras simples (cor,
fundo, margem, alinhamento, flex por longhand) passam a aplicar em vez de nada, e `ALLOY_LOG=alloy=debug` mostra
quantitativamente o que cada site perde. Renderização fiel à web moderna depende de `core/js` (v0.7) e de alargar o
corte de CSS (v0.7+).

## 5. Verificação executada

- `cargo test -p css` — casos novos de recuperação por seletor, contagem de notas, sem regressão em `authored_style.rs`
  / `manifest_runner.rs`.
- `cargo test -p alloy` — `@media` aplica/não aplica; `rel="preload stylesheet"` e `<base href>`; status 404 não
  absorvido.
- `cargo test --workspace` — sem falhas.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — limpo.
- Manual: `ALLOY_LOG=alloy=debug alloy render <html com seletores fora do corte> -o /dev/null` →
  `css cascade input, rules: 2, notes: 2` para uma folha de 3 regras com `a::before` e `:not()`.
