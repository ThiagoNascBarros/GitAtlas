# Fase 1: Matriz documentação × código

Fontes da documentação: REA (vault Obsidian `AtlasGit/en`: PRD, SDD, roadmap, feature-map, ux-analysis, design-tokens) e documentação interna (`docs/HANDOFF.md`, `docs/specs`, `docs/a11y/audit.md`). Código em `develop` @ `02ec246`.

Legenda: **Impl.** = evidência no código (e teste, quando existe); **Parcial**; **Ausente**; **N/V** = não verificado (exige rodar o app). "Teste?" = existe teste automatizado que cobre.

## Funcionalidades (F)

| ID | Item | Origem | Status | Evidência | Teste? |
|---|---|---|---|---|---|
| F-001 | Abrir repo por argumento de CLI | PRD §5.1, feature-map | Impl. | `main.rs:284,580`, `App.tsx:293` | não (sem e2e) |
| F-002 | Abrir repo por diálogo de pasta | PRD §5.1 | Impl. | `App.tsx:256-259` | não |
| F-003 | Lista de repositórios recentes | PRD §5.1, feature-map "Repo management" (P0) | **Ausente** | nenhuma persistência de caminhos (só `localStorage` de largura/idioma) | n/a |
| F-004 | Vários repos em abas | PRD §5.1 | Impl. | `App.tsx:51,315-325` | não |
| F-005 | Restaurar sessão ("Remember tabs") | PRD §5.1 (session restore), feature-map P0 | **Ausente** | `tabs` só em memória (`App.tsx:51`) | n/a |
| F-006 | Tela de gestão de repos (Favoritos, Todos, Init, Clone) | feature-map P0/P1 | **Ausente** | sem comando `init`/`clone` em `main.rs:584-636` | n/a |
| F-007 | Grafo com lanes, arestas, refs | PRD §5.2 | Impl. | `GraphView.tsx`, `crates/ak-core/src/graph` | sim (`tests/graph.rs`, golden) |
| F-008 | Linhas WIP e stash no grafo | PRD §5.2 | Parcial | WIP: `GraphView.tsx:248,259`; stash como rótulo no commit base (desvio documentado no roadmap) | parcial |
| F-009 | Colunas configuráveis (redimensionar/ocultar/reordenar) | PRD §5.2 | Parcial | só redimensionar (`GraphView.tsx:64`, `columns.ts`); ocultar e reordenar ausentes; resize só por ponteiro (a11y F-14) | sim (`columns.test.ts`) |
| F-010 | Navegação por teclado no grafo | PRD §5.2 | Impl. | `GraphView.tsx:381-396` | sim (`a11y.test.ts`) |
| F-011 | Painel de detalhes do commit (lista caminho/árvore) | PRD §5.3 | Impl. | `App.tsx:523-586`, `files/tree.ts` | sim |
| F-012 | Diff hunk/inline/split | PRD §5.4 | Parcial | modos unified e split (`DiffView.tsx:15`); **sem** toggle de espaço em branco, **sem** syntax highlight, **sem** diff por palavra | não |
| F-013 | Stage/unstage de arquivo e hunk | PRD §5.5 | Impl. | `WipPanel.tsx`, `main.rs:437-455` | sim (`ops.rs`) |
| F-014 | Stage por linha, multisseleção, drag | feature-map P0 "melhoria" | **Ausente** | diferido no HANDOFF | n/a |
| F-015 | Commit e amend | PRD §5.5 | Impl. | `WipPanel.tsx:73-83` | sim |
| F-016 | Discard com backup recuperável | PRD §5.5, NFR-5 | Impl. | `WipPanel.tsx:85-93`, `ops/mod.rs` `undo_discard:606` | sim |
| F-017 | Criar, renomear, excluir branch; checkout | PRD §5.6 | Parcial | `ops/branch.rs`; criar **não faz checkout** (`git branch name sha`, `branch.rs:76`); sem opções (origem, upstream, checkout, tipo) | sim (`ops.rs:428`) |
| F-018 | Merge por menu/arrastar + previsão de conflitos | PRD §5.7 | Impl. | `RefMenu.tsx`, `merge_preview` `main.rs:383` | sim |
| F-019 | Ferramenta de conflito 3 painéis | PRD §5.7, feature-map P0 | Parcial | `ConflictEditor.tsx`: escolha por **bloco** (nossa/deles/ambas), sem linha a linha, sem navegação "n de m", sem base, sem ferramenta externa/IDE | sim só `markers.test.ts` |
| F-020 | Abortar merge/rebase | PRD §5.7 | Impl. | `op_merge_abort` `main.rs:400` | sim |
| F-021 | Stash push/pop/apply/drop | PRD §5.8 | Impl. | `ops/stash.rs`, `StashPanel.tsx` | sim |
| F-022 | Fetch/pull/push com credenciais do sistema | PRD §5.9 | Impl. | `op_net` `main.rs:490`, `askpass.rs` | sim (remoto bare local) |
| F-023 | Undo/Redo por journal | PRD §5.10 | Impl. | `ops/journal.rs`, `main.rs:552-561` | sim |
| F-024 | Temas Dark, Light, sistema | PRD §5.11 | Parcial | `theme.ts`; **escolha não é lembrada** (começa sempre em "sistema", HANDOFF) | sim (contraste) |
| F-025 | Zoom 80–200 % | PRD §5.11 | **Ausente** | sem ocorrência de zoom no código | n/a |
| F-026 | Shared Stash | PRD §5.13 / N0 | Impl. | `ops/shared_stash*.rs`, `features/shared-stash` | sim (31 + 102 linhas UI) |
| F-027 | Rebase e rebase interativo | PRD §6 | Parcial | `ops/rebase.rs`; faixas com merge recusadas; **sem Skip** do commit | sim |
| F-028 | Cherry-pick, revert, reset, tag | PRD §6 | Impl. | `ops/history.rs`, `CommitMenu.tsx` | sim |
| F-029 | Fast-forward como ação | feature-map P1 | **Ausente** (ação dedicada) | só aparece como texto de previsão | n/a |
| F-030 | Blame e histórico de arquivo | PRD (N, feature-map P1) | **Ausente** | | n/a |
| F-031 | Busca no grafo | feature-map P1 | **Ausente** | só filtro de refs | n/a |
| F-032 | Paleta de comandos / atalhos remapeáveis | PRD N5, feature-map P1 | **Ausente** | | n/a |
| F-033 | Worktrees na UI | feature-map P2 | **Ausente** | | n/a |
| F-034 | Tela de Preferências/Settings | ux-analysis §2.7 (preferências em página inteira) | **Ausente** | só seletores de tema e idioma na barra (`App.tsx:350-366`) | n/a |
| F-035 | Assinatura de commit (GPG/SSH) | feature-map P1 | **Ausente** | | n/a |
| F-036 | Auto-fetch por intervalo | feature-map P0 "melhoria" | **Ausente** | | n/a |
| F-037 | Stash parcial | PRD N6 | **Ausente** | stash sempre `include_untracked: true` (`main.rs:423`) | n/a |

