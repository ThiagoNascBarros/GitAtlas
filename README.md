<p align="center"><img src="app/src-tauri/icons/icon.png" width="128" alt="AtlasGit icon"></p>

# AtlasGit

Desktop Git client for Linux — Tauri 2 + Rust core + SolidJS. Licensed under Apache-2.0.

> Formerly **AtlasKraken**. Per-repository data still lives under `<git-dir>/atlaskraken/` and `refs/atlaskraken/` so journals, snapshots and backups made before the rename keep working.

## Status

Phases 1–4 are done (in `develop`); next is the Shared Stash addendum, then Phase 5 (MVP release).

| Phase | What it delivers |
|-------|------------------|
| 1 — Read-only viewer | Open a repo, full commit graph (canvas, lanes, virtualized rows, graph cache), commit details, file diffs, Atlas Dark/Light themes |
| 2 — Operation engine | Stage/unstage files and hunks, commit, amend, discard, Undo/Redo backed by a journal and snapshots, journal panel with opt-in cleanup, file watcher |
| 3 — Branching & merge | Create/rename/delete/checkout branches, detached checkout of tags/remotes, merge with conflict preview (`git merge-tree`), drag a ref onto the current branch to merge, 3-pane conflict editor, abort, stash push/pop/apply/drop with labels on the graph, refs filter |
| 4 — Remotes & rebase | Fetch/pull/push with credentials through Git's own helpers and an askpass bridge, rebase and interactive rebase editor, cherry-pick, revert, reset, tags |

Also in: resizable panels, fullscreen diff, file tree / path view in the diff panel, desktop shortcut with the app icon.

Git ≥ 2.20 is supported; older versions open repositories read-only. All writes go through the Git CLI; `gix` is used only for reads.

## Run

```bash
scripts/run.sh /path/to/repo
```

Add `--release` for an optimized build. Requirements: Rust ≥ 1.80, Node 22+, pnpm, WebKitGTK 4.1 dev libs, Git ≥ 2.20.

### Desktop shortcut

```bash
scripts/install-shortcut.sh
```

Adds **AtlasGit** to the application menu (and the desktop, if there is one). It opens the app without a repository and rebuilds the release binary when sources changed; logs go to `~/.cache/atlasgit/launch.log`. Running it again replaces the old AtlasKraken shortcut.

## Layout

- `crates/ak-gitcli` — git CLI executor, capability detection, machine-format parsers
- `crates/ak-core` — repository reads, commit graph layout, binary row windows, operation engine (`ops/`: lock, journal, branch, merge, stash, remote, history, rebase)
- `app/src-tauri` — Tauri shell (commands, file watcher, askpass bridge)
- `app/ui` — SolidJS UI (`features/`: graph, diff, files, wip, refs, stash, conflict, rebase, journal, split)
- `themes/` — theme token files · `scripts/contrast.mjs` — WCAG AA check
- `tests/fixtures/make.sh` — deterministic fixture repos
- `docs/HANDOFF.md` — current state and rules for contributors and agents

## Tests

```bash
cargo test --workspace
pnpm --dir app/ui test
node scripts/contrast.mjs
```

## Git flow

`main` holds releases, `develop` is the integration branch. Work in `feature/`, `fix/`, `release/` or `hotfix/` branches; pull requests go to `develop`.
