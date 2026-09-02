# Benchmarks web comparativos, requisitos mínimos de sistema e simulação de uso

| Campo               | Valor                                                                                                                             |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| **Status**          | ❌ Não iniciado — não existe `bench/`, nem `Dockerfile`, nem `criterion`, nem diretório `benches/` no repositório                 |
| **Cobertura**       | 0 de 4 suítes executáveis contra o Alloy hoje; 0 de 10 critérios novos (C-19 … C-28)                                              |
| **Esforço**         | 51–78 dias-dev `[modelado]` no escopo completo; 23–35 d para o primeiro comparativo publicável só com navegadores de referência   |
| **Depende de**      | Nada para B0–B3 (só navegadores de referência). B5 exige `core/window` + `devtools` (F8); perfis com Alloy exigem a v0.5          |
| **Atenção**         | ⚠️ Nenhuma das quatro suítes roda no Alloy antes da **v0.7**; três delas nunca rodam sem `canvas2d`/`webgl`, hoje fora do roadmap |
| **Fecha requisito** | `PRD-009` integral · evidência adicional para C-15, C-16, C-17 (cascata gráfica do `ADR-0009`)                                    |

Este relatório é o plano de implementação da funcionalidade pedida: **rodar Speedometer, JetStream, MotionMark e
Basemark Web, em Docker, nas versões estáveis mais recentes dos navegadores de mercado e no navegador deste
repositório**, para (a) posicionar o Alloy, (b) derivar os requisitos mínimos de sistema e (c) simular uso padrão e
avançado. Nada aqui foi implementado — o entregável desta rodada é a documentação: `PRD-009`, `ADR-0016`, `ADR-0017` e
`docs/architecture/benchmark-harness.md`.

> **Ressalva de escopo, dita uma vez e no lugar certo.** O objetivo declarado — "ajudar a definir esse navegador por
> cima dos demais" — não é alcançável pelas quatro suítes, e não por falha de engenharia: o Alloy hoje não tem DOM,
> HTML, CSS, JS nem janela (§1), e quando tiver, será um interpretador (`boa_engine`) contra motores com JIT. O plano
> abaixo entrega o escopo **integral** que foi pedido e acrescenta o eixo onde a comparação é honesta e favorável: a
> suíte de maleabilidade do `PRD-009` §7 (hot-reload, overhead de hook, footprint, custo de customização), que os
> navegadores de referência sequer conseguem executar. As duas coisas convivem no mesmo harness e no mesmo relatório.

---

## 1. Estado atual — evidências

Cada ausência abaixo é uma busca com resultado zero, executada no commit atual:

```bash
ls bench            # → ls: cannot access 'bench': No such file or directory
find . -name "Dockerfile*" -o -name "compose*.yml" | grep -v target   # → 0 resultados
find . -type d -name benches -not -path "./target/*"                  # → 0 resultados
grep -rn "criterion" --include=Cargo.toml .                           # → 0 resultados
grep -rln "Speedometer\|JetStream\|MotionMark\|Basemark" .            # → só os documentos criados nesta rodada
```

**Não existe nenhuma medição de desempenho no projeto.** A CI tem seis jobs (`.github/workflows/ci.yml:24-129`:
`markdown`, `rust-quality`, `test`, `supply-chain`, `arch-lint`, `coverage`) e nenhum deles mede tempo, memória ou
quadros. O único número de desempenho de toda a especificação é o `<10μs` por hook de `PRD-001:96`, e ele não é aferido
por nada — o portão de `criterion` que o guardaria só entra na v0.5 (`ROADMAP-IMPLEMENTACAO-V1.md` §5).

**O sujeito a medir ainda não existe como navegador.** Os crates de conteúdo continuam sendo stubs de 8 linhas:

```bash
wc -l core/{css,dom,html,js,graphics,window}/src/lib.rs
# → 8 linhas cada — apenas doc-comment e #![forbid(unsafe_code)]
```

O que existe e roda é `core/engine` (78 linhas em `lib.rs`), `core/runtime/rhai` e o binário `alloy` (75 linhas), que
executa um `.rhai` sob sandbox. Isso é a v0.1 "O engine vive" — e é ortogonal a qualquer benchmark web.

**Não há requisito de sistema publicado.** `README.md` e os oito PRDs não declaram CPU, RAM, GPU nem versão de SO
mínimos. A promessa do `ADR-0009` (Vulkan → OpenGL → software) implica um envelope de hardware largo, que hoje é
desconhecido em ambas as pontas.

---

## 2. O que cada suíte exige × o que o Alloy tem

