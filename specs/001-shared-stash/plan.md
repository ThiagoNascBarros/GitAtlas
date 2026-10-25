# Implementation Plan: Shared Stash

**Branch**: `feature/shared-stash` | **Spec**: `specs/001-shared-stash/spec.md` | **Source**: `docs/specs/shared-stash-plan.md`

## Summary
Core model and engine operations in `ak-core`, Tauri commands and fetch-mode setting in the shell, SolidJS panel in the UI. TDD throughout; one commit per task.

## Technical Context
**Language**: Rust (core, shell), TypeScript/SolidJS (UI) | **Testing**: cargo test, vitest | **Target**: Linux desktop, Git >= 2.20

## Project Structure
- `crates/ak-core/src/ops/shared_stash*.rs`, `lock.rs` (`Lock::Remote`), `crates/ak-core/tests/shared_stash.rs`
- `app/src-tauri/src/{shared_stash,settings}.rs`
- `app/ui/src/features/shared-stash/` (`logic.ts`, `model.ts`, `SharedStashPanel`, `ShareDialog`, `SharedStashMenu`), `features/stash/LocalStashMenu.tsx`
- Docs: `docs/user/shared-stash.md`

## Milestones
M1 core model → M2 engine ops → M3 e2e (checkpoint) → M4 Tauri shell → M5 UI (checkpoint) → M6 close-out.

## Notes
Deviations and implementation notes: spec §13 (M1–M4) and §14 (M5–M6). Known UI follow-ups: no size warning before publish, no real `user.name` slug before first share, no sha→row lookup (glyph only on loaded rows), untracked files absent from the read-only diff.
