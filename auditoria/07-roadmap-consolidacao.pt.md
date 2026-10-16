# 07 · Roadmap consolidado da auditoria

## 1. Resumo executivo
- O AtlasGit tem um core sólido (grafo rápido, motor de operações com Undo, 111 testes de Rust e 81 de UI passando) e o MVP "funcional" está pronto no papel. O que pesa para o dono está fora do core: **recarga total a cada operação**, **blur de vidro em painéis grandes**, **cache de grafo sem limite**, **modais genéricos**, **ausência de animações**, **editor de conflitos básico**, **sem Git Flow**, **sem sessão/recentes** e **sem tela de Settings**.
- **48 achados** nas 6 áreas: 0 críticos, 12 ALTA, 25 MÉDIA, 11 BAIXA. Matriz: 37 itens da documentação (16 implementados, 7 parciais, 14 ausentes) mais 5 pedidos do dono.
- Três correções baratas atacam a sensação de peso: tirar o blur das laterais (PERF-02), limitar o cache do grafo (PERF-03) e adicionar logs/diagnóstico de queda (PERF-12).
- Lacuna de evidência: **nenhum número de fps, RAM ou tempo de troca de repositório foi medido**; o ambiente de nuvem não executa o app. A seção 7 diz como coletar isso na sua máquina.

## 2. Lista priorizada (ALTA → BAIXA)

| Prioridade | ID | Área | Título | Esforço |
|---|---|---|---|---|
| ALTA | PERF-01 | Desempenho | Toda operação recarrega o repositório inteiro | G |
| ALTA | PERF-02 | Desempenho | Blur de vidro em painéis grandes (duplicado por UI-04) | P |
| ALTA | PERF-03 | Desempenho | Cache de chunks do grafo nunca é liberado | P |
| ALTA | PERF-04 | Desempenho | Diff sem limite e sem virtualização | M |
| ALTA | PERF-12 | Desempenho | Sem log nem diagnóstico de queda | P-M |
| ALTA | FUN-01 | Funcionalidades | Conflitos só por bloco (duplicado por MOD-03) | G |
| ALTA | FUN-03 | Funcionalidades | Sem Git Flow nem padrão de branch | G |
| ALTA | FUN-04 | Funcionalidades | Criar branch não faz checkout nem tem opções (relacionado a MOD-01) | M |
| ALTA | FUN-05 | Funcionalidades | Sem recentes nem restauração de sessão | M |
| ALTA | MOD-01 | Modais | "Nova branch" é um modal pobre | M |
| ALTA | MOD-03 | Modais | Editor de conflito pequeno (duplicado de FUN-01) | G |
| ALTA | PKG-01 | Empacotamento | Flatpak nunca foi construído | G |
| MÉDIA | PERF-05, 06, 07, 08, 09, 10, 11 | Desempenho | async bloqueante, status pesado, inotify, remontagem do grafo, memória por aba, pintura do canvas, git sem timeout | P a M |
| MÉDIA | FUN-02, 06, 07, 08, 09, QA-01 | Funcionalidades | resolve sem journal, Settings, confirmações, rebase skip, recursos faltantes, corrida nas fixtures | P a G |
| MÉDIA | UI-01, 02, 03, 04 | UI | animações, ícones, barra, vidro | P a M |
| MÉDIA | MOD-02, 04 | Modais | base de modal, confirmações | M, P |
| MÉDIA | LAY-02, 03, 04 | Layout | centro apertado, diff, zoom | M |
| MÉDIA | PKG-02, 03, 07 | Empacotamento | nomes, atualização, teste manual | P a M |
| BAIXA | FUN-10, UI-05, 06, 07, MOD-05, 06, LAY-01, 05, PKG-04, 05, 06 | várias | polimento | P a M |

Duplicados (mantém o ID principal): UI-04 → PERF-02; MOD-03 → FUN-01; MOD-01 → FUN-03/FUN-04 (mesma tela, escopos complementares); LAY-05 → FUN-06; UI-03 → FUN-06; PERF-11 e FUN-10 (timeout/cancelamento de rede).

## 3. Ganhos rápidos (ALTA, esforço P)
1. **PERF-02**: remover `backdrop-filter` de `.left`/`.right` e reduzir nos overlays.
2. **PERF-03**: LRU de chunks do grafo e liberar ao fechar a aba.
3. **PERF-12** (P-M): `tracing` com arquivo de log e `panic::set_hook`.

Quase ganhos (MÉDIA, P) que valem junto: MOD-04 (um `ConfirmDialog` único), FUN-07 (confirmar reset hard e force delete), UI-02 (trocar ícones repetidos), PERF-06 (`--untracked-files=normal`), QA-01 (fixtures sem corrida).

## 4. Ordem sugerida por marcos

