# X — suporte a `<img>`

## Contexto

Sem esta fase, uma página com imagens sempre tem caixas vazias onde a imagem deveria estar. X decodifica PNG e desenha a
imagem no _backend_ de software, e ensina o layout a re-calcular o tamanho de uma caixa quando a imagem termina de
carregar. Depende de **B4** (o marcador `IntrinsicSize::Pending` já foi adicionado ao `LayoutBoxTree` em B4, antes do
congelamento I3 — ver `core/css/src/domain/computed/ intrinsic.rs`, já escrito e completo) e de **C1**
(`network::inflate`, o descompressor RFC 1951 que já existe dentro de `core/network`).

## Estado atual

```bash
ls core/graphics/src/infrastructure/png_decode.rs core/graphics/src/infrastructure/software/image.rs
# → No such file or directory (nenhum dos dois existe)
grep -n "pub fn inflate\|pub mod inflate" core/network/src/infrastructure/inflate.rs
# → confirma que o descompressor já existe e pode ser reexportado
```

`core/css/src/domain/computed/intrinsic.rs` já define `IntrinsicSize::{Resolved, Pending}` e a função `for_tag` que
marca `<img>`/`<video>`/`<canvas>`/`<iframe>`/`<object>`/`<embed>`/`<svg>` como `Pending` enquanto nenhum dos dois eixos
(`width`/`height`) foi fixado pelo autor — essa parte **já está pronta**, não precisa ser refeita aqui.

## Passos

1. **Reexportar `inflate` de `network`** — não criar um crate novo nem duplicar o descompressor. Se `network::inflate`
   ainda não é público na _facade_ do crate (`core/network/src/lib.rs`), adicione o `pub use` necessário lá antes de
   consumir de `core/graphics`.

2. **`core/graphics/src/infrastructure/png_decode.rs`** — decodificador PNG (assinatura de arquivo, chunks
   `IHDR`/`IDAT`/`IEND`, no mínimo `color_type` RGB e RGBA de 8 bits por canal — os dois casos que cobrem a grande
   maioria de imagens web reais) sobre `network::inflate`. Trata toda entrada como potencialmente hostil (ADR-0018 linha
   1: decodificador de bytes de rede é superfície de ameaça — `unsafe` proibido, erro tipado em vez de pânico em
   qualquer chunk malformado).

3. **`core/graphics/src/infrastructure/software/image.rs`** — implementa `DrawImage` no `SoftwareCpuBackend` (hoje
   recusado — ver o `match` de comandos em `core/graphics/src/infrastructure/software/mod.rs`). Escala por amostragem de
   caixa em inteiros, nunca filtro em ponto flutuante (ADR-0016 — mesma disciplina de `Au` que o resto do _backend_ já
   segue).

4. **Sizing intrínseco em `core/css`** — o layout já roda com caixa vazia (ou com `width`/`height` do autor) via o
   marcador `IntrinsicSize::Pending` de B4. O que falta é o **gatilho**: quando uma imagem termina de decodificar, o
   consumidor (que só existe a partir de I2/I4) precisa disparar uma re-cascata + relayout da árvore inteira. Essa
   fiação de gatilho e coalescência **mora em `alloy` (I2/I4), não aqui** — X só entrega o decodificador e o comando de
   desenho; não tente antecipar o pipeline.

5. **Alvos de fuzz** — `fuzz/fuzz_targets/{inflate.rs, png_decode.rs}` (o diretório `fuzz/` fica fora do workspace
   principal, como já documentado no plano original). Zero pânico esperado em 10 min por alvo.

## Crates de referência

- `core/graphics` — `infrastructure/software/mod.rs` (onde `DrawText` foi ligado em B3, no mesmo padrão de "antes
  recusado, agora implementado") e `infrastructure/font/ttf_provider.rs` (exemplo de decodificador de bytes hostis já
  revisado — bom molde de disciplina de erro tipado sem pânico).
- `core/network/src/infrastructure/inflate.rs` — o descompressor a reexportar, não duplicar.

## Definition of Done

- [ ] `cargo build -p graphics --all-targets` limpo.
- [ ] `cargo clippy -p graphics --all-targets --all-features -- -D warnings` limpo.
- [ ] Golden de imagem com um PNG real decodificado e desenhado.
- [ ] `fuzz/fuzz_targets/inflate.rs` e `png_decode.rs`: zero pânico em 10 min por alvo.
- [ ] Nenhum `unsafe` no decodificador (`ttf-parser` e `inflate` já estabeleceram o precedente de decodificador de bytes
      hostis 100 % seguro — mantenha o padrão).
- [ ] `cargo test --workspace` continua todo verde.

## Convenção de commit

```text
feat(graphics): PNG decode over network::inflate, DrawImage (v0.5 X)

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
```
