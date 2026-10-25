<p align="center"><img src="app/src-tauri/icons/icon.png" width="128" alt="GitAtlas icon"></p>

# GitAtlas

A fast, safe, open-source desktop Git client for Linux, built with Tauri 2, a Rust core and a SolidJS interface. Licensed under Apache-2.0.

> **Why this exists.** GitAtlas was made as an open-source alternative to [GitKraken](https://www.gitkraken.com/): the same kind of experience (a visual commit graph, staging, branches, merges, stashes, remotes and rebase in one window) without a subscription, an account or a closed codebase. It is an independent project and is not affiliated with or endorsed by GitKraken.

## Highlights

- **Commit graph** drawn on a canvas with virtualized rows and a per-repository cache, so large histories stay smooth.
- **Everything is undoable.** Each write goes through an operation engine with a lock, a journal and snapshots; Undo/Redo works for commits, amends, discards, resets and more.
- **Git does the writing.** All changes run through your own `git` (with your credential helpers and SSH setup); `gix` is used only for reads.
- **Branching and merging** with conflict preview, a 3-pane conflict editor, drag-a-ref-onto-the-branch to merge, plus interactive rebase, cherry-pick, revert, reset and tags.
- **Shared Stash:** share a local stash through a plain Git remote, with no server and no extra branch.
- **Accessible and localized:** keyboard menus, screen-reader rows for the graph, Atlas Dark/Light themes (WCAG AA checked), English and pt-BR.

> Formerly **AtlasGit**, and before that **AtlasKraken**. Journals, snapshots and backups still live under `<git-dir>/atlaskraken/` and `refs/atlaskraken/`. Shared stashes and settings use `<git-dir>/gitatlas/` and `refs/gitatlas/`; ones made as AtlasGit are not read and must be shared again.

## Status

Phases 1–5 and the Shared Stash addendum are done: the MVP (version 0.1.0) is feature-complete.
Cutting the release waits on the owner checks listed below and in the
[release checklist](docs/release/checklist.md).

| Phase | What it delivers |
|-------|------------------|
| 1 — Read-only viewer | Open a repo, full commit graph (canvas, lanes, virtualized rows, graph cache), commit details, file diffs, Atlas Dark/Light themes |
| 2 — Operation engine | Stage/unstage files and hunks, commit, amend, discard, Undo/Redo backed by a journal and snapshots, journal panel with opt-in cleanup, file watcher |
| 3 — Branching & merge | Create/rename/delete/checkout branches, detached checkout of tags/remotes, merge with conflict preview (`git merge-tree`), drag a ref onto the current branch to merge, 3-pane conflict editor, abort, stash push/pop/apply/drop with labels on the graph, refs filter |
| 4 — Remotes & rebase | Fetch/pull/push with credentials through Git's own helpers and an askpass bridge, rebase and interactive rebase editor, cherry-pick, revert, reset, tags |
| 4 addendum — Shared Stash | Share a local stash through a Git remote (no branch, no server): list, apply, branch from, update, rename, unshare/reshare; verified/claimed authors; graph marks |
| 5 — MVP release | pt-BR interface with a language picker; accessibility audit (keyboard menus, screen-reader rows for the graph, modal dialogs); desktop entry, AppStream metadata, .deb, AppImage and Flatpak packaging with a release workflow; user guide, release checklist, changelog |

Also in: resizable panels, fullscreen diff, file tree / path view in the diff panel, desktop shortcut with the app icon.

Pending owner checks before the release: manual test of each package (PRD criteria 4, 7 and 14,
HTTPS/SSH remotes, signing, pt-BR, keyboard, Orca), the first Flatpak build in the release
workflow, the SonarCloud finding on PR #2, and the open decisions in
`docs/specs/mvp-phase5.md` §8 (version, pt-BR glossary, Flathub, package naming).

Git ≥ 2.20 is supported; older versions open repositories read-only. All writes go through the Git CLI; `gix` is used only for reads.

## Install

Packages are built from this repository; there is no public download yet. Details, removal and
the Flatpak sandbox limits: [docs/user/install.md](docs/user/install.md).

| Package | Build | Install |
|---|---|---|
| .deb (Ubuntu 22.04+, Debian) | `scripts/package.sh deb` | `sudo apt install ./target/release/bundle/deb/GitAtlas_0.1.0_amd64.deb` (package `git-atlas`, pulls in Git ≥ 2.20) |
| AppImage | `scripts/package.sh appimage` | `chmod +x GitAtlas_0.1.0_amd64.AppImage` and run it (uses the system Git) |
| Flatpak bundle | `scripts/package-flatpak.sh` | `flatpak install --user target/flatpak/GitAtlas-0.1.0.flatpak`, then `flatpak run dev.gitatlas.app` (bundles Git 2.56.0; needs the Flathub remote for the GNOME 51 runtime) |

The **Release packages** workflow (`.github/workflows/release.yml`, on `v*` tags or by hand)
builds all three on GitHub and uploads them as artifacts; it never publishes a release.

## User guide

[docs/user/index.md](docs/user/index.md): installing, getting started, history and diffs,
committing and undo, branches/merges/stashes, remotes and rebase, shared stash, settings,
keyboard and accessibility, troubleshooting. Changes per phase: [CHANGELOG.md](CHANGELOG.md).

## Run

```bash
scripts/run.sh /path/to/repo
```

Add `--release` for an optimized build. Requirements: Rust ≥ 1.80, Node 22+, pnpm, WebKitGTK 4.1 dev libs, Git ≥ 2.20.

### Desktop shortcut

```bash
scripts/install-shortcut.sh
```

Adds **GitAtlas** to the application menu (and the desktop, if there is one). It opens the app without a repository and rebuilds the release binary when sources changed; logs go to `~/.cache/gitatlas/launch.log`. Running it again replaces the old AtlasKraken shortcut.

## Architecture

How the crates, the Tauri shell and the UI fit together, and how a write operation flows through the journal: [docs/architecture.md](docs/architecture.md).

## Layout

- `crates/ak-gitcli` — git CLI executor, capability detection, machine-format parsers
- `crates/ak-core` — repository reads, commit graph layout, binary row windows, operation engine (`ops/`: lock, journal, branch, merge, stash, remote, history, rebase, shared stash)
- `app/src-tauri` — Tauri shell (commands, file watcher, askpass bridge, shared stash commands, settings)
- `app/ui` — SolidJS UI (`features/`: graph, diff, files, wip, refs, stash, shared-stash, conflict, rebase, journal, split; strings in `i18n.ts` (en) and `locales/pt-BR.ts`)
- `themes/` — theme token files · `scripts/contrast.mjs` — WCAG AA check
- `tests/fixtures/make.sh` — deterministic fixture repos
- `packaging/` — desktop entry, AppStream metainfo, Flatpak manifest · `scripts/package.sh`, `scripts/package-flatpak.sh` — packages
- `docs/user/` — user guide · `docs/release/checklist.md` — release checklist · `docs/a11y/audit.md` — accessibility audit · `docs/specs/` — specs
- `docs/HANDOFF.md` — current state and rules for contributors and agents

## Tests

```bash
cargo test --workspace
pnpm --dir app/ui test
node scripts/contrast.mjs
```

## Contributing

Pull requests go to `develop`. Read [docs/HANDOFF.md](docs/HANDOFF.md) for the current state and the rules for contributors and agents, and [docs/architecture.md](docs/architecture.md) before touching the operation engine.

## Git flow

`main` holds releases, `develop` is the integration branch. Work in `feature/`, `fix/`, `release/` or `hotfix/` branches; pull requests go to `develop`.