## Fora da documentação do REA (pedidos do dono nesta auditoria)

| ID | Item | Status | Evidência |
|---|---|---|---|
| X-001 | Suporte a **Git Flow** (iniciar no repo; escolher feature/release/fix/hotfix ao criar branch) | **Ausente** | nenhuma ocorrência de gitflow no código; só a convenção de branches do próprio projeto no `HANDOFF.md` |
| X-002 | **Padrão de nome de branch personalizável** (ex.: `ACT-1010 título`) | **Ausente** | `NameDialog` aceita só texto livre (`NameDialog.tsx`); validação só `check-ref-format` (`branch.rs:32`) |
| X-003 | Reabrir no último repositório / nos últimos repositórios | **Ausente** | = F-003/F-005 |
| X-004 | Resolver conflitos escolhendo linhas, como no GitKraken | **Parcial** | = F-019 |
| X-005 | Animações de abrir/fechar componentes | **Ausente** | `design-tokens.md §6` define `motion.duration` 90/160 ms e easing, mas `styles.css` só tem transição no splitter (`:332`) e o spinner |

## Requisitos não funcionais (NF)

| ID | Requisito | Status | Evidência |
|---|---|---|---|
| NF-001 | Layout de 100k commits ≤ 400 ms; primeira janela ≤ 300 ms | Impl. no core | `report-wide-100k.json`: layout 4 ms, janela 0,08 ms. Primeiro paint da UI não medido |
| NF-001b | Scroll a 60 fps | N/V | benchmark declara "not_measured" |
| NF-002 | RSS ocioso ≤ 250 MB com 100k commits | Parcial | HANDOFF: PSS ~231 MB, **RSS ~390 MB**; o critério foi trocado de RSS para PSS |
| NF-003 | Inicialização ≤ 1,0 s | N/V | |
| NF-004 | Git ≥ 2.20 com fallbacks | Impl. | `caps.rs`, CI com Git 2.20.5 e "system" (`ci.yml:14-33`); PRD pede também 2.34 |
| NF-005 | Snapshot antes de operação que perde trabalho | Parcial | discard, reset hard, drop stash com recuperação; force delete de branch e reset hard **sem confirmação** (dependem de Undo) |
| NF-006 | Segurança (UI sem fs, sem `safe.directory=*`) | Impl. | `capabilities/default.json`, `exec.rs:93` |
| NF-007 | A11y WCAG 2.2 AA | Parcial | `docs/a11y/audit.md`; diferidos F-14, F-15, F-23; Orca não testado |
| NF-008 | Wayland/X11, portals XDG, `.desktop` | N/V | `.desktop` existe; Wayland/portal não confirmados |
| NF-009 | i18n en + pt-BR | Impl. | `i18n.ts`, `locales/pt-BR.ts` tipado, `i18n.test.ts` |
| NF-010 | Offline | N/V | |

