# Fase 0: Linha de base

Executada na nuvem da Anthropic (Linux x64, Git 2.43.0, Node 22, Rust stable) sobre `develop` @ `02ec246`. Comandos da raiz do repositório.

| Comando | Resultado | Duração | Observação |
|---|---|---|---|
| `cargo build --workspace` | FALHOU (exit 101) | 54 s | `gdk-sys` precisa de `gdk-3.0` via pkg-config; o container não tem as libs de sistema do WebKitGTK/Tauri. Falha do ambiente, não do código. |
| `cargo test --workspace` | FALHOU (exit 101) | 4 s | Mesma causa (compila o crate `atlasgit` do Tauri). |
| `cargo clippy --workspace -- -D warnings` | FALHOU (exit 101) | 13 s | Mesma causa. |
| `cargo test -p ak-core -p ak-gitcli` (1ª execução) | FALHOU (3 testes de `tests/graph.rs`) | 17 s | Veja achado QA-01 (corrida na geração de fixtures). |
| `cargo test -p ak-core -p ak-gitcli --no-fail-fast` (2ª execução) | OK | ~40 s | 21 + 7 (1 ignorado) + 44 + 31 + 8 testes passaram. |
| `cargo clippy -p ak-core -p ak-gitcli --all-targets -- -D warnings` | OK, 0 avisos | 6 s | |
| `npm install` em `app/ui` | OK | 23 s | O projeto usa pnpm (`pnpm-lock.yaml`); usei npm por ser o disponível. O `package-lock.json` gerado foi removido. |
| `npm run build` (`tsc --noEmit` + vite) | OK | 8 s | JS 192,8 kB (59,8 kB gzip), CSS 36,6 kB. |
| `npm test` (vitest) | OK: 81 testes em 12 arquivos | 3 s | Bate com o HANDOFF (UI 81). |
| `node scripts/contrast.mjs` | OK: 0 falhas (atlas-dark, atlas-light) | <1 s | |
| Benchmarks `tests/bench` | Não executados | n/d | Só existe `report-wide-100k.json` (resultado já gerado). O próprio relatório declara "not_measured": primeiro paint da UI, fps de scroll e RSS do app. |
| `scripts/run.sh` (executar o app) | Não executado | n/d | Sem display e sem WebKitGTK no container. UI inspecionada só por análise estática. |

## O que não pôde ser verificado
- Testes do crate `app/src-tauri` (5 testes de `settings.rs`): exigem as libs de sistema.
- Qualquer medida de fps, memória real do app, tempo de abertura e travamentos: exigem executar o app na sua máquina (veja "Próximo passo de evidência" no roadmap).

## Dados do relatório de benchmark existente (`tests/bench/report-wide-100k.json`)
100.000 commits, 25 lanes: leitura git 577 ms, topologia 14,7 ms, layout 4,1 ms, primeira janela 0,08 ms, 33 bytes/commit, pico de RSS do core 76,8 MB. Cobre só o core Rust, não o WebKit.
