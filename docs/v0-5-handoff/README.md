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
- `docs/reports/V0-5-PROGRESSO-E-PENDENCIAS.md` — o estado exato de tudo que já foi verificado nesta sessão, com
  evidência `arquivo:linha`.
- O `CLAUDE.md` da raiz do repositório — arquitetura, Object Calisthenics, comandos, convenções de commit.

O plano original e completo (todas as 16 fases, incluindo as já entregues) continua em
`~/.claude/plans/verifique-o-docs-reports-implementacao-d-fancy-dijkstra.md`, mas **não é necessário lê-lo** para
trabalhar numa fase — cada arquivo aqui já extrai e atualiza a seção relevante dele.

## Estado em uma tabela

| Fase                              | Estado                                                           | Arquivo desta pasta                                                      | Commit                     | Despacho                                             |
| --------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------ | -------------------------- | ---------------------------------------------------- |
| 0, B0, C0, B1, C1, B2, C2, EE, B3 | ✅ entregues e verificadas (`just gate`)                         | — (ver relatório de progresso)                                           | `95c88bb` … `e07971d`      | —                                                    |
| **B4**                            | 🟡 em andamento — ~3.100 linhas escritas, `core/css` não compila | [`01-b4-box-model-inline-flexbox.md`](01-b4-box-model-inline-flexbox.md) | nenhum (WIP não commitado) | misto 🟢🟡🔴 — ver [triagem](00-triagem-despacho.md) |
| B5                                | ⏳ não iniciada                                                  | [`02-b5-html-tokenizer.md`](02-b5-html-tokenizer.md)                     | —                          | 🟡 médio                                             |
| X                                 | ⏳ não iniciada                                                  | [`03-x-image-support.md`](03-x-image-support.md)                         | —                          | 🟢 leve                                              |
| I2                                | ⏳ não iniciada                                                  | [`04-i2-headless-pipeline.md`](04-i2-headless-pipeline.md)               | —                          | 🟢 leve                                              |
| M                                 | ⏳ não iniciada                                                  | [`05-m-muscle-scripting.md`](05-m-muscle-scripting.md)                   | —                          | 🟢 leve, com uma releitura de capability             |
| I4                                | ⏳ não iniciada                                                  | [`06-i4-alloy-url.md`](06-i4-alloy-url.md)                               | —                          | 🔴 pesado                                            |
| P                                 | ⏳ não iniciada                                                  | [`07-p-final-gates.md`](07-p-final-gates.md)                             | —                          | 🟢 leve                                              |

## Grafo de dependência do que resta

```text
B4 (em andamento)
  ├──> B5 (pode começar em paralelo — só depende de B0, já entregue)
  │       ↓
  │      I2  (precisa de B4 + B5)  ──> push + PR draft "v0.5 · I2 render headless"
  │       ↓
X (precisa de B4 + C1, pode rodar em paralelo com I2)
  │       ↓
  M  (precisa de EE + B4 + C1 + C2 — todas já prontas menos B4)
       ↓
  I4  (precisa de I2 + C1 + C2 + M + X)  ──> push + PR draft "v0.5 · I4 alloy <url>"
       ↓
  P  (precisa de todas)  ──> PR final
```

Ordem recomendada para sessões separadas: **fechar B4 primeiro** (é o bloqueador de tudo — nada mais compila, testa ou
builda em cima de um `core/css` quebrado). Depois, B5 e X podem rodar em sessões paralelas. M só faz sentido depois de
B4. I2 precisa de B4+B5. I4 precisa de quase tudo. P é sempre a última.

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
