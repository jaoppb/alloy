# v0.5 — pacote de handoff para as fases restantes

Este é o índice de continuação da campanha de implementação da v0.5. Existe porque a campanha não cabe numa sessão só:
nove fases já foram entregues, e as sete que faltam são grandes demais para chegar ao fim antes de qualquer limite de
sessão. Cada arquivo abaixo é **autocontido** — uma sessão nova (ou um sub-agente novo) deve conseguir abrir só aquele
arquivo, mais o crate que ele referencia, e começar a trabalhar sem precisar ler o plano inteiro nem o histórico desta
conversa.

**Antes de abrir qualquer arquivo de fase**, leia:

- [`00-triagem-despacho.md`](00-triagem-despacho.md) — separa o que é mecânico o bastante para despachar a outra IA (ou
  a uma sessão sem contexto nenhum da campanha) e conferir só pelo `Definition of Done`, do que exige alguém
  acompanhando a decisão de design enquanto ela é tomada.
- [`PARALELO-COM-B4.md`](PARALELO-COM-B4.md) — dessas tarefas leves, quais **não tocam nenhum arquivo de `core/css`** e
  por isso podem rodar agora mesmo, em sessões separadas, enquanto uma IA mais forte fica dedicada só ao
  `01-b4-box-model-inline-flexbox.md`.
- `docs/reports/V0-5-PROGRESSO-E-PENDENCIAS.md` — o estado exato de tudo que já foi verificado nesta sessão, com
  evidência `arquivo:linha`.
- O `CLAUDE.md` da raiz do repositório — arquitetura, Object Calisthenics, comandos, convenções de commit.

O plano original e completo (todas as 16 fases, incluindo as já entregues) continua em
`~/.claude/plans/verifique-o-docs-reports-implementacao-d-fancy-dijkstra.md`, mas **não é necessário lê-lo** para
trabalhar numa fase — cada arquivo aqui já extrai e atualiza a seção relevante dele.

## Estado em uma tabela

| Fase                              | Estado                                                                                                                                                                                                       | Arquivo desta pasta                                                      | Commit                 | Despacho                                             |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ | ---------------------- | ---------------------------------------------------- |
| 0, B0, C0, B1, C1, B2, C2, EE, B3 | ✅ entregues e verificadas (`just gate`)                                                                                                                                                                     | — (ver relatório de progresso)                                           | `95c88bb` … `e07971d`  | —                                                    |
| **B4**                            | ✅ entregue e verificada (`cargo test -p css`, `clippy`, `just no-engine`)                                                                                                                                   | [`01-b4-box-model-inline-flexbox.md`](01-b4-box-model-inline-flexbox.md) | `1353647`              | misto 🟢🟡🔴 — ver [triagem](00-triagem-despacho.md) |
| B5                                | ✅ entregue                                                                                                                                                                                                  | [`02-b5-html-tokenizer.md`](02-b5-html-tokenizer.md)                     | `fc7d55e`              | 🟡 médio                                             |
| X                                 | ✅ entregue                                                                                                                                                                                                  | [`03-x-image-support.md`](03-x-image-support.md)                         | `3558e4c`              | 🟢 leve                                              |
| **I2**                            | ✅ entregue e verificada (`cargo test -p alloy`, golden e2e)                                                                                                                                                 | [`04-i2-headless-pipeline.md`](04-i2-headless-pipeline.md)               | `9bb8ae3`              | 🟢 leve                                              |
| **M**                             | ✅ entregue e verificada (`cargo test -p rhai-bindings`, bench)                                                                                                                                              | [`05-m-muscle-scripting.md`](05-m-muscle-scripting.md)                   | `067e87b`              | 🟢 leve, com uma releitura de capability             |
| **I4**                            | ✅ entregue e verificada (`cargo test --workspace`, `clippy`, golden e2e) — falta só o checkpoint de `push`/PR (ação compartilhada, aguardando confirmação do usuário) e a verificação manual de janela real | [`06-i4-alloy-url.md`](06-i4-alloy-url.md)                               | _pendente_             | 🔴 pesado                                            |
| **P**                             | 🟡 quase entregue — docs e a maioria dos portões de CI feitos; o portão `coverage` está honestamente vermelho (~66% < 85%) até mais testes de domínio serem escritos para `network`/`window`                 | [`07-p-final-gates.md`](07-p-final-gates.md)                             | `56d487d` + _pendente_ | 🟢 leve                                              |

## Grafo de dependência do que resta

```text
B4 ✅ ──> B5 ✅
  │         ↓
  │        I2 ✅ ──────────────────────────────────────────────┐
  │                                                             ↓
X ✅ ──>   M ✅ (precisa de EE + B4 + C1 + C2 — todas prontas) ─> I4 ✅ ──> push + PR draft "v0.5 · I4 alloy <url>" ⏳
                                                                 ↓
                                                            P 🟡 (docs + maioria dos portões feitos; coverage vermelho) ──> PR final ⏳
```

B4, B5, X, I2, M e I4 estão entregues (I4 ainda sem commit/push desta sessão). P está quase entregue — falta fechar o
portão `coverage` (~66% < 85%) com mais testes de domínio, e depois abrir o PR final. **Nenhum PR foi aberto e nenhum
push foi feito**: com várias sessões mexendo em `feat/v0-5` ao mesmo tempo nesta rodada, um `push`/PR é uma ação
compartilhada que precisa de confirmação explícita do usuário antes de qualquer sessão executar.

## Como usar cada arquivo de fase

Cada arquivo de fase segue o mesmo esqueleto:

1. **Contexto** — o que essa fase entrega e por que, em 2–3 frases.
2. **Estado atual** — o que já existe no repositório, com `arquivo:linha`, e o que falta, verificado por busca (nunca
   por suposição).
3. **Passos** — a lista ordenada do que escrever, na ordem em que faz sentido escrever.
4. **Crates de referência** — quais crates já entregues espelhar em estrutura e estilo.
5. **Definition of Done** — a lista exata de comandos e verificações que precisam passar antes de considerar a fase
   pronta.
6. **Convenção de commit** — a mensagem exata esperada.

Uma sessão (ou sub-agente) que pega um desses arquivos deve: ler o arquivo da fase, ler o `CLAUDE.md` da raiz, ler os
crates de referência citados, implementar, verificar contra o DoD, e commitar — **uma fase por commit**, na branch
`feat/v0-5`, sem dar push exceto nos checkpoints marcados (I2 e I4).

## Por que este pacote existe

A tentativa original era despachar um sub-agente por fase dentro da mesma sessão orquestradora, cada um lendo o plano
gigante inteiro (`~/.claude/plans/…-fancy-dijkstra.md`, > 700 linhas) mais o histórico da conversa. Isso funcionou para
as primeiras nove fases, mas tem dois custos que pioram a cada fase: o tempo para o agente reconstruir contexto antes de
escrever a primeira linha, e o risco de o agente morrer por limite de sessão da conta no meio do trabalho (aconteceu em
B0, B1, C1 e — pela quarta vez — em B4). Dividir o restante em arquivos pequenos e autocontidos resolve os dois
problemas: cada sessão nova começa a trabalhar imediatamente, e uma morte no meio do caminho perde no máximo uma fase,
nunca o pacote inteiro.
