# Git Atlas — instruções para agentes de IA

Cliente Git desktop em Rust + Tauri, com UI em SolidJS/Vite/TypeScript (`app/ui`), core em `crates/` e shell Tauri em `app/src-tauri`.

## Skills a usar neste projeto

Instaladas globalmente (`~/.claude/skills`, `~/.agents/skills`). Use-as quando a tarefa se encaixar:

- `rust-best-practices` — ao escrever, revisar ou refatorar qualquer código Rust (ownership, tratamento de erros com `Result`, performance).
- `rust-async-patterns` — ao mexer em código assíncrono: file watcher, comandos Tauri, operações concorrentes.
- `rust-testing` — ao escrever testes (unitários, integração, async, property-based), especialmente do motor de operações (locks, journal, undo/redo).
- `git-guardrails-claude-code` — bloqueia comandos git destrutivos (push, reset --hard, clean, branch -D). Se os hooks ainda não estiverem configurados, rode a skill para configurá-los.

Também disponível: o plugin `superpowers` (fluxos de planejamento, TDD e depuração).

## Fluxo Git

`main` / `develop`, com branches `feature/`, `fix/`, `release/` e `hotfix/`. Pull requests vão para `develop`. Nunca faça push, reset --hard ou delete de branch sem pedido explícito do usuário.
