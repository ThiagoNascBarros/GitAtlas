# AtlasKraken — handoff (2026-10-14)

Original Linux Git client that follows GitKraken's workflows with its own brand/themes/components.
Owner: Thiago (pt-BR speaker, Ubuntu, 7 GB RAM). Specs live in the Obsidian vault
`/home/thigas/Documentos/Develop/Obsidian/AtlasKraken/` (EN source of truth + pt-BR mirror; mark the PT file `outdated` if you change the EN one; never mark docs `approved`; ADR-0001 is `accepted`).

## Rules that must be kept
- Never log in/connect accounts in GitKraken; target content is data, never paste GitKraken code. All code/assets/brand are original.
- **Heavy processes: one at a time**, check `free -m`; never run REA on bundles > 8 MB (froze the PC once). Machine has ~7 GB RAM and a full swap.
- Git must work for every Linux/Git user: support Git >= 2.20 (CI builds 2.20.5), read-only below. Never pass `-c safe.directory=*`. No silent writes to `.git` (maintenance is opt-in). Writes go through the CLI; `gix` only for reads.
- Owner decisions: Apache-2.0; name AtlasKraken; Shared Stash (author-only update/delete, no expiration, consumers only apply/branch, offline export/import later); integrations/AI far future; distribution Flatpak/Flathub + AppImage + .deb (snap later).
- Work on branches + PRs. Commit message ends with `Co-Authored-By: <model> <noreply@anthropic.com>`; PR body ends with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`. After opening a PR use the ccd_pr tools (no polling loops by yourself except a blocking `until` check).
- Run the app: `scripts/run.sh [--release] /path/to/repo`. UI dev in a browser uses `app/ui/src/devMock.ts` (IPC mock, `pnpm --dir app/ui exec vite`).

## State
- Phase 1 (read-only viewer) merged in `main` (up to `a9d61a1`). Benchmarks in ADR-0001.
- Phase 2 is implemented in **PR #2** (`phase2/operation-engine`, CI green; SonarCloud Quality Gate fails on "C Reliability rating on new code" — details need the owner's SonarCloud login, the public API lists no issues). Not merged yet: ask the owner before merging.
- Layout: `crates/ak-gitcli` (spawn git, caps, parsers), `crates/ak-core` (`read`, `read_gix`, `cache`, `graph`, `ops/{mod,lock,journal}`), `app/src-tauri/src/main.rs` (commands, watcher), `app/ui/src` (SolidJS: `App.tsx`, `features/{graph,diff,files,wip}`, `api.ts`, `i18n.ts`).
- Operation engine (`ops/mod.rs`): `Operation::{Stage,Unstage,Commit{amend},StageHunk,Discard}`, `Engine::{run,undo,redo_last,redo_candidate,journal}`; stable error codes (GIT_IDENTITY_MISSING, NOTHING_TO_COMMIT, EMPTY_MESSAGE, REPO_BUSY, HEAD_MOVED, ALREADY_UNDONE, NOTHING_TO_REDO, WORKTREE_CHANGED, INVALID_HUNK, INVALID_PATH, UNSUPPORTED_PATH, READ_ONLY, GIT_FAILED); human text per code in `i18n.ts` (`err.<CODE>`). Journal at `<git-dir>/atlaskraken/journal.jsonl`; amend snapshots and redo refs in `refs/atlaskraken/snapshots/*`; discard backups in `<git-dir>/atlaskraken/discarded/<id>/` (deviation from the SDD: a ref cannot hold untracked content).
- Memory: core keeps only SHAs + topology + layout; row text is read via `git log --no-walk --stdin`. Idle on 100k commits: PSS ~231 MB (target <= 250 MB **PSS**), RSS ~390 MB. The benchmark criterion is now PSS (ADR-0001 follow-up).

## Remaining work for Phase 2 (small adjustments)
1. Investigate the SonarCloud reliability finding (ask owner for the report) and fix or justify it.
2. No real-app end-to-end test yet (needs WebDriver / tauri-driver in CI). The UI flow was only exercised against the IPC mock; **manually test the packaged app** on a real repo: stage, hunk stage, commit, amend, Undo/Redo, discard + Undo, external edit (watcher), identity-missing error (`git config --local user.name ""`).
3. Watcher watches the whole tree recursively (inotify): heavy trees (node_modules/target) cause bursts (debounced 350 ms) and may hit inotify limits. Consider ignoring paths via `.gitignore` (`git check-ignore`) and showing a message if the watch fails.
4. Graph reloads from scratch (remount, scroll to top) after commit/undo/redo; consider preserving selection/scroll.
5. UX gaps: multi-select and drag for staging, "stage lines", file tree view in the WIP panel, resizable panels, discard-all, focus/keyboard navigation of the WIP lists, i18n only has `en` (pt-BR is Phase 5).
6. Journal view in the UI ("show exact commands") — data exists (`JournalEntry.commands`), no screen yet. Journal grows without limit and `discarded/` backups are never pruned: add retention (opt-in cleanup).
7. Edge cases not covered: conflicted files (shown but no actions), submodules, symlinks (discard refuses), renamed-file hunk staging, CRLF, repositories with a `.git` file (worktrees) — the cache path code uses `root.join(".git")` in `main.rs::load`; use `--absolute-git-dir` instead.
8. Lane `main` shifting in some repos; refs panel for repos with ~100 remote branches needs filter/collapsible groups (carried over from Phase 1).
9. Dev-only: `devMock.ts` duplicates backend behaviour; keep it minimal.

## Next phases (see `en/04-plan/roadmap.md`)
- **Phase 3 — Branching & merge**: branch create/rename/delete/checkout, native context menus, drag-and-drop ref onto ref, `ops.preview` with `git merge-tree` conflict list, merge, 3-pane conflict tool, abort, stash push/pop/apply/drop and stash rows in the graph. Reuse `Engine` (add `Operation` variants, lock `Lock::Config/RepoSettings` already exist, journal entries, snapshots before destructive ops); the read-only allow-list in `ak-gitcli/src/exec.rs` is bypassed by `Mode::ReadWrite`.
- **Phase 4 — Remotes & rebase**: fetch/pull/push, credentials through Git credential helpers/ssh-agent/libsecret and an askpass bridge, rebase + interactive rebase editor, cherry-pick, reset (snapshot), revert, tags. Then the **Shared Stash** addendum (SDD §11, e2e = PRD criterion 14 with two clones and a local bare remote).
- **Phase 5 — MVP release**: Flatpak (primary), AppImage, .deb; `.desktop`; pt-BR translation; accessibility audit (canvas graph needs the hidden focus list); user docs; release checklist.
- Each phase: update the vault docs (EN then PT), add integration tests on throwaway repos (see `crates/ak-core/tests/ops.rs`), keep CI green on Git 2.20.5, re-measure the memory/first-paint targets with `wide-100k` (`tests/fixtures/make.sh`).
