# GitAtlas architecture

A short map for contributors. The detailed decisions live in `docs/specs/` and in [HANDOFF.md](HANDOFF.md).

## Layers

```
app/ui (SolidJS, Vite)      screens, graph canvas, dialogs, i18n (en + pt-BR)
        │  Tauri IPC (commands + events)
app/src-tauri (Rust)        window, commands, file watcher, askpass bridge, settings, diagnostics
        │
crates/ak-core              repository reads, commit graph layout, operation engine (ops/)
        │
crates/ak-gitcli            runs `git`, detects its version/capabilities, parses machine formats
```

The UI can only call the commands registered in `app/src-tauri/src/main.rs`; the Tauri capability
file allows a folder picker and nothing else (no fs, shell or http).

## Crates

| Crate | Role |
|---|---|
| `ak-gitcli` | Spawns Git, checks the version (≥ 2.20 to write; older opens read-only), parses `log`, `status`, `diff`, … output. |
| `ak-core` | `read` / `read_gix` (reads, `gix` only here), `graph` (lane layout and binary row windows for the UI), `cache` (per-repo graph cache in `$XDG_CACHE_HOME/gitatlas/graph/`), `ops/` (every write). |

The crate prefix `ak-` is a leftover from the AtlasKraken name; the crates are internal and were not renamed.

## Reading and the graph

The core keeps only SHAs, topology and the computed layout in memory. Row text (subject, author,
date) is fetched on demand with `git log --no-walk --stdin`, so a 100k-commit repository idles
near 230 MB. The UI receives binary row windows and draws only the visible rows on a canvas.
A hidden `role="row"` mirror of the visible rows serves screen readers.

## Writing: the operation engine

Every change goes through `ops::Engine` (`crates/ak-core/src/ops/mod.rs`):

1. **Lock** (`lock.rs`): one writer per repository; a busy repository fails with `REPO_BUSY`.
2. **Run** the Git CLI command(s) for the operation (stage, commit, branch, merge, stash, remote, history, rebase, shared stash).
3. **Journal** (`journal.rs`): append the operation and the exact Git commands to `<git-dir>/atlaskraken/journal.jsonl`, with what Undo needs.
4. **Snapshots**: commits that would become unreachable (amend, reset, undone commits) are kept under `refs/atlaskraken/snapshots/`; discarded files are backed up under `<git-dir>/atlaskraken/discarded/`.

Undo and Redo replay the journal; `HEAD_MOVED` and similar stable error codes (translated in the
UI) stop an undo that would no longer be safe. Journal cleanup is opt-in. Nothing writes to `.git`
silently.

## Remotes and credentials

Fetch, pull and push run Git with the user's own credential helpers and SSH agent. When Git needs
a secret, the shell re-executes its own binary as `GIT_ASKPASS` (`askpass.rs`, socket path from
`GITATLAS_ASKPASS_SOCK`) and the UI shows a dialog; secrets are not stored by GitAtlas.

## Shared Stash

A stash can be published as a ref under `refs/gitatlas/shared-stash/<author>/<id>` on any Git remote.
The commit message carries `GitAtlas-*` trailers (author email, base, timestamps). Only the author
may update or delete it. Local bookkeeping is in `<git-dir>/gitatlas/`. Spec: [specs/shared-stash.md](specs/shared-stash.md).

## Names from earlier versions

The app was AtlasKraken, then AtlasGit, and is now GitAtlas. Journal, snapshot and backup data
keeps the AtlasKraken names (`.git/atlaskraken/`, `refs/atlaskraken/`). Shared stash data uses
`.git/gitatlas/`, `refs/gitatlas/` and `GitAtlas-*` trailers; data written under the AtlasGit
names has no migration. Do not rename any of these without one.

## Tests

```bash
cargo test --workspace          # unit + integration on throwaway repos (crates/ak-core/tests)
pnpm --dir app/ui test          # UI logic (vitest)
node scripts/contrast.mjs       # theme contrast, WCAG AA
```

Fixture repositories are built deterministically by `tests/fixtures/make.sh`; the graph has golden
files in `crates/ak-core/tests/golden/`.