| Marco | Objetivo | Itens | Depende de |
|---|---|---|---|
| M0 · Evidência | medir antes de mexer | seção 7; PERF-12 (logs) | nada |
| M1 · Estabilidade e fluidez | app leve e que não cai | PERF-02, 03, 06, 05, 11, 04, depois PERF-01 e 08 (refresh incremental) e 09 | M0 para validar o ganho |
| M2 · Base de UI | componentes certos antes das telas | `Modal` com tamanhos, `ConfirmDialog` (MOD-02, 04), tabela de ícones (UI-02), tokens e animações (UI-01), plano de Settings (FUN-06) | nada; M1 não bloqueia |
| M3 · O que o dono pediu | Git Flow, branch, sessão | `settings.json` global e por repo (FUN-06), Nova branch (MOD-01, FUN-04), Git Flow e padrão `{ticket} {título}` (FUN-03), recentes e sessão (FUN-05) | M2 (Modal, Settings) |
| M4 · Conflitos | resolver como no GitKraken | tela cheia, base diff3, seleção por linha, navegação n/m, ferramenta externa (FUN-01, MOD-03), journal em `resolve_with` (FUN-02) | M2 |
| M5 · Cliente consolidado | uso diário completo | clone/init, busca no grafo, blame, autofetch, rebase Skip, zoom (FUN-08, 09, LAY-04) | M3 |
| M6 · Distribuição | publicar | PKG-01 a 07 | M1 a M4 estáveis |

## 5. Definição de "consolidado" (checklist objetivo)
- [ ] Abrir um repo de 100k commits e trocar de aba/repo em ≤ 1 s, sem travar a UI (medido, PERF-01/08).
- [ ] Rolar o grafo a 60 fps em notebook com GPU integrada (medido).
- [ ] RSS total do app (com WebKit) ≤ meta definida e estável após 1 h de uso (PERF-03/09).
- [ ] Qualquer queda gera log e mensagem na próxima abertura (PERF-12).
- [ ] Reabrir o app restaura abas e último repositório (FUN-05).
- [ ] Settings única com tema, zoom, idioma, animações, Git Flow e padrão de branch, tudo persistente (FUN-06).
- [ ] Criar branch com tipo/ticket/título, base e checkout; Git Flow iniciar/finalizar (FUN-03/04).
- [ ] Resolver conflito por linha, com navegação e saída ao vivo, e opção de abrir no editor (FUN-01).
- [ ] Todas as operações destrutivas têm confirmação ou Undo visível (FUN-07, MOD-04).
- [ ] Cada ação do menu tem ícone próprio; modais com tamanho padronizado e transições (UI-01/02, MOD-02).
- [ ] Flatpak, AppImage e .deb instalam e abrem em Ubuntu 22.04/24.04 (PKG-01/07).
- [ ] `cargo test`, `clippy`, `fmt --check` e testes de UI verdes em CI limpo (QA-01, PKG-05).

## 6. Branches sugeridas (fluxo do projeto: a partir de `develop`)
- `fix/perf-glass-and-chunk-cache` (PERF-02, 03)
- `feature/crash-diagnostics` (PERF-12)
- `fix/graph-fixtures-race` (QA-01)
- `feature/modal-base-and-confirm` (MOD-02, 04, FUN-07)
- `fix/menu-icons` (UI-02)
- `feature/settings-screen` (FUN-06)
- `feature/session-restore-and-recents` (FUN-05)
- `feature/branch-dialog-and-git-flow` (MOD-01, FUN-03, FUN-04)
- `feature/conflict-resolver-v2` (FUN-01, FUN-02)
- `feature/incremental-refresh` (PERF-01, 08)

## 7. Evidências a coletar na sua máquina (M0)
Comandos para gerar os relatórios que faltam, sem mudar código:
1. Fixture grande: `tests/fixtures/make.sh wide-100k`, abrir com `scripts/run.sh --release tests/fixtures/out/wide-100k`.
2. Pressionar `Ctrl+Shift+P` (painel de desempenho embutido) e registrar primeiro paint, tempo de desenho e fps ao rolar.
3. Memória: `ps -eo pid,rss,cmd | grep -E "atlasgit|WebKit"` antes e depois de 5 min rolando o grafo e abrindo 3 repositórios.
4. Trocar de repositório: cronometrar da abertura até o grafo aparecer, repetido com e sem o blur (testar `backdrop-filter: none` pelo inspetor ou editando `styles.css`).
5. Queda: abrir o app pelo terminal (`scripts/run.sh ... 2>&1 | tee atlasgit.log`) e registrar a saída quando fechar sozinho; `dmesg | grep -i -E "oom|killed"`.
6. Diffs grandes: abrir o diff de um `pnpm-lock.yaml` e medir o tempo até exibir.

Com esses números, a seção "Evidência" dos achados PERF-02/03/05/09/10 passa de hipótese para medição.