## Critérios de aceitação do PRD §9

| # | Critério | Status |
|---|---|---|
| 1 | Abrir por CLI em ≤ 1 s | N/V |
| 2 | Grafo igual aos golden | Impl. (`tests/graph.rs`) |
| 3 | 100k commits no benchmark | Parcial (só core) |
| 4 | Stage de hunk e commit | Impl. em teste do core; teste manual do app pendente (HANDOFF) |
| 5, 6, 7, 8 | Merge com conflito, abort, undo, stash | Impl. (`tests/ops.rs`) |
| 9 | Remoto bare local | Impl. (`tests/ops.rs`) |
| 10 | Troca de tema + contraste | Impl. (`scripts/contrast.mjs`: 0 falhas) |
| 11 | Sem `safe.directory=*`; UI sem fs | Impl. por inspeção |
| 12 | Flatpak, AppImage, .deb em 22.04/24.04 | Parcial: .deb e AppImage rodaram sob Xvfb; **Flatpak nunca foi construído** (HANDOFF) |
| 13 | Matriz de Git 2.20, 2.34, latest | Parcial: 2.20.5 e system |
| 14 | Shared Stash | Impl. (`tests/shared_stash.rs`) |

## Divergências entre a doc do REA e a doc interna
- O REA define motion tokens (90/160 ms); a implementação não os usa (X-005).
- O REA prevê Preferências em página inteira; o repo adotou seletores na barra e `settings.rs` só guarda `sharedStash.fetchMode`.
- `HANDOFF.md` diz que a Fase 5 ainda está em branch de integração, mas o log do git mostra o merge da Fase 5 em `develop` (`02ec246`). Documento desatualizado.
