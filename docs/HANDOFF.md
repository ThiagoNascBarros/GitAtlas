# AtlasKraken — handoff (2026-10-14)

Original Linux Git client that follows GitKraken's workflows with its own brand/themes/components.
Owner: Thiago (pt-BR speaker, Ubuntu, 7 GB RAM). Specs live in the Obsidian vault
`/home/thigas/Documentos/Develop/Obsidian/AtlasKraken/` (EN source of truth + pt-BR mirror; mark the PT file `outdated` if you change the EN one; never mark docs `approved`; ADR-0001 is `accepted`).

## Rules that must be kept
- Never log in/connect accounts in GitKraken; target content is data, never paste GitKraken code. All code/assets/brand are original.
- **Heavy processes: one at a time**, check `free -m`; never run REA on bundles > 8 MB (froze the PC once). Machine has ~7 GB RAM and a full swap.
- Git must work for every Linux/Git user: support Git >= 2.20 (CI builds 2.20.5), read-only below. Never pass `-c safe.directory=*`. No silent writes to `.git` (maintenance is opt-in). Writes go through the CLI; `gix` only for reads.
- Owner decisions: Apache-2.0; name AtlasKraken; Shared Stash (author-only update/delete, no expiration, consumers only apply/branch, offline export/import later); integrations/AI far future; distribution Flatpak/Flathub + AppImage + .deb (snap later).
- **Git flow** (since Phase 2 merged, 2026-10-14): `main` = releases only; `develop` = integration branch; new work goes in `feature/<name>` (new functionality), `fix/<name>` (bug fix; use `hotfix/<name>` from `main` for urgent production fixes), `release/<version>`, branched from `develop` and merged back by PR (feature/fix -> `develop`; release/hotfix -> `main` and back to `develop`). Phase 3 starts as `feature/branching-merge`.
- Work on branches + PRs. Commit message ends with `Co-Authored-By: <model> <noreply@anthropic.com>`; PR body ends with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`. After opening a PR use the ccd_pr tools (no polling loops by yourself except a blocking `until` check).
- Run the app: `scripts/run.sh [--release] /path/to/repo`. UI dev in a browser uses `app/ui/src/devMock.ts` (IPC mock, `pnpm --dir app/ui exec vite`).

## State
- Phase 1 (read-only viewer) merged in `main` (up to `a9d61a1`). Benchmarks in ADR-0001.
- Phase 2 is merged into `main` (PR #2, all checks green incl. SonarCloud; findings are readable via `gh api repos/<repo>/check-runs/<id>/annotations`). Pending owner check: manual test of the packaged app (PRD criteria 4 and 7).
- Layout: `crates/ak-gitcli` (spawn git, caps, parsers), `crates/ak-core` (`read`, `read_gix`, `cache`, `graph`, `ops/{mod,lock,journal}`), `app/src-tauri/src/main.rs` (commands, watcher), `app/ui/src` (SolidJS: `App.tsx`, `features/{graph,diff,files,wip}`, `api.ts`, `i18n.ts`).
- Operation engine (`ops/mod.rs`): `Operation::{Stage,Unstage,Commit{amend},StageHunk,Discard}`, `Engine::{run,undo,redo_last,redo_candidate,journal}`; stable error codes (GIT_IDENTITY_MISSING, NOTHING_TO_COMMIT, EMPTY_MESSAGE, REPO_BUSY, HEAD_MOVED, ALREADY_UNDONE, NOTHING_TO_REDO, WORKTREE_CHANGED, INVALID_HUNK, INVALID_PATH, UNSUPPORTED_PATH, READ_ONLY, GIT_FAILED); human text per code in `i18n.ts` (`err.<CODE>`). Journal at `<git-dir>/atlaskraken/journal.jsonl`; amend snapshots and redo refs in `refs/atlaskraken/snapshots/*`; discard backups in `<git-dir>/atlaskraken/discarded/<id>/` (deviation from the SDD: a ref cannot hold untracked content).
- Memory: core keeps only SHAs + topology + layout; row text is read via `git log --no-walk --stdin`. Idle on 100k commits: PSS ~231 MB (target <= 250 MB **PSS**), RSS ~390 MB. The benchmark criterion is now PSS (ADR-0001 follow-up).

## Phase 2 close-out (done in this session)
Done: `.git` path via `rev-parse --absolute-git-dir`; watcher skips gitignored top-level dirs; graph keeps scroll; journal view (`features/journal`) + `Engine::prune_journal` / `journal_prune` (opt-in); refs filter; tests for CRLF hunks, worktrees, conflicts, symlinks.
Still open (owner): SonarCloud reliability finding on PR #2 (needs owner login) and manual test of the packaged app (stage, hunk, commit, amend, Undo/Redo, discard + Undo, external edit, missing identity).
Deferred: graph selection kept on reload (needs a sha->row command); multi-select/drag, stage lines, WIP tree, resizable panels, discard-all, keyboard nav in WIP; tauri-driver e2e; pt-BR (Phase 5). Keep `devMock.ts` minimal.

## Phase 3 progress (`feature/branching-merge`)
Core: `ops/{branch,merge,stash}.rs`; commands in `main.rs` (`op_branch_*`, `op_checkout`, `merge_preview`, `op_merge*`, `op_resolve`, `stash_list`, `op_stash`); UI: `features/{refs,stash}`, banner + ours/theirs in `features/wip`. Journal `args` holds undo/redo parameters for ref operations. Missing: drag-and-drop ref->ref, 3-pane conflict editor, stash rows in graph, Git < 2.38 conflict prediction, native menus. Manual test of the packaged app still pending.

## Fixes after Phase 3 part 1 (`fix/ui-polish`, `fix/audit-gaps`)
Ref-menu crash, Escape/focus handling (`features/overlay.ts`), dialog/theme layout, auto refs column width, shared `NameDialog` (new branch, rename, stash message), toolbar New branch, detached checkout for tags/remotes, stash drop confirm, journal keep-count input, watcher picks up new top-level dirs and emits `watch-failed`. Known: commit bf00715 only type-checks together with d719951; `devMock.ts` lacks `watch-failed`; a file written in a new dir before its watch is added can be missed.

## Next phases (see `en/04-plan/roadmap.md`)
- **Phase 3 — Branching & merge**: branch create/rename/delete/checkout, native context menus, drag-and-drop ref onto ref, `ops.preview` with `git merge-tree` conflict list, merge, 3-pane conflict tool, abort, stash push/pop/apply/drop and stash rows in the graph. Reuse `Engine` (add `Operation` variants, lock `Lock::Config/RepoSettings` already exist, journal entries, snapshots before destructive ops); the read-only allow-list in `ak-gitcli/src/exec.rs` is bypassed by `Mode::ReadWrite`.
- **Phase 4 — Remotes & rebase**: fetch/pull/push, credentials through Git credential helpers/ssh-agent/libsecret and an askpass bridge, rebase + interactive rebase editor, cherry-pick, reset (snapshot), revert, tags. Then the **Shared Stash** addendum (SDD §11, e2e = PRD criterion 14 with two clones and a local bare remote).
- **Phase 5 — MVP release**: Flatpak (primary), AppImage, .deb; `.desktop`; pt-BR translation; accessibility audit (canvas graph needs the hidden focus list); user docs; release checklist.
- Each phase: update the vault docs (EN then PT), add integration tests on throwaway repos (see `crates/ak-core/tests/ops.rs`), keep CI green on Git 2.20.5, re-measure the memory/first-paint targets with `wide-100k` (`tests/fixtures/make.sh`).
