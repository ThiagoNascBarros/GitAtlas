# Linha de base (Fase 0)

Data: 2026-10-15. Máquina: Ubuntu, 7 GB RAM, sessão Wayland (`WAYLAND_DISPLAY=wayland-0`), webkit2gtk-4.1 2.52.6.

| Comando | Resultado | Duração |
|---|---|---|
| `cargo build --workspace` (1ª tentativa) | **FALHOU** rc=101 — o build script do tauri leu `/home/thigas/Documentos/Develop/AtlasEcoSystem/AtlasKraken/target/.../app_hide.toml` (`target/` antigo do caminho anterior `AtlasKraken`) | 1 s |
| `cargo clean -p atlasgit -p tauri -p tauri-utils` + `cargo build --workspace` | OK | 23 s |
| `cargo test --workspace` | OK — 120 passaram, 0 falhas, 2 ignorados | 7 s |
| `cargo clippy --workspace -- -D warnings` | OK, sem avisos | 13 s |
| `cargo fmt --check` (extra) | 565 hunks de diferença (conhecido, HANDOFF) | <1 s |
| `pnpm --dir app/ui install --frozen-lockfile` (o projeto usa pnpm; `npm ci` não se aplica, sem package-lock) | OK | <1 s |
| `pnpm --dir app/ui run build` (tsc + vite) | OK | 5 s |
| `pnpm --dir app/ui test` (vitest) | OK — 12 arquivos, 81 testes | 1 s |
| `node scripts/contrast.mjs` | OK — atlas-dark, atlas-light, 0 falhas | <1 s |
| `tests/bench` | sem executor ali; só o `report-wide-100k.json` versionado (core: layout 4,07 ms, primeira janela 0,08 ms, 33 B/commit, pico RSS do core 76,8 MB). Não reexecutado: gerar a fixture (`tests/fixtures/make.sh`) é pesado para 7 GB de RAM. `not_measured`: primeiro paint da UI, fps de scroll, RSS ocioso do app | — |
| `scripts/run.sh` | compilou UI + Tauri, o app rodou até o timeout de 60 s (rc=124, sem crash); janela não inspecionada (sem captura de tela) | 60 s |

Achado da linha de base: **um `target/` obsoleto fez o primeiro build do workspace falhar** após renomear o diretório (AtlasKraken → AtlasGit). A correção é local (`cargo clean -p`), mas a doc de CI/dev não menciona.
