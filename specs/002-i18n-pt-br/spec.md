# Feature Specification: pt-BR translation and locale selection

**Feature Branch**: `ccr-07e53df7-35wcgc` (Phase 5 integration branch)
**Created**: 2026-10-15
**Status**: Implemented — merged on the Phase 5 integration branch (`5ae6256`), not yet in `develop`
**Source**: `docs/specs/i18n-pt-br.md`

## User Scenarios & Testing *(mandatory)*

### User Story 1 - UI in Brazilian Portuguese (Priority: P1)

A pt-BR user gets the whole UI in Portuguese, following the system language by default.

**Independent Test**: set the browser language to `pt-PT`, open the app, all texts and dates are pt-BR.

**Acceptance Scenarios**:

1. **Given** choice `system` and first supported language `pt*`, **Then** the locale is `pt-BR`; otherwise `en`.
2. **Given** pt-BR is active, **Then** no user-visible English remains except Git terms from the glossary and raw Git/core output.

---

### User Story 2 - Switch language live (Priority: P2)

**Acceptance Scenarios**:

1. **Given** the toolbar picker (Follow system / English / Português (Brasil)), **When** changed, **Then** every text, the graph date column and `<html lang>` update without reload.
2. **Given** a choice was made, **Then** it survives reload (`localStorage["ak.lang"]`); an invalid stored value falls back to `system`.

---

### User Story 3 - Understandable errors (Priority: P2)

**Acceptance Scenarios**:

1. **Given** a core/shell error code with no text yet (`INVALID_PATH`, `IO_ERROR`, `GIT_NOT_FOUND`, `NOT_A_REPO`, `READ_FAILED`, `UNKNOWN_REPO`), **Then** `err.<CODE>` exists in both tables.
2. **Given** opening a repo fails, **Then** `errorText` (+ `errorFix`) is shown instead of the raw message.

### Edge Cases

- Storage unreadable or blocked: ignored, default `system`.
- Placeholders (`{n}`, `{path}`…) must match in both tables.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `resolveLocale(choice, systemLanguages)` pure and unit-tested.
- **FR-002**: Choice persisted in `localStorage["ak.lang"]`, default `system`.
- **FR-003**: Locale is a Solid signal read inside `t()`; live re-render.
- **FR-004**: Toolbar language picker with accessible name `toolbar.language`.
- **FR-005**: Dates/numbers formatted with the active locale; `datetime=` ISO values unchanged.
- **FR-006**: `<html lang>` follows the active locale.
- **FR-007**: `locales/pt-BR.ts` typed so a missing or extra key fails `tsc`.
- **FR-008**: Add missing `err.<CODE>` texts in both tables.
- **FR-009**: Open-repo error uses `errorText`/`errorFix`.

## Success Criteria *(mandatory)*

- **SC-001**: UI tests, `pnpm --dir app/ui build`, `node scripts/contrast.mjs` and the Tauri build pass (Phase 5 end state: UI 81 tests).
- **SC-002**: 5 screenshots in `docs/screenshots/phase5/i18n-pt-br/`.
- **SC-003**: Owner review of the pt-BR glossary and the long "compartilhado" graph label — **pending owner**.
