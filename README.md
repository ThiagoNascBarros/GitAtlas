<p align="center"><img src="app/src-tauri/icons/icon.png" width="128" alt="GitAtlas icon"></p>

# GitAtlas

A fast, safe, open-source desktop Git client for Linux, built with Tauri 2, a Rust core and a SolidJS interface. Licensed under Apache-2.0.

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

## Git flow

`main` holds releases, `develop` is the integration branch. Work in `feature/`, `fix/`, `release/` or `hotfix/` branches; pull requests go to `develop`.
