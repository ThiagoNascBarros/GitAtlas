---
status: draft
feature: Shared Stash
spec: docs/specs/shared-stash.md
branch: feature/shared-stash (from develop, PR to develop)
---

# Shared Stash — implementation plan

TDD throughout: each task starts with a failing test. Heavy processes one at a time (`cargo test -p ak-core` before the full workspace). Each task ends with `cargo fmt`, `cargo clippy -- -D warnings`, tests green, one commit.

## Milestone 1 — Core model (`crates/ak-core/src/ops/shared_stash.rs`)

- **T1. Slug.** `author_slug(name, email) -> String` (accents, invalid ref chars, collapse, 40 chars, email fallback). Unit + property tests (always a valid ref component via `git check-ref-format`).
- **T2. Trailers.** `SharedStashMeta` struct; `format_message` / `parse_message` (title ≤ 120, single line). Round-trip property test; malformed inputs → `None`.
- **T3. Ref names.** Builders/parsers for remote ref, mirror ref, keep ref, probe ref; tests.
- **T4. Validation.** `validate(wrapper_sha) -> Result<SharedStash, Invalid>`: trailers + stash shape (`^1 == Base`, `^2` exists, optional `^3`). Integration test with a real temp repo (valid, non-merge parent, wrong base).
- **T5. List.** `Engine::shared_stashes()` reading mirror refs of all remotes; `mine` by normalized email; invalid rows kept with flag. Signature check → `verified` (gpg/ssh via `git verify-commit`, false when unavailable).

## Milestone 2 — Engine operations

- **T6. `Lock::Remote`** in `lock.rs` + test that two network ops conflict and Index ops don't.
- **T7. Error codes** in `OpError` constructors (§5.1 list).
- **T8. Fetch.** `SharedStashFetch { remote }` with prune; per-remote results (`Vec<(remote, Result)>`). Test with two bare remotes, one missing.
- **T9. Probe.** Throwaway ref push/delete, cache in `.git/atlasgit/remotes.json`; test with a bare remote whose `pre-receive` hook rejects `refs/atlasgit/*`.
- **T10. Publish.** By stash sha; identity/remote checks; size estimate warning (returned in `OpOutcome`, not an error); `commit-tree` (+ `-S` when `commit.gpgsign`); push; mirror fetch; journal entry (not undoable).
- **T11. Update / Rename.** Fresh single-ref fetch → lease from mirror → push; `NOT_AUTHOR`, `STALE_SHARED_STASH` tests (simulate concurrent move from a second clone).
- **T12. Unshare / Reshare / Discard keep.** Keep ref write, lease delete, create-only re-push; tests.
- **T13. Apply.** Validate → `stash apply --index`, fallback without `--index` (flag in outcome); `DIRTY_TREE` mapping; conflict path sets in-progress like local apply.
- **T14. Branch.** Base fetch from same remote if missing → `BASE_MISSING`; branch + checkout + apply with rollback; undoable like BranchCreate.
- **T15. Journal/undo wiring.** Network kinds excluded from `undo`/`redo_candidate`; test.

## Milestone 3 — E2E (PRD criterion 14)

- **T16.** `tests/` integration suite with R1, R2 bare + clones A, B covering spec §9 criteria 1–12 at engine level (criterion 2 via shell setting logic in T18).
- **T17.** CI: run the suite on Git 2.20.5 and current Git (existing matrix); confirm `fetch --prune` with custom refspec on 2.20.

## Milestone 4 — Tauri shell (`app/src-tauri`)

- **T18.** Setting `sharedStash.fetchMode` (repo + global default, `panel`). Fetch/Pull hook: fetch shared stashes when `always` or when the UI reports the tab open (`shared_stash_tab_open(bool)` command).
- **T19.** Commands: list, fetch, publish, update, unshare, reshare, discard_keep, apply, branch, probe_reset; network on `spawn_blocking` with askpass bridge; cancel support.

## Milestone 5 — UI (`app/ui/src/features/shared-stash`)

- **T20.** Sidebar tab "Shared stash" next to Local/Remote; grouping remote → Mine/Team; badges; invalid rows; header (refresh, mode toggle, last fetched).
- **T21.** States: loading+cancel, empty, per-remote error, host rejected + Retry check, "Recently unshared".
- **T22.** Share dialog from local stash menu: title, remote picker, public-data/name warning, size warning.
- **T23.** Context menus (author vs others), Update-from-local-stash picker, Rename, Unshare confirm.
- **T24.** Graph glyph on base commit + view toggle; row click → read-only diff.
- **T25.** Keyboard navigation; strings in `i18n.ts`; `devMock.ts` minimal mocks.

## Milestone 6 — Close-out

- **T26.** User docs (incl. "ownership is declarative", host compatibility), update `docs/HANDOFF.md`.
- **T27.** Self code review + `/code-review`, run app manually with `scripts/run.sh` against two clones, open PR to `develop`.

## Order and checkpoints

M1 → M2 → M3 (checkpoint: engine fully tested, report to owner) → M4 → M5 (checkpoint: screenshots to owner) → M6.