| Suíte              | Mede                                            | Exige                                                            | Roda no Alloy hoje | Roda quando                                             |
| ------------------ | ----------------------------------------------- | ---------------------------------------------------------------- | ------------------ | ------------------------------------------------------- |
| **Speedometer 3**  | Responsividade em workloads de frameworks reais | ES2020, DOM L3 + eventos, Flexbox, SVG, canvas, `history`, fetch | ❌                 | Depois de F9 **e** F10; SVG/canvas hoje fora do roadmap |
| **JetStream 2**    | JS/Wasm: start-up, throughput e latência        | ES2020, **WebAssembly**, workers                                 | ❌                 | Subconjunto sem Wasm após F10 (v0.7) — ver V-06         |
| **MotionMark**     | Complexidade de animação sustentável a 60 fps   | canvas2d, SVG, animação CSS, `rAF`, vsync real                   | ❌                 | Sem previsão — nenhum item de roadmap entrega canvas    |
| **Basemark Web 3** | Bateria ampla HTML5/CSS/JS/**WebGL**, hospedada | Tudo acima + WebGL 1.0                                           | ❌                 | WebGL é **não-objetivo** declarado (`PRD-001:44`)       |

Três leituras que decidem o desenho do harness:

1. **A ausência precisa ser um dado, não um zero.** Um harness que reporta `0` esconde a razão e convida a comparação
   errada. O `PRD-009` §4.4 define a regra: sonda de capacidades → `status: unsupported` + lista de features faltantes,
   **sem campo de score**. A lista que encolhe release após release é o placar de progresso do Alloy.
2. **Duas suítes são auto-hospedáveis, duas não são.** Speedometer e JetStream entram vendorizadas por SHA e servidas
   por um mirror local; MotionMark exige vsync e GPU real (só lane de laboratório); Basemark Web é serviço proprietário
   hospedado — manual, externo, nunca bloqueante, e com revisão de termos de uso antes de qualquer publicação (`PRD-009`
   V-03).
3. **Safari não entra.** Não há build Linux e não roda em container. Entra WebKitGTK como proxy de família de motor,
   rotulado como proxy em todo manifesto e em toda tabela (`ADR-0016`).

---

## 3. A solução, em uma página

- **`ADR-0016`** — sujeitos em containers OCI fixados por digest, suítes servidas por mirror local sem egress, tiers
  como limites de cgroup, e **duas lanes**: CI (runner compartilhado, só detecta auto-regressão, **nunca** publica
  comparativo) e laboratório (bare metal fixado, GPU real, refresh fixo — a única fonte de número publicável).
- **`ADR-0017`** — quatro tiers de hardware (T0 piso 2 vCPU/2 GiB software … T3 16 vCPU/32 GiB Vulkan) × dois perfis de
  uso (padrão e avançado). **Mínimo** = menor tier que cumpre todos os orçamentos do perfil padrão; **recomendado** =
  menor tier que cumpre os do perfil avançado. Toda alegação nomeia o tier gráfico do `ADR-0009` que assumiu.
- **`PRD-009`** — porta `BenchmarkSubject` sob o contrato do `ADR-0011` (Alloy é _um_ sujeito entre quatro, com
  `MockSubject` e suíte de conformidade), sonda de capacidades, manifesto `run.json` versionado, estatística por mediana
  e IQR com intervalo de confiança por bootstrap, e a suíte de maleabilidade do Alloy.
- **`docs/architecture/benchmark-harness.md`** — topologia, layout de `bench/`, flags de container, chaves de sonda,
  schema do manifesto, CLI (`just bench …`) e os três workflows de CI.

O Alloy é dirigido pelo canal `bench` do crate `devtools` (protocolo de introspecção de `PRD-001:70`), com métricas
saindo como spans `tracing` (`ADR-0014`) — **sem CDP** e **sem afrouxar o sandbox** do `PRD-003`: o canal é gated por
capability e desligado por padrão.

---

## 4. Fases

Trilha **E** (medição), paralela às quatro trilhas do roadmap. B0–B3 e B6 não dependem de nenhuma linha nova de Alloy —
podem começar hoje e já produzem o comparativo entre Chrome, Firefox e WebKitGTK.

| Fase   | Conteúdo                                                                                                              | Entregável verificável                                          | Esforço `[modelado]` |
| ------ | --------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- | -------------------- |
| **B0** | Crate `bench/runner`: `domain/` (`Tier`, `FeatureSet`, `RunManifest`), porta `BenchmarkSubject`, `MockSubject`, CLI   | **C-28**: run ponta a ponta sem nenhum navegador presente       | 6–9 d                |
| **B1** | Imagens `mirror`/`chrome`/`firefox`/`webkit`, ponte Playwright, Speedometer + JetStream vendorizados por SHA          | **C-19**, **C-20**: 3 navegadores, manifesto válido, sem egress | 8–12 d               |
| **B2** | Estatística (mediana, IQR, bootstrap), gerador de relatório, regras de lane e de tier no renderizador                 | **C-25** (metade), tabela recusando misturar lanes              | 5–8 d                |
| **B3** | Tiers T0–T3 em cgroup, verificação de dentro do container, sweep                                                      | **C-22**: delta T0 × T3 visível na mesma suíte                  | 4–6 d                |
| **B4** | Perfis padrão/avançado, corpus congelado, métricas de sessão (start, RSS, tab-switch, input-to-paint, drop de quadro) | **C-23**: 10 repetições dentro do piso de ruído                 | 10–15 d              |
| **B5** | Sujeito Alloy: canal `bench` no `devtools`, sink JSON de `tracing`, `alloy --benchmark-capabilities`, sonda           | **C-21**: manifesto `unsupported` com lista de features         | 8–12 d               |
| **B6** | CI: `bench-smoke` (PR), `bench-nightly` (regressão), `bench-lab` (manual); calibração e tabela de requisitos          | **C-24**, **C-25**, **C-26**                                    | 6–9 d                |
| **B7** | Lane de laboratório: MotionMark com refresh fixo e GPU, Basemark manual pós-revisão de ToS; suíte de maleabilidade    | **C-27** + primeira publicação comparativa                      | 4–7 d                |

**Ordem não negociável em dois pontos.** **B2 antes de B6**: um gate de regressão sem piso de ruído calibrado gera falha
aleatória e é desligado na primeira semana. **B5 depois de F8**: sem janela e sem event loop, o sujeito Alloy não tem o
que instrumentar além do que o `criterion` já mediria melhor.

**Encaixe no roadmap**: B0–B3 entregam junto da **v0.3**; B4 e B6 acompanham a **v0.5** (primeira versão com sessão de
uso real); B5 e B7 fecham na **v0.9**, e a tabela de requisitos mínimos vira item de release da **v1.0**, ao lado da
fase F13 de endurecimento (`ROADMAP-IMPLEMENTACAO-V1.md:270`).

---

## 5. Como o requisito mínimo é derivado

Nunca de um score. O procedimento do `ADR-0017`, resumido:

1. Perfis padrão e avançado rodam em T0, T1, T2 e T3, `n ≥ 10`, mediana com IQR.
2. Um tier **falha** um perfil quando qualquer métrica primária estoura o orçamento fora do piso de ruído.
3. **Mínimo** = menor tier que passa em todos os orçamentos do perfil padrão. **Recomendado** = menor tier que passa nos
   do perfil avançado.
4. A tabela publicada leva a evidência junto: qual orçamento falhou no tier abaixo, por quanto, em qual manifesto.
5. Regeneração a cada release; piso que sobe é discussão de release, não edição silenciosa de documentação.

Os oito orçamentos iniciais (start a interativo, input-to-paint p95, tab-switch p95, RSS em repouso, crescimento de RSS
por hora, overhead de hook p99, swap de hot-reload p95, taxa de quadro perdido) estão em `ADR-0017` e são todos
`[modelado]` — a primeira calibração no host de laboratório substitui cada um por uma linha de base medida, exatamente
como o roadmap §5 já prescreve para os seus portões.

---

## 6. Armadilhas

1. **Citar número da lane de CI.** Runner compartilhado com vCPU de nuvem não sustenta comparativo entre navegadores. A
   proibição está codificada no gerador de relatório (`PRD-009` I-07), não na boa vontade de quem escreve o release.
2. **Comparar com número publicado por fabricante.** Metodologia, hardware e build diferentes; é a falácia clássica de
   benchmark. Só se compara resultado que este harness produziu, na mesma lane, tier e sessão.
3. **MotionMark em rasterizador de software.** O score se ajusta ao refresh rate; sem vsync real ele mede o driver de
   software, não o navegador. Lane de laboratório com refresh fixo, ou nada.
4. **Tag flutuante de imagem.** `chrome:latest` transforma a série histórica em ficção. Digest, sempre — e a troca de
   digest re-baseia toda a linha de tendência, o que precisa ser um evento anotado.
5. **`0` como score.** Confunde "não roda" com "roda mal". O schema separa os dois estados e o renderizador respeita a
   diferença.
6. **Abrir o sandbox para automatizar.** A tentação é expor um canal de controle sem capability. O canal `bench` é gated
   e desligado por padrão; se o benchmark exigir afrouxar `PRD-003`, o benchmark é que está errado.
7. **Vendorizar suíte sem licença.** Cada checkout carrega o `LICENSE` upstream e a atribuição; Basemark não é
   vendorizável de forma alguma.
8. **Corpus de páginas apodrecendo.** O corpus dos perfis é congelado e versionado; atualizá-lo re-baseia as métricas de
   sessão e precisa ser tratado como quebra de série.

---

## 7. Riscos

| Risco                                                                          | Prob. | Impacto | Mitigação                                                                                      |
| ------------------------------------------------------------------------------ | ----- | ------- | ---------------------------------------------------------------------------------------------- |
| Termos do Basemark Web proíbem execução automatizada ou publicação comparativa | Média | Médio   | V-03 antes de B7; lane manual e relatório interno se necessário — o harness não depende dele   |
| Nenhuma suíte roda no Alloy até a v0.7, esvaziando o comparativo               | Alta  | Alto    | Placar de cobertura de features + suíte de maleabilidade dão sinal desde a v0.3                |
| Host de laboratório indisponível ou aposentado                                 | Média | Alto    | Manifesto guarda todos os fatos do host; troca de host é quebra de série declarada, não oculta |
| Ruído de container maior que o efeito medido                                   | Média | Alto    | Calibração obrigatória (C-26); limiar derivado do piso de ruído, nunca um percentual inventado |
| Passthrough de GPU indisponível no host                                        | Média | Médio   | Tiers T1–T3 degradam para OpenGL/software e o manifesto registra; MotionMark fica bloqueado    |
| Manutenção recorrente das imagens vira dívida                                  | Alta  | Médio   | Atualização de digest é PR mensal com re-baseline explícito; falha de build é gate do smoke    |
| `boa_engine` não cobre subconjunto útil do JetStream                           | Média | Médio   | V-06 mede antes de prometer; resultado parcial declarado por subteste                          |

---

## 8. Verificação

Nada abaixo foi executado. Nenhum item nasce marcado.

- [ ] `just bench suite speedometer chrome T2` produz `run.json` validado contra o schema v1.
- [ ] Run completo com egress externo bloqueado (mirror por loopback), Basemark excluído e declarado.
- [ ] Sujeito sem features exigidas termina em `unsupported`, com lista de faltantes e **sem** campo de score.
- [ ] Limites de cgroup do tier conferidos de dentro do container e registrados no manifesto.
- [ ] 10 repetições do perfil padrão no mesmo host ficam dentro do piso de ruído calibrado.
- [ ] Gerador de relatório recusa tabela com linhas de lanes ou tiers diferentes.
- [ ] `MockSubject` passa na suíte `bench-conformance` sem nenhum navegador instalado.
- [ ] `bench-nightly` falha em regressão estatisticamente significativa e passa em ruído.
- [ ] Tabela de requisitos mínimos gerada a partir de manifestos, com o orçamento que falhou no tier abaixo.
- [ ] `cargo tree -p engine` continua sem interpretador e `arch-lint check` continua verde após a entrada de `bench/`.

---

## 9. Arquivos tocados

| Caminho                                                                   | O que acontece                                                              |
| ------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `docs/requirements/PRD-009-browser-benchmark-and-performance-harness.md`  | **novo** — requisitos, suítes, lanes, tiers, perfis, C-19 … C-28            |
| `docs/adr/0016-containerized-cross-browser-benchmark-harness.md`          | **novo** — MADR: containers, digests, duas lanes, WebKitGTK como proxy      |
| `docs/adr/0017-performance-tiers-and-minimum-system-requirements.md`      | **novo** — MADR: tiers, perfis, orçamentos, o que pode ser alegado          |
| `docs/architecture/benchmark-harness.md`                                  | **novo** — topologia, `bench/`, flags, sonda, schema `run.json`, CLI, CI    |
| `docs/adr/README.md`, `docs/README.md`                                    | índices atualizados                                                         |
| `docs/reports/ROADMAP-IMPLEMENTACAO-V1.md`                                | nota de extensão: trilha **E**, fases B0–B7, novos portões                  |
| `bench/**`                                                                | **futuro** — crate `bench-runner`, imagens, suítes vendorizadas, perfis     |
| `Cargo.toml`, `justfile`                                                  | **futuro** — membro `bench/runner`; receitas `just bench …`                 |
| `.github/workflows/bench-smoke.yml`, `bench-nightly.yml`, `bench-lab.yml` | **futuro** — os três workflows do §7 da referência de arquitetura           |
| `devtools/`, `alloy/`                                                     | **futuro** — canal `bench` gated por capability, `--benchmark-capabilities` |
