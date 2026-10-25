# Feature Specification: Accessibility audit and fixes

**Feature Branch**: `ccr-07e53df7-35wcgc` (Phase 5 integration branch)
**Created**: 2026-10-15
**Status**: Implemented — merged on the Phase 5 integration branch (`1abb716`); owner checks pending
**Source**: `docs/specs/accessibility.md`, audit in `docs/a11y/audit.md`

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Screen-reader access to the commit graph (Priority: P1)

The graph is a canvas; a hidden DOM mirror lets assistive technology read it.

**Independent Test**: Tab to the graph, arrow keys change `aria-activedescendant`; rows exist only for the visible window.

**Acceptance Scenarios**:

1. **Given** the graph with loaded rows, **Then** a visually hidden `role="row"` per visible row (plus overscan and selected) with `aria-rowindex`, `aria-selected` and gridcells.
2. **Given** unloaded rows, **Then** they are omitted, not rendered empty.

---

### User Story 2 - Keyboard-only context menus and dialogs (Priority: P1)

**Acceptance Scenarios**:

1. **Given** a selected commit, ref row or local stash row, **When** Menu key or Shift+F10 is pressed, **Then** the matching menu opens at the element and focus returns on close.
2. **Given** any dialog, **Then** it has `aria-modal`, an accessible name, a Tab trap, Escape to close and restored focus.

---

### User Story 3 - Written audit (Priority: P2)

**Acceptance Scenarios**:

1. **Given** the audit report, **Then** it lists method, WCAG 2.1 AA criteria, status per UI area, the keyboard map and the owner's manual checks.

### Edge Cases

- Reduced motion: transitions disabled under `prefers-reduced-motion`.
- Focus after a list re-render returns to the list or a neighbour.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001** (A-1): Hidden focus list mirroring visible graph rows with `aria-activedescendant`.
- **FR-002** (A-2): Menu key / Shift+F10 on graph, refs, local and shared stashes.
- **FR-003** (A-3): Ref menu offers "merge into current branch" as an alternative to drag.
- **FR-004** (A-4): Modal semantics and one focus-trap mechanism for all dialogs.
- **FR-005** (A-5/A-6/A-7): Names, roles, states, visible focus, logical order, reduced motion.
- **FR-006** (A-8): `docs/a11y/audit.md`.

## Success Criteria *(mandatory)*

- **SC-001**: UI tests (81 at Phase 5 end), build, `node scripts/contrast.mjs` and the Tauri build pass.
- **SC-002**: Hidden rows bounded by viewport; draw time not worse than before (numbers in the audit).
- **SC-003**: 5 screenshots in `docs/screenshots/phase5/accessibility/`.
- **SC-004**: Orca on the packaged app, WebKitGTK Menu key, F-15, F-23 and focus-ring pairs in `scripts/contrast.mjs` — **pending owner / deferred**.
