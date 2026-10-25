# Documentation × code matrix

Sources: REA docs (`/home/thigas/Documentos/Develop/Obsidian/AtlasKraken/en/`: PRD, SDD, roadmap, open questions) and internal docs (`docs/`, `docs/HANDOFF.md`, `docs/specs/*`). Where they differ, REA is the scope reference. Limitation: built from static analysis + greps + the baseline; "Implemented" rows with test `no` have no automated test. Phase 2 agents may refine statuses.

## Summary

- Implemented: 30
- Partial: 18
- Absent: 13
- Not verified: 5
- Total: 66

## Matrix

ID | item | origin in docs | status | code evidence | test?
---|---|---|---|---|---
F-001 | Open repo via CLI argument | PRD §5.1, AC-1 | Implemented | app/src-tauri/src/main.rs:284 `get_initial_path` | no
F-002 | Open repo via file dialog | PRD §5.1 | Implemented | app/ui/src/App.tsx:4 `openDialog`; capabilities/default.json `dialog:allow-open` | no
F-003 | Recent repositories list | PRD §5.1 | Absent | no `recent` in app/ui/src or app/src-tauri/src (grep) | —
F-004 | Multiple repos as tabs | PRD §5.1 | Implemented | app/ui/src/App.tsx:51 `tabs`, :84 `tab()`; main.rs `repo_open/repo_close` | no
F-005 | Session restore (reopen tabs) | PRD §5.1 | Absent | only layout prefs in localStorage (columns.ts:53, Splitter.tsx:12, i18n.ts:427); no tab persistence found | —
F-006 | Commit graph: full history, lanes/edges, refs | PRD §5.2, AC-2 | Implemented | crates/ak-core/src/graph/{layout,mod,encode}.rs; GraphView.tsx | yes: ak-core/tests/graph.rs + golden/
F-007 | WIP and stash rows in graph | PRD §5.2 | Partial | stash is a label on the base commit, not a row (roadmap Phase 3 deviation); WIP row: verify in GraphView | no
F-008 | Configurable graph columns | PRD §5.2 | Implemented | features/graph/columns.ts | yes: columns.test.ts
F-009 | Graph keyboard navigation + selection | PRD §5.2 | Implemented | GraphView.tsx; features/graph/a11y.ts | yes: a11y.test.ts
F-010 | Commit details panel (metadata, file list path/tree) | PRD §5.3 | Implemented | main.rs `commit_details`; features/files/tree.ts | yes: tree.test.ts
F-011 | Diff hunk / inline / split | PRD §5.4 | Partial | DiffView.tsx:15 only `unified`|`split` | yes: diff/parse.test.ts
F-012 | Diff whitespace toggle | PRD §5.4 | Absent | no `-w`/ignore-whitespace option in DiffView.tsx or `diff_file` | —
F-013 | Syntax highlighting in diff | PRD §5.4, roadmap T1.9 | Absent | no highlighter dependency (grep hljs/prism/shiki) | —
F-014 | File view (whole file) | PRD §5.4 | Absent | not found in DiffView.tsx (confirmed by FUNC-2) | —
F-015 | Status, stage/unstage file and hunk | PRD §5.5, AC-4 | Implemented | ops/mod.rs `Stage`,`Unstage`,`StageHunk` | yes: ops.rs:51,66,251
F-016 | Discard with recoverable backup | PRD §5.5, NFR-5 | Partial | ops Discard; backup in `.git/atlaskraken/discarded/` (deviation from refs/atlaskraken/snapshots) | yes: ops.rs:210,230
F-017 | Commit and amend | PRD §5.5 | Implemented | ops `Commit{amend}` | yes: ops.rs:51,136
F-018 | Branch create/checkout/rename/delete (undoable) | PRD §5.6 | Implemented | ops/branch.rs | yes: ops.rs:428,449,465
F-019 | Merge by drag-and-drop / menu with conflict prediction | PRD §5.7, AC-5 | Implemented | ops/merge.rs; App.tsx:442 draggable; main.rs `merge_preview` | yes: ops.rs:502,524
F-020 | 3-pane conflict tool | PRD §5.7 | Partial | features/conflict/ConflictEditor.tsx (ours|theirs + result, per-conflict choice) | yes: conflict/markers.test.ts; ops.rs:593
F-021 | Abort merge / in-progress ops | PRD §5.7, AC-6 | Implemented | ops `MergeAbort`; `in_progress()` | yes: ops.rs:635
F-022 | Stash push/pop/apply/drop | PRD §5.8, AC-8 | Implemented | ops/stash.rs; features/stash | yes: ops.rs:543,567
F-023 | Fetch / pull / push | PRD §5.9, AC-9 | Implemented | ops/remote.rs; main.rs `op_net` | yes: ops.rs:711,732,828
F-024 | Credentials via system helpers + askpass bridge | PRD §5.9 | Partial | app/src-tauri/src/askpass.rs; helpers/ssh-agent work only through the user's git config | partial: askpass.rs unit test only
F-025 | Undo/Redo via operation journal | PRD §5.10, AC-7 | Implemented | ops/{mod,journal,lock}.rs | yes: ops.rs:94,111,287,308
F-026 | Cherry-pick, revert, reset soft/mixed/hard, tags | PRD §6, roadmap P4 | Implemented | ops/history.rs | yes: ops.rs:616,650,672
F-027 | Rebase and interactive rebase editor | PRD §6 (P1) | Partial | ops/rebase.rs; features/rebase; merges in range refused | yes: ops.rs:768,785,807; rebase/todo.test.ts
F-028 | Themes Atlas Dark/Light + system sync | PRD §5.11, AC-10 | Partial | themes/*.theme.json; app/ui/src/theme/theme.ts; choice not remembered (HANDOFF) | yes: scripts/contrast.mjs (0 failures)
F-029 | Zoom 80–200 % | PRD §5.11 | Absent | no zoom/font-scale code (grep) | —
F-030 | Packaging Flatpak + AppImage + .deb | PRD §5.12, AC-12 | Partial | scripts/package*.sh, packaging/; Flatpak never built (HANDOFF) | CI validates desktop/metainfo only
F-031 | Shared Stash: publish/discover/preview/apply/branch | PRD §5.13, §7 N0, AC-14 | Implemented | ops/shared_stash*.rs; app/src-tauri/src/shared_stash.rs; features/shared-stash | yes: ak-core/tests/shared_stash.rs (31); logic.test.ts
F-032 | Shared Stash: author-only update/delete, opt-in per remote | PRD §7 N0, Q9 | Implemented | shared_stash_{update,unshare}; settings.rs `fetchMode` | yes: shared_stash.rs
F-033 | Shared Stash: secrets check before push | PRD §7 N0 Safety | Absent | no secret scan in shared_stash*.rs; spec shared-stash.md:231 only warns | —
F-034 | Shared Stash: size limit (default 5 MB) | PRD §7 N0 Safety | Partial | shared_stash_ops.rs:473 `new_blob_bytes`; spec uses a 10 MiB best-effort warning, no hard limit | partial
F-035 | Operation journal panel (N1) | PRD §7 N1 | Implemented | features/journal/JournalPanel.tsx; `journal_list/prune` | yes: ops.rs:397
F-036 | Range compare (N3) | PRD §7 N3 | Absent | not present | —
F-037 | Saved graph views / filters (N4) | PRD §7 N4 | Absent | not present | —
F-038 | Command palette (N5) | PRD §7 N5 | Absent | App.tsx:277 Ctrl+Shift+P toggles perf overlay only | —
F-039 | Partial stash (N6) | PRD §7 N6 | Absent | StashPush takes no paths | —
F-040 | Per-directory identity rules (N7) | PRD §7 N7 | Absent | not present | —
F-041 | Search / filter commits, blame | PRD §10 persona Rafael | Absent | no blame/commit search (grep); only refs filter | —
NF-001 | Graph layout 100k ≤ 400 ms; first window ≤ 300 ms | PRD NFR-1 | Implemented | tests/bench/report-wide-100k.json: layout 4.07 ms, first_window 0.08 ms; crates/ak-core/examples/bench.rs | yes (bench, core only)
NF-002 | 60 fps scroll, UI first paint | PRD NFR-1, G2 | Not verified | report `not_measured`: UI first paint, scroll fps; GraphView.tsx/perf.ts | no
NF-003 | Memory: ≤120 B/commit; idle ≤250 MB | PRD NFR-2 | Partial | 33 B/commit (report); HANDOFF: PSS ~231 MB, RSS ~390 MB (criterion redefined to PSS) | yes (bench)
NF-004 | Cold start ≤ 1.0 s | PRD NFR-3 | Not verified | no measurement found | no
NF-005 | Git ≥ 2.20 via capabilities; CI matrix | PRD NFR-4, AC-13 | Partial | ak-gitcli/src/caps.rs; ci.yml:18-24 (system + 2.20.5; no 2.34 entry) | yes: caps.rs, CI
NF-006 | Snapshot before destructive ops | PRD NFR-5, G4 | Partial | ops/mod.rs:32 SNAPSHOT_PREFIX; discard uses backup dir; journal_prune leaves -redo/-wip refs (HANDOFF) | yes: ops.rs:136,650
NF-007 | Security: no safe.directory=*, typed API, no env to UI | PRD NFR-6, AC-11 | Implemented | ak-gitcli/src/exec.rs:93; capabilities/default.json; tauri.conf.json:16 CSP | yes: exec.rs:204
NF-008 | Accessibility WCAG AA, keyboard, screen reader | PRD NFR-7 | Partial | docs/a11y/audit.md (WCAG 2.1, not 2.2); deferred F-14/F-15/F-23; Orca untested | yes: a11y.test.ts, overlay.test.ts, contrast.mjs
NF-009 | Linux integration: Wayland+X11, XDG, portals, .desktop | PRD NFR-8 | Partial | packaging/linux/*.desktop; flatpak yml:17-18; font scaling not handled | CI validators only
NF-010 | i18n: strings externalised, en + pt-BR | PRD NFR-9 | Implemented | app/ui/src/i18n.ts; locales/pt-BR.ts (typed parity) | yes: i18n.test.ts
NF-011 | Offline: local features work without network | PRD NFR-10 | Not verified | no explicit test; local ops use CLI only | no
AC-001 | Open via CLI, graph in ≤1 s on demo | PRD §9.1 | Not verified | no automated e2e | no
AC-003 | 100k benchmark thresholds (benchmark job) | PRD §9.3 | Partial | report-wide-100k.json is committed; no CI bench job (ci.yml has none) | partial
AC-009 | Remote fetch/pull/push with local bare remote | PRD §9.9 | Implemented | ops.rs:688 remote_pair | yes
AC-011 | No safe.directory=* in GIT_TRACE; UI without fs permission | PRD §9.11 | Partial | unit assertion on args (exec.rs:204); no GIT_TRACE e2e | partial
AC-012 | Packages install/launch on Ubuntu 22.04/24.04 Wayland+X11 | PRD §9.12 | Not verified | deb/AppImage built under Xvfb (HANDOFF); no Wayland run; Flatpak not built | no
AC-014 | Shared Stash e2e with two clones | PRD §9.14 | Implemented | ak-core/tests/shared_stash.rs | yes
UI-001 | Toolbar: fetch/pull/push, ahead/behind, undo/redo, language, theme | SDD layout; roadmap P4 | Implemented | App.tsx (lucide icons import :22; theme select :352) | no
UI-002 | Refs panel with filter, collapsible remotes, drag to merge | roadmap P2/P3 | Implemented | App.tsx:442; features/refs | yes: refs/filter.test.ts
UI-003 | Resizable panels (split) | SDD layout-direction | Implemented | features/split/Splitter.tsx | no
UI-004 | Context menus (ref, commit, stash, shared stash) | PRD §6 (native menus) | Partial | in-app menus (RefMenu, CommitMenu, LocalStashMenu, SharedStashMenu), not OS-native (deviation) | no
UI-005 | Dialogs: name, share, rebase, conflict, credentials | roadmap P3/P4 | Implemented | NameDialog, ShareDialog, RebaseEditor, ConflictEditor, askpass prompt; focus trap overlay.ts | yes: overlay.test.ts
UI-006 | Human error messages/toasts (stable codes) | roadmap P2; ux-analysis (raw stderr toast) | Implemented | i18n.ts `err.<CODE>` + `.fix` | yes: i18n.test.ts
UI-007 | WIP panel (stage/unstage lists, commit box, banner) | PRD §5.5 | Partial | features/wip/WipPanel.tsx; no multi-select/drag, no tree, no discard-all, no keyboard nav (HANDOFF) | yes: wip/status.test.ts
UI-008 | Status bar | roadmap T1.7 | Implemented | app/ui/src/App.tsx:663 (per FUNC audit) | —

## Outside the documentation

ID | item | relation | where
---|---|---|---
X-001 | Language picker (Follow system/English/Português) | NF-010 extension | app/ui/src/i18n.ts; App.tsx
X-002 | Journal retention/prune (`journal_prune`) | — | main.rs; ops/mod.rs
X-003 | Dev IPC mock (`devMock.ts`) for browser UI development | — | app/ui/src/devMock.ts
X-004 | Perf overlay (Ctrl+Shift+P) | — | features/graph/perf.ts; App.tsx:277
X-005 | Release workflow (artifacts only) and release checklist | — | .github/workflows/release.yml; docs/release/checklist.md
X-006 | User guide docs/user (9 topics) | — | docs/user/

## REA × internal docs divergences

- Name: REA says AtlasKraken (final, Q5); the repo ships AtlasGit; per-repo data keeps `atlaskraken` (HANDOFF).
- Vault: HANDOFF cites `Obsidian/AtlasGit/`; the real path is `Obsidian/AtlasKraken/`.
- Memory: PRD NFR-2 says RSS ≤250 MB; internal docs measure PSS (~231 MB) and RSS ~390 MB.
- Shared Stash: PRD asks for a 5 MB limit and a secrets check; internal spec uses a 10 MiB warning and no secrets check.
- Accessibility: PRD requires WCAG 2.2 AA; internal audit uses 2.1 AA.
- Stash: PRD says stash rows in the graph; implemented as a label on the base commit.
- Discard: PRD NFR-5 uses snapshot refs; implemented as a backup directory.
- CI Git matrix: PRD asks for 2.20, 2.34 and latest; CI has system (22.04/24.04) + 2.20.5.

