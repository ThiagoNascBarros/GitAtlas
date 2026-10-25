# Tasks: pt-BR and locale selection

**Spec**: `specs/002-i18n-pt-br/spec.md`

## Phase 1: Locale engine
- [X] T001 [US1] Failing tests in `i18n.test.ts` (resolveLocale, table and placeholder parity, persistence)
- [X] T002 [US1] Locale signal, resolution, persistence and `<html lang>` in `i18n.ts`
- [X] T003 [US1] Create `locales/pt-BR.ts` with type-enforced parity

## Phase 2: Errors and formatting
- [X] T004 [US3] Audit error codes and add missing `err.*` keys in both tables
- [X] T005 [US3] Open-repo error shows `errorText`/`errorFix`
- [X] T006 [US2] Locale-aware date/number call sites; graph redraw on locale change

## Phase 3: Picker and validation
- [X] T007 [US2] Toolbar picker and `initLocale()` wiring
- [X] T008 [US1] Run UI tests, build, contrast and Tauri build
- [X] T009 [US1] Browser check with the dev mock and 5 screenshots
- [ ] T010 [US1] Owner review of the pt-BR glossary and the long "compartilhado" label
