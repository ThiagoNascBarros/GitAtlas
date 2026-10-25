# Roadmap de consolidação

## 1. Resumo executivo

- 71 achados dos 6 agentes: 0 CRÍTICA, 9 ALTA, 41 MÉDIA, 21 BAIXA; 5 são duplicatas de outro achado (66 itens únicos).
- O núcleo (Rust) é sólido: 120 testes, clippy limpo, sem achado crítico. O risco está nas bordas: UI sem rede de segurança, confirmações, empacotamento nunca validado.
- Maior lacuna de produto: sem repositórios recentes/restauração de sessão, sem zoom, sem verificação de segredos no Shared Stash.
- Maior lacuna de entrega: Flatpak nunca construído, sem atualização, CI não constrói pacotes nem mede benchmark.
- Tudo foi análise estática; nenhum agente rodou a UI. Medições de fps, memória e leitor de tela continuam pendentes.
- Matriz: 30 Implementado, 18 Parcial, 13 Ausente, 5 Não verificado (após correções do agente 2).

## 2. Lista priorizada

Nota: títulos mantidos como nos arquivos de origem; versões em português em `*.pt.md`.

ID | área | severidade | esforço | título | duplicata de
---|---|---|---|---|---
FUNC-6 | Funcionalidades | ALTA | P | Hard reset runs on one click with no confirmation | 
FUNC-7 | Funcionalidades | ALTA | P | Journal entries can panic the undo path (unchecked args[n]) | 
MOD-1 | Modais | ALTA | P | No ErrorBoundary: a failed resource can blank the whole app | 
MOD-2 | Modais | ALTA | P | Abort merge/rebase/cherry-pick has no confirmation and cannot be undone | 
LAY-1 | Layout | ALTA | P | Pane size limits are static and stored sizes are not clamped | 
PKG-1 | Empacotamento | ALTA | P | Flatpak manifest has never been built or run | 
FUNC-1 | Funcionalidades | ALTA | M | No recent repositories and no session restore | 
FUNC-4 | Funcionalidades | ALTA | M | Shared Stash publishes without secret check or hard size limit | 
LAY-3 | Layout | ALTA | G | No UI zoom (80-200 %) and no system font scaling | 
FUNC-11 | Funcionalidades | MÉDIA | P | Askpass socket: permission race, no peer check, silent fallback | 
FUNC-12 | Funcionalidades | MÉDIA | P | is_secret heuristic masks usernames, shows some secrets in clear | 
FUNC-13 | Funcionalidades | MÉDIA | P | worktree_diff accepts any path and reads untracked files unbounded | 
FUNC-14 | Funcionalidades | MÉDIA | P | Silent degradation: engine, watcher and git executable | 
UIV-1 | UI visual | MÉDIA | P | Theme choice is not persisted | 
UIV-2 | UI visual | MÉDIA | P | Shipped palette does not match design-tokens.md | 
UIV-4 | UI visual | MÉDIA | P | Contrast gaps not covered by contrast.mjs | 
UIV-5 | UI visual | MÉDIA | P | Input borders use border.default (1.3:1) | 
UIV-7 | UI visual | MÉDIA | P | Raw error text shown in graph; hard-coded English in perf overlay | 
MOD-3 | Modais | MÉDIA | P | Hard reset runs immediately from the commit menu | FUNC-6
MOD-4 | Modais | MÉDIA | P | Branch/force/tag delete have no confirmation | 
MOD-6 | Modais | MÉDIA | P | Force-push confirmation state survives dismissing the banner | 
MOD-7 | Modais | MÉDIA | P | Escape closes every open overlay at once | 
LAY-2 | Layout | MÉDIA | P | Responsive breakpoints below minimum window size are dead code | 
LAY-4 | Layout | MÉDIA | P | Graph canvas does not redraw on devicePixelRatio change | 
PKG-4 | Empacotamento | MÉDIA | P | Package/desktop-file naming does not match the app id | 
PKG-5 | Empacotamento | MÉDIA | P | CI never builds packages on pull requests | 
PKG-8 | Empacotamento | MÉDIA | P | Release workflow has no integrity or consistency checks | 
PERF-1 | Desempenho | MÉDIA | M | Blocking git calls run inline inside async Tauri commands | 
PERF-3 | Desempenho | MÉDIA | M | File watcher: trailing-only debounce, rebuilt on every refresh, .git recursed | 
PERF-4 | Desempenho | MÉDIA | M | git status unbounded and rendered without virtualisation | 
PERF-5 | Desempenho | MÉDIA | M | Diff view: no size limit, no virtualisation, parse not memoised | 
PERF-7 | Desempenho | MÉDIA | M | Benchmark coverage is narrow; UI perf not measured or gated | 
FUNC-2 | Funcionalidades | MÉDIA | M | Diff: no whitespace toggle, whole-file view, syntax highlighting | 
FUNC-3 | Funcionalidades | MÉDIA | M | Zoom 80-200 % absent | LAY-3
FUNC-8 | Funcionalidades | MÉDIA | M | Journal durability and silent loss of corrupt lines | 
FUNC-9 | Funcionalidades | MÉDIA | M | Conflicted merge/cherry-pick/revert is never journaled | 
FUNC-10 | Funcionalidades | MÉDIA | M | Blocking git work on the async runtime; no timeout/cancel for local ops | PERF-1
UIV-3 | UI visual | MÉDIA | M | No high-contrast themes or forced-colors support | 
UIV-8 | UI visual | MÉDIA | M | Deferred keyboard gaps: graph column resize, WIP list | 
UIV-9 | UI visual | MÉDIA | M | Accessibility verified only in Chromium; target is WebKitGTK | 
MOD-8 | Modais | MÉDIA | M | Click-again confirmations: no announcement, no delay, weak wording | 
LAY-5 | Layout | MÉDIA | M | Splitters: no collapse, no reset, no pointer cancel | 
LAY-6 | Layout | MÉDIA | M | Graph column resize is pointer-only | UIV-8
PKG-2 | Empacotamento | MÉDIA | M | --filesystem=home exposes ~/.ssh and ~/.gnupg | 
PKG-3 | Empacotamento | MÉDIA | M | Flatpak credential/ssh/gpg paths rest on untested assumptions | 
PKG-6 | Empacotamento | MÉDIA | M | CI Git matrix implicit for 2.34/latest; no benchmark job | 
PKG-7 | Empacotamento | MÉDIA | M | No update strategy | 
PERF-2 | Desempenho | MÉDIA | G | Every refresh is a full reload; each commit invalidates the on-disk cache | 
FUNC-5 | Funcionalidades | MÉDIA | G | N3-N7 and search/blame not implemented | 
FUNC-16 | Funcionalidades | MÉDIA | G | Test coverage gaps in Tauri commands, UI and failure paths | 
PERF-8 | Desempenho | BAIXA | P | Graph canvas redraw cost unverified; per-chunk git log process | 
FUNC-15 | Funcionalidades | BAIXA | P | Shared-state Mutex unwrap can cascade after one panic | 
FUNC-17 | Funcionalidades | BAIXA | P | Author-only Shared Stash rules are advisory | 
UIV-6 | UI visual | BAIXA | P | Fonts differ from tokens and none is bundled | 
UIV-11 | UI visual | BAIXA | P | pt-BR labels up to 3.5x longer than English | 
MOD-5 | Modais | BAIXA | P | Non-interactive rebase starts directly from ref menu | 
MOD-10 | Modais | BAIXA | P | English text leaks in pt-BR for uncatalogued errors | 
MOD-11 | Modais | BAIXA | P | No re-entrancy guard on run(), undo/redo, stash buttons | 
MOD-12 | Modais | BAIXA | P | Toast single-slot, auto-hides, no pause on hover/focus | 
MOD-13 | Modais | BAIXA | P | Journal dialog shows empty while loading; no error state | 
MOD-14 | Modais | BAIXA | P | Credentials prompt shows no repo/remote context | 
LAY-7 | Layout | BAIXA | P | Diff splitter max read non-reactively, differs from CSS cap | 
LAY-8 | Layout | BAIXA | P | Canvas font and bar height disagree with layout direction | 
PKG-9 | Empacotamento | BAIXA | P | AppStream metainfo incomplete for store submission | 
PKG-10 | Empacotamento | BAIXA | P | Dev shortcut script diverges from packaged desktop entry | 
PKG-11 | Empacotamento | BAIXA | P | Stale target/ build failure not documented | 
PKG-12 | Empacotamento | BAIXA | P | Licensing headers mostly consistent, small gaps | 
PKG-13 | Empacotamento | BAIXA | P | Release profile minimal and unmeasured | 
PERF-6 | Desempenho | BAIXA | M | On-disk cache: whole-file JSON rewrite, no eviction | 
UIV-10 | UI visual | BAIXA | M | Fixed px sizing: no font-scale support | LAY-3
MOD-9 | Modais | BAIXA | M | Dialogs are <dialog open> without showModal() | 

