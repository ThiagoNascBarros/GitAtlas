# AtlasKraken

Desktop Git client for Linux — Tauri 2 + Rust core + SolidJS. Licensed under Apache-2.0.

**Status:** Phase 1 (read-only viewer): open a repo, full commit graph, commit details, file diffs, Atlas Dark/Light themes.

## Run

```bash
scripts/run.sh /path/to/repo
```

Requirements: Rust ≥ 1.80, Node 22+, pnpm, WebKitGTK 4.1 dev libs, Git ≥ 2.20.

## Layout

- `crates/ak-gitcli` — git CLI executor, capability detection, machine-format parsers
- `crates/ak-core` — repository reads, commit graph layout, binary row windows
- `app/src-tauri` — Tauri shell (read-only commands)
- `app/ui` — SolidJS UI (canvas graph, diff, theme engine)
- `themes/` — theme token files · `scripts/contrast.mjs` — WCAG AA check
- `tests/fixtures/make.sh` — deterministic fixture repos

## Tests

```bash
cargo test --workspace
pnpm --dir app/ui test
node scripts/contrast.mjs
```
