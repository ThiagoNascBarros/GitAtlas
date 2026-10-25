# Matriz documentação × código

Fontes: documentação do REA (`/home/thigas/Documentos/Develop/Obsidian/AtlasKraken/en/`, PRD, SDD, roadmap, open questions) e documentação interna (`docs/`, `docs/HANDOFF.md`, `docs/specs/*`). Onde divergem, a do REA é a referência de escopo. Limitação: matriz montada por análise estática + greps + linha de base; itens "Implementado" cujo teste é `no` não têm teste automatizado. Os agentes da Fase 2 podem refinar status.

## Resumo

- Implementado: 30
- Parcial: 18
- Ausente: 13
- Não verificado: 5
- Total: 66

## Matriz

ID | item | origem na doc | status | evidência no código | teste?
---|---|---|---|---|---
F-001 | Abrir repositório por argumento de CLI | PRD §5.1, AC-1 | Implementado | app/src-tauri/src/main.rs:284 `get_initial_path` | não
F-002 | Abrir repositório por diálogo de arquivo | PRD §5.1 | Implementado | app/ui/src/App.tsx:4 `openDialog`; capabilities/default.json `dialog:allow-open` | não
F-003 | Lista de repositórios recentes | PRD §5.1 | Ausente | no `recent` in app/ui/src or app/src-tauri/src (grep) | —
F-004 | Vários repositórios em abas | PRD §5.1 | Implementado | app/ui/src/App.tsx:51 `tabs`, :84 `tab()`; main.rs `repo_open/repo_close` | não
F-005 | Restaurar sessão (reabrir abas) | PRD §5.1 | Ausente | only layout prefs in localStorage (columns.ts:53, Splitter.tsx:12, i18n.ts:427); no tab persistence found | —
F-006 | Grafo de commits: histórico completo, lanes/arestas, refs | PRD §5.2, AC-2 | Implementado | crates/ak-core/src/graph/{layout,mod,encode}.rs; GraphView.tsx | sim: ak-core/tests/graph.rs + golden/
F-007 | Linhas WIP e stash no grafo | PRD §5.2 | Parcial | stash is a label on the base commit, not a row (roadmap Phase 3 deviation); WIP row: verify in GraphView | não
F-008 | Colunas do grafo configuráveis | PRD §5.2 | Implementado | features/graph/columns.ts | sim: columns.test.ts
F-009 | Navegação por teclado e seleção no grafo | PRD §5.2 | Implementado | GraphView.tsx; features/graph/a11y.ts | sim: a11y.test.ts
F-010 | Painel de detalhes do commit (metadados, lista de arquivos) | PRD §5.3 | Implementado | main.rs `commit_details`; features/files/tree.ts | sim: tree.test.ts
F-011 | Diff hunk / inline / split | PRD §5.4 | Parcial | DiffView.tsx:15 only `unified`|`split` | sim: diff/parse.test.ts
F-012 | Alternar espaços em branco no diff | PRD §5.4 | Ausente | no `-w`/ignore-whitespace option in DiffView.tsx or `diff_file` | —
F-013 | Realce de sintaxe no diff | PRD §5.4, roadmap T1.9 | Ausente | no highlighter dependency (grep hljs/prism/shiki) | —
F-014 | Visualização do arquivo inteiro | PRD §5.4 | Ausente | not found in DiffView.tsx (confirmed by FUNC-2) | —
F-015 | Status, stage/unstage de arquivo e hunk | PRD §5.5, AC-4 | Implementado | ops/mod.rs `Stage`,`Unstage`,`StageHunk` | sim: ops.rs:51,66,251
F-016 | Descartar com backup recuperável | PRD §5.5, NFR-5 | Parcial | ops Discard; backup in `.git/atlaskraken/discarded/` (deviation from refs/atlaskraken/snapshots) | sim: ops.rs:210,230
F-017 | Commit e amend | PRD §5.5 | Implementado | ops `Commit{amend}` | sim: ops.rs:51,136
F-018 | Branch criar/checkout/renomear/excluir (com undo) | PRD §5.6 | Implementado | ops/branch.rs | sim: ops.rs:428,449,465
F-019 | Merge por arrastar ou menu com previsão de conflito | PRD §5.7, AC-5 | Implementado | ops/merge.rs; App.tsx:442 draggable; main.rs `merge_preview` | sim: ops.rs:502,524
F-020 | Ferramenta de conflito em 3 painéis | PRD §5.7 | Parcial | features/conflict/ConflictEditor.tsx (ours|theirs + result, per-conflict choice) | sim: conflict/markers.test.ts; ops.rs:593
F-021 | Abortar merge / operações em andamento | PRD §5.7, AC-6 | Implementado | ops `MergeAbort`; `in_progress()` | sim: ops.rs:635
F-022 | Stash push/pop/apply/drop | PRD §5.8, AC-8 | Implementado | ops/stash.rs; features/stash | sim: ops.rs:543,567
F-023 | Fetch / pull / push | PRD §5.9, AC-9 | Implementado | ops/remote.rs; main.rs `op_net` | sim: ops.rs:711,732,828
F-024 | Credenciais por helpers do sistema + ponte askpass | PRD §5.9 | Parcial | app/src-tauri/src/askpass.rs; helpers/ssh-agent work only through the user's git config | parcial: askpass.rs unit test only
F-025 | Undo/Redo pelo journal de operações | PRD §5.10, AC-7 | Implementado | ops/{mod,journal,lock}.rs | sim: ops.rs:94,111,287,308
F-026 | Cherry-pick, revert, reset soft/mixed/hard, tags | PRD §6, roadmap P4 | Implementado | ops/history.rs | sim: ops.rs:616,650,672
F-027 | Rebase e editor de rebase interativo | PRD §6 (P1) | Parcial | ops/rebase.rs; features/rebase; merges in range refused | sim: ops.rs:768,785,807; rebase/todo.test.ts
F-028 | Temas Atlas Dark/Light + sincronizar com o sistema | PRD §5.11, AC-10 | Parcial | themes/*.theme.json; app/ui/src/theme/theme.ts; choice not remembered (HANDOFF) | sim: scripts/contrast.mjs (0 failures)
F-029 | Zoom 80–200 % | PRD §5.11 | Ausente | no zoom/font-scale code (grep) | —
F-030 | Empacotamento Flatpak + AppImage + .deb | PRD §5.12, AC-12 | Parcial | scripts/package*.sh, packaging/; Flatpak never built (HANDOFF) | CI validates desktop/metainfo only
F-031 | Shared Stash: publicar/descobrir/prévia/aplicar/branch | PRD §5.13, §7 N0, AC-14 | Implementado | ops/shared_stash*.rs; app/src-tauri/src/shared_stash.rs; features/shared-stash | sim: ak-core/tests/shared_stash.rs (31); logic.test.ts
F-032 | Shared Stash: update/delete só do autor, opt-in por remote | PRD §7 N0, Q9 | Implementado | shared_stash_{update,unshare}; settings.rs `fetchMode` | sim: shared_stash.rs
F-033 | Shared Stash: verificação de segredos antes do push | PRD §7 N0 Safety | Ausente | no secret scan in shared_stash*.rs; spec shared-stash.md:231 only warns | —
F-034 | Shared Stash: limite de tamanho (padrão 5 MB) | PRD §7 N0 Safety | Parcial | shared_stash_ops.rs:473 `new_blob_bytes`; spec uses a 10 MiB best-effort warning, no hard limit | parcial
F-035 | Painel do journal de operações (N1) | PRD §7 N1 | Implementado | features/journal/JournalPanel.tsx; `journal_list/prune` | sim: ops.rs:397
F-036 | Comparar intervalo (N3) | PRD §7 N3 | Ausente | not present | —
F-037 | Visões salvas do grafo / filtros (N4) | PRD §7 N4 | Ausente | not present | —
F-038 | Paleta de comandos (N5) | PRD §7 N5 | Ausente | App.tsx:277 Ctrl+Shift+P toggles perf overlay only | —
F-039 | Stash parcial (N6) | PRD §7 N6 | Ausente | StashPush takes no paths | —
F-040 | Regras de identidade por diretório (N7) | PRD §7 N7 | Ausente | not present | —
F-041 | Busca/filtro de commits, blame | PRD §10 persona Rafael | Ausente | no blame/commit search (grep); only refs filter | —
NF-001 | Layout 100k ≤ 400 ms; primeira janela ≤ 300 ms | PRD NFR-1 | Implementado | tests/bench/report-wide-100k.json: layout 4.07 ms, first_window 0.08 ms; crates/ak-core/examples/bench.rs | sim (bench, core only)
NF-002 | Scroll a 60 fps, primeiro paint da UI | PRD NFR-1, G2 | Não verificado | report `not_measured`: UI first paint, scroll fps; GraphView.tsx/perf.ts | não
NF-003 | Memória: ≤120 B/commit; ocioso ≤250 MB | PRD NFR-2 | Parcial | 33 B/commit (report); HANDOFF: PSS ~231 MB, RSS ~390 MB (criterion redefined to PSS) | sim (bench)
NF-004 | Partida a frio ≤ 1,0 s | PRD NFR-3 | Não verificado | no measurement found | não
NF-005 | Git ≥ 2.20 por capacidades; matriz de CI | PRD NFR-4, AC-13 | Parcial | ak-gitcli/src/caps.rs; ci.yml:18-24 (system + 2.20.5; no 2.34 entry) | sim: caps.rs, CI
NF-006 | Snapshot antes de operações destrutivas | PRD NFR-5, G4 | Parcial | ops/mod.rs:32 SNAPSHOT_PREFIX; discard uses backup dir; journal_prune leaves -redo/-wip refs (HANDOFF) | sim: ops.rs:136,650
NF-007 | Segurança: sem safe.directory=*, API tipada, sem env na UI | PRD NFR-6, AC-11 | Implementado | ak-gitcli/src/exec.rs:93; capabilities/default.json; tauri.conf.json:16 CSP | sim: exec.rs:204
NF-008 | Acessibilidade WCAG AA, teclado, leitor de tela | PRD NFR-7 | Parcial | docs/a11y/audit.md (WCAG 2.1, not 2.2); deferred F-14/F-15/F-23; Orca untested | sim: a11y.test.ts, overlay.test.ts, contrast.mjs
NF-009 | Integração Linux: Wayland+X11, XDG, portais, .desktop | PRD NFR-8 | Parcial | packaging/linux/*.desktop; flatpak yml:17-18; font scaling not handled | CI validators only
NF-010 | i18n: textos externalizados, en + pt-BR | PRD NFR-9 | Implementado | app/ui/src/i18n.ts; locales/pt-BR.ts (typed parity) | sim: i18n.test.ts
NF-011 | Offline: recursos locais sem rede | PRD NFR-10 | Não verificado | no explicit test; local ops use CLI only | não
AC-001 | Abrir por CLI, grafo em ≤1 s no demo | PRD §9.1 | Não verificado | no automated e2e | não
AC-003 | Limites do benchmark de 100k (job de benchmark) | PRD §9.3 | Parcial | report-wide-100k.json is committed; no CI bench job (ci.yml has none) | parcial
AC-009 | Remote: fetch/pull/push com bare local | PRD §9.9 | Implementado | ops.rs:688 remote_pair | sim
AC-011 | Sem safe.directory=* no GIT_TRACE; UI sem permissão de fs | PRD §9.11 | Parcial | unit assertion on args (exec.rs:204); no GIT_TRACE e2e | parcial
AC-012 | Pacotes instalam/abrem no Ubuntu 22.04/24.04 Wayland+X11 | PRD §9.12 | Não verificado | deb/AppImage built under Xvfb (HANDOFF); no Wayland run; Flatpak not built | não
AC-014 | Shared Stash e2e com dois clones | PRD §9.14 | Implementado | ak-core/tests/shared_stash.rs | sim
UI-001 | Barra: fetch/pull/push, ahead/behind, undo/redo, idioma, tema | SDD layout; roadmap P4 | Implementado | App.tsx (lucide icons import :22; theme select :352) | não
UI-002 | Painel de refs com filtro, remotos recolhíveis, arrastar p/ merge | roadmap P2/P3 | Implementado | App.tsx:442; features/refs | sim: refs/filter.test.ts
UI-003 | Painéis redimensionáveis (split) | SDD layout-direction | Implementado | features/split/Splitter.tsx | não
UI-004 | Menus de contexto (ref, commit, stash, shared stash) | PRD §6 (native menus) | Parcial | in-app menus (RefMenu, CommitMenu, LocalStashMenu, SharedStashMenu), not OS-native (deviation) | não
UI-005 | Diálogos: nome, compartilhar, rebase, conflito, credenciais | roadmap P3/P4 | Implementado | NameDialog, ShareDialog, RebaseEditor, ConflictEditor, askpass prompt; focus trap overlay.ts | sim: overlay.test.ts
UI-006 | Mensagens de erro humanas/toasts (códigos estáveis) | roadmap P2; ux-analysis (raw stderr toast) | Implementado | i18n.ts `err.<CODE>` + `.fix` | sim: i18n.test.ts
UI-007 | Painel WIP (listas stage/unstage, caixa de commit, banner) | PRD §5.5 | Parcial | features/wip/WipPanel.tsx; no multi-select/drag, no tree, no discard-all, no keyboard nav (HANDOFF) | sim: wip/status.test.ts
UI-008 | Barra de status | roadmap T1.7 | Implementado | app/ui/src/App.tsx:663 (per FUNC audit) | —

## Fora da documentação

ID | item | relação | local
---|---|---|---
X-001 | Seletor de idioma | NF-010 extension | app/ui/src/i18n.ts; App.tsx
X-002 | Retenção/limpeza do journal | — | main.rs; ops/mod.rs
X-003 | Mock IPC de dev (`devMock.ts`) | — | app/ui/src/devMock.ts
X-004 | Overlay de desempenho (Ctrl+Shift+P) | — | features/graph/perf.ts; App.tsx:277
X-005 | Workflow de release (só artefatos) e checklist | — | .github/workflows/release.yml; docs/release/checklist.md
X-006 | Guia do usuário docs/user (9 tópicos) | — | docs/user/

## Divergências REA × documentação interna

- Nome: REA usa AtlasKraken (final, Q5); o repositório usa AtlasGit; dados por repo mantêm `atlaskraken` (HANDOFF).
- Vault: HANDOFF cita `Obsidian/AtlasGit/`; o caminho real é `Obsidian/AtlasKraken/`.
- Memória: PRD NFR-2 diz RSS ≤250 MB; interno mede PSS (~231 MB) e RSS ~390 MB.
- Shared Stash: PRD pede limite de 5 MB e checagem de segredos; spec interna usa aviso de 10 MiB e sem checagem de segredos.
- Acessibilidade: PRD exige WCAG 2.2 AA; auditoria interna usa 2.1 AA.
- Stash: PRD fala em linhas de stash no grafo; implementado como rótulo no commit base.
- Discard: PRD NFR-5 usa refs de snapshot; implementado em diretório de backup.
- Matriz Git de CI: PRD pede 2.20, 2.34 e latest; CI tem system (22.04/24.04) + 2.20.5.