Duplicatas: MOD-3 = FUNC-6 (reset hard; MOD-3 considera MÉDIA por ser desfazível, mantida a severidade ALTA de FUNC-6); FUNC-3 e UIV-10 = LAY-3 (zoom/escala de fonte); LAY-6 = UIV-8 (resize de colunas só por ponteiro); FUNC-10 = PERF-1 (git bloqueante no runtime async).

## 3. Ganhos rápidos (ALTA/CRÍTICA, esforço P)

- FUNC-6: Hard reset runs on one click with no confirmation
- FUNC-7: Journal entries can panic the undo path (unchecked args[n])
- MOD-1: No ErrorBoundary: a failed resource can blank the whole app
- MOD-2: Abort merge/rebase/cherry-pick has no confirmation and cannot be undone
- LAY-1: Pane size limits are static and stored sizes are not clamped
- PKG-1: Flatpak manifest has never been built or run

## 4. Marcos sugeridos

### M1 — Estabilidade e segurança de dados
- Itens: FUNC-7, MOD-1, MOD-2, FUNC-6 (inclui MOD-3), MOD-4, MOD-6, FUNC-13, FUNC-14, FUNC-8, FUNC-9, FUNC-11, FUNC-12
- Dependências: Nenhuma. Faça FUNC-7 antes de FUNC-8/9 (mesmo código do journal).

