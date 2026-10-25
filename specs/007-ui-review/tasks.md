# Tasks: UI review

Picks come from `auditoria.local-bak/03..05` (impact high, risk low, testable without a browser). Each task is TDD: failing vitest first.

- [x] T001 Triage `auditoria.local-bak/03-ui-visual.md`, `04-modais-componentes.md`, `05-layout-responsividade.md` into the picks below
- [x] T002 [UIV-6] Panels typography: `--font-ui` starts with Open Sans (GitKraken look) — `src/styles.test.ts`
- [x] T003 [density] Panel/menu rows 22px: final `--row-h` is 22px — `src/styles.test.ts`
- [x] T004 [UIV-5] Input/select borders use `border.strong` (>= 3:1 vs `bg.raised` in both themes) — `src/styles.test.ts`
- [x] T005 [LAY-1] Saved splitter sizes are clamped to the pane limits (`readSize`) — `features/split/size.test.ts`
- [ ] T006 [MOD-7] Escape closes only the innermost overlay (`handlesEscape`) — `features/overlay.test.ts`
- [ ] T007 [UIV-1] Theme choice persists (`parseThemeChoice`, key `ak.theme`) — `theme/choice.test.ts`

Skipped (not picked): LAY-3 zoom (G effort), MOD-1/2/3/4 confirmations and ErrorBoundary (UX flows, need component tests), LAY-2/5/6 (structural), UIV-2/3 palette changes (touch theme JSON beyond scope).
