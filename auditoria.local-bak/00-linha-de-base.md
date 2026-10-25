# Baseline (Phase 0)

Date: 2026-10-15. Host: Ubuntu, 7 GB RAM, Wayland session (`WAYLAND_DISPLAY=wayland-0`), webkit2gtk-4.1 2.52.6.

| Command | Result | Duration |
|---|---|---|
| `cargo build --workspace` (1st try) | **FAIL** rc=101 — tauri build script read `/home/thigas/Documentos/Develop/AtlasEcoSystem/AtlasKraken/target/.../app_hide.toml` (stale `target/` from the old repo path `AtlasKraken`) | 1 s |
| `cargo clean -p atlasgit -p tauri -p tauri-utils` + `cargo build --workspace` | OK | 23 s |
| `cargo test --workspace` | OK — 120 passed, 0 failed, 2 ignored | 7 s |
| `cargo clippy --workspace -- -D warnings` | OK, no warnings | 13 s |
| `cargo fmt --check` (extra) | 565 diff hunks (known, HANDOFF) | <1 s |
| `pnpm --dir app/ui install --frozen-lockfile` (project uses pnpm; `npm ci` not applicable, no package-lock) | OK | <1 s |
| `pnpm --dir app/ui run build` (tsc + vite) | OK | 5 s |
| `pnpm --dir app/ui test` (vitest) | OK — 12 files, 81 tests | 1 s |
| `node scripts/contrast.mjs` | OK — atlas-dark, atlas-light, 0 failures | <1 s |
| `tests/bench` | no runner there; only the committed `report-wide-100k.json` (core: layout 4.07 ms, first window 0.08 ms, 33 B/commit, core peak RSS 76.8 MB). Not re-run: fixture generation `tests/fixtures/make.sh` is heavy on 7 GB RAM. `not_measured`: UI first paint, scroll fps, app idle RSS | — |
| `scripts/run.sh` | built UI + Tauri, app ran until the 60 s timeout (rc=124, no crash); window not inspected (no screenshot taken) | 60 s |

Finding from baseline: **a stale `target/` made the first workspace build fail** after the repo directory rename (AtlasKraken → AtlasGit). Fix is local (`cargo clean -p`), but CI/dev docs do not mention it.
