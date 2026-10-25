# Tasks: Shared Stash

**Spec**: `specs/001-shared-stash/spec.md` | **Plan**: `plan.md`

## Phase 1: Core model (M1)
- [X] T001 [US1] Slug: `author_slug(name, email)` in `crates/ak-core/src/ops/shared_stash.rs`
- [X] T002 [US1] Trailers: `SharedStashMeta`, `format_message` / `parse_message`
- [X] T003 [US1] Ref name builders/parsers (remote, mirror, keep, probe)
- [X] T004 [US2] `validate(wrapper_sha)` with stash-shape checks
- [X] T005 [US2] `Engine::shared_stashes()` list with `mine` and `verified`

## Phase 2: Engine operations (M2)
- [X] T006 [US1] `Lock::Remote` in `lock.rs`
- [X] T007 [US1] Error codes in `OpError`
- [X] T008 [US4] Fetch with prune and per-remote results
- [X] T009 [US1] Remote capability probe and cache
- [X] T010 [US1] Publish (size warning, signing, journal entry)
- [X] T011 [US3] Update / Rename with lease (`NOT_AUTHOR`, `STALE_SHARED_STASH`)
- [X] T012 [US3] Unshare / Reshare / Discard keep
- [X] T013 [US2] Apply with `--index` fallback and `DIRTY_TREE`
- [X] T014 [US2] Branch from shared stash with rollback
- [X] T015 [US1] Journal/undo wiring: network kinds not undoable

## Phase 3: E2E (M3)
- [X] T016 [US1] Integration suite for spec §9 criteria 1–12
- [X] T017 [US1] CI matrix on Git 2.20.5 and current Git

## Phase 4: Tauri shell (M4)
- [X] T018 [US2] `sharedStash.fetchMode` setting and Fetch/Pull hook
- [X] T019 [US1] Tauri commands (list, fetch, publish, update, unshare, reshare, discard_keep, apply, branch, probe_reset)

## Phase 5: UI (M5)
- [X] T020 [US2] Sidebar tab, grouping, badges, invalid rows
- [X] T021 [US4] Loading/empty/error/rejected/"Recently unshared" states
- [X] T022 [US1] Share dialog
- [X] T023 [US3] Context menus, update picker, rename, unshare confirm
- [X] T024 [US2] Graph glyph and read-only diff
- [X] T025 [US2] Keyboard navigation, i18n strings, `devMock.ts`

## Phase 6: Close-out (M6)
- [X] T026 User docs and HANDOFF update
- [ ] T027 `/code-review` pass and manual run with two clones (owner); PR #12 already merged
