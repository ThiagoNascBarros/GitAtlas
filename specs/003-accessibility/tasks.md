# Tasks: Accessibility

**Spec**: `specs/003-accessibility/spec.md`

## Phase 1: Audit
- [X] T001 [US3] Walk every component against A-1…A-7 and draft `docs/a11y/audit.md`

## Phase 2: Graph and menus
- [X] T002 [US1] TDD `a11y.ts` (`visibleRange`, `rowText`, `menuAnchor`, `isMenuKey`)
- [X] T003 [US1] Hidden row mirror and `aria-activedescendant` in `GraphView.tsx`
- [X] T004 [US2] Keyboard menus on graph, refs, local stashes; confirm A-3

## Phase 3: Dialogs and polish
- [X] T005 [US2] One focus-trap mechanism for the 8 dialogs
- [X] T006 [US2] Names/roles/states, focus visibility, reduced motion
- [X] T007 [US1] Strings in both tables; tests, build, contrast, Tauri build
- [X] T008 [US3] Finalise audit (status per finding, keyboard map, owner checks) and 5 screenshots
- [ ] T009 [US3] Owner checks: Orca on the packaged app, WebKitGTK Menu key, deferred F-14/F-15/F-23