### M2 — Desempenho e medição
- Itens: PERF-1 (inclui FUNC-10), PERF-3, PERF-4, PERF-5, PERF-2, PERF-7, PERF-6, PERF-8
- Dependências: PERF-7 (medir primeiro) deve vir antes de PERF-2/4/5.

### M3 — Funcionalidades da doc
- Itens: FUNC-1, FUNC-4, FUNC-2, LAY-3 (inclui FUNC-3, UIV-10), FUNC-5, FUNC-16 (testes)
- Dependências: LAY-3 depende de LAY-1 e UIV-10/estilos em px; FUNC-16 deve acompanhar os demais.

### M4 — UI, acessibilidade, layout
- Itens: LAY-1, LAY-2, LAY-4, LAY-5, UIV-1..UIV-5, UIV-7, UIV-8 (inclui LAY-6), UIV-9, MOD-7, MOD-8, UIV-3, restante BAIXA
- Dependências: UIV-9 exige rodar no WebKitGTK; UIV-2 exige decisão do dono (paleta violeta vs âmbar do REA).

### M5 — Empacotamento e release
- Itens: PKG-1, PKG-5, PKG-4, PKG-2, PKG-3, PKG-6, PKG-8, PKG-7, PKG-9..PKG-13
- Dependências: PKG-1 e PKG-5 antes de PKG-2/3 (precisam de um build real); PKG-6 depende de PERF-7.

## 5. Definição de "consolidado"

- [ ] Nenhum achado CRÍTICA ou ALTA aberto
- [ ] Operações destrutivas e abortos com confirmação ou undo (FUNC-6, MOD-2, MOD-4)
- [ ] Benchmark de UI (primeiro paint, fps, RSS) medido e com gate no CI (PERF-7, PKG-6)
- [ ] Repositórios recentes e restauração de sessão (FUNC-1); tema persistido (UIV-1); zoom (LAY-3)
- [ ] Shared Stash com checagem de segredos e limite de tamanho (FUNC-4)
- [ ] Contraste e teclado verificados no WebKitGTK/Orca (UIV-4, UIV-5, UIV-8, UIV-9)
- [ ] Flatpak, AppImage e .deb construídos no CI e testados em Ubuntu 22.04/24.04, Wayland e X11 (PKG-1, PKG-5; AC-012)
- [ ] Estratégia de atualização definida (PKG-7)
- [ ] Cada linha Ausente/Parcial da matriz implementada ou explicitamente adiada pelo dono
- [ ] Teste manual de docs/release/checklist.md §6 concluído (critérios 4, 7, 14)

## 6. Branches sugeridas (a partir de `develop`)

- `fix/journal-undo-panic` (FUNC-7, depois FUNC-8)
- `fix/error-boundary` (MOD-1)
- `fix/destructive-confirms` (FUNC-6, MOD-2, MOD-3, MOD-4, MOD-6)
- `fix/pane-size-clamp` (LAY-1)
- `fix/theme-persistence` (UIV-1)
- `fix/worktree-diff-path-check` (FUNC-13)
- `feature/recent-repos-session` (FUNC-1)
- `feature/shared-stash-secrets-check` (FUNC-4)
- `feature/ui-zoom` (LAY-3)
- `feature/ui-bench-ci` (PERF-7, PKG-6)
- `fix/flatpak-first-build` (PKG-1, PKG-5)
