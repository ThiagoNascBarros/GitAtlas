---
status: draft
feature: Accessibility audit and fixes (keyboard + screen reader)
phase: 5 (module accessibility)
overview: docs/specs/mvp-phase5.md
depends-on: docs/specs/i18n-pt-br.md (merged first)
date: 2026-10-15
---

# Accessibility — audit and MVP fixes

## 1. Objective

The roadmap requires an accessibility audit before the MVP release and names one known gap:
"the canvas graph needs the hidden focus list" (HANDOFF). The commit graph is drawn on a
`<canvas>`, so screen readers see one focusable `role="grid"` with no rows; today selection is
only announced through a live region. Several context menus open only with the mouse and dialogs
do not keep focus inside. Result: the core flows (open repo, browse history, inspect a commit,
stage/commit, branch/merge, stash, fetch/pull/push, shared stash) are operable with the keyboard
alone and understandable with a screen reader, against WCAG 2.1 AA, with a written audit.

## 2. Scope

**In scope**
- Graph hidden focus list (ARIA grid rows mirrored in the DOM for the visible window).
- Keyboard access to every context menu (graph rows, ref rows, local stash rows; shared stash
  rows already have it).
- Modal semantics and focus containment for all dialogs.
- Audit of names/roles/states, focus order and visibility, status messages, form labels,
  reduced motion; fixes for what fails, within the files listed in `mvp-phase5.md` §5.
- Audit report `docs/a11y/audit.md`.
- New strings in **both** `en` (`i18n.ts`) and pt-BR (`locales/pt-BR.ts`) tables.

**Out of scope**
- Real screen reader testing (Orca on the packaged app) — listed as an owner check in the report.
- High-contrast theme, UI scaling settings, keyboard shortcut customization.
- Keyboard column resizing in the graph header (layout convenience; record as deferred).
- Multi-select/drag in WIP (deferred since Phase 2), sha-to-row lookup (deferred).
- Any change to `crates/**` or `app/src-tauri/**`; new dependencies (no jsdom, no a11y libs).

## 3. Repository context

- `features/graph/GraphView.tsx`: scroller `div.graph-scroller` `tabindex=0 role=grid
  aria-label aria-rowcount`; `ROW_H=28`, rows loaded in chunks of 200 (`rowAt(i)` returns
  `{g, m}` when loaded); WIP row drawn above row 0 when `props.wip` (offset `off()`); keys
  ArrowUp/Down, PageUp/Down, Home/End in `onKey`; mouse `onContextMenu` → `props.onMenu(sha,x,y)`;
  live region `div.sr-only[aria-live=polite]` announces selection/WIP.
- `features/overlay.ts`: `useOverlay` (Escape closes, focus in on open, restored on close),
  `menuKeys` (arrow navigation in menus).
- Menus: `features/graph/CommitMenu.tsx`, `features/refs/RefMenu.tsx` (also hosts the merge
  dialog, so "merge into current" is reachable from the ref menu), `features/stash/
  LocalStashMenu.tsx`, `features/shared-stash/SharedStashMenu.tsx`. Shared stash rows already
  open their menu with Menu / Shift+F10 (`SharedStashPanel.tsx`) — the reference pattern.
- Dialogs: `<dialog open class="modal">` in `JournalPanel.tsx`, `NameDialog.tsx`,
  `RefMenu.tsx`, `ShareDialog.tsx`, `SharedStashMenu.tsx` (×2); check `ConflictEditor.tsx`,
  `RebaseEditor.tsx` too.
- Ref rows: `App.tsx` refs list, `<button>` with mouse `onContextMenu` only and drag-and-drop.
- Local stash rows: `features/stash/StashPanel.tsx` `<li onContextMenu>` (not focusable).
- Focus styles: `styles.css` `:focus-visible` rules; contrast checked by `scripts/contrast.mjs`.
- Tests run in `environment: "node"` (no DOM) → testable logic goes in pure modules.

## 4. Requirements

### 4.1 Functional

- **A-1 Graph hidden focus list.**
  - Inside the grid element render, visually hidden, one `role="row"` element per row in the
    visible window plus overscan (and the selected row if outside it), each with a stable id
    (e.g. `graph-row-<index>`; WIP `graph-row-wip`), `aria-rowindex` (1-based, counting the WIP
    row when present), `aria-selected`, and `role="gridcell"` children for refs, message,
    author, date, sha (same text the canvas draws; date formatted with the active locale).
  - The grid has `aria-activedescendant` = selected row id, `aria-rowcount` = total rows (+1
    with WIP), `aria-colcount`, `aria-multiselectable="false"`.
  - Rows not yet loaded are omitted (not empty rows).
  - Shared-stash glyph information currently in the live region becomes part of the row's
    accessible text (key `ss.glyphTitle`).
  - The live region stops announcing selection (activedescendant does it); remove it unless it
    still carries other messages.
  - No visual change; mouse behaviour unchanged.
- **A-2 Keyboard context menus.** `ContextMenu` key and `Shift+F10`:
  - on the graph with a selected commit → `CommitMenu` positioned at the selected row (left edge
    of the message column, bottom of the row, inside the viewport); nothing on the WIP row;
  - on a focused ref row button → `RefMenu` at the button's rect;
  - on a local stash row → `LocalStashMenu` at the row's rect; make the row focusable (button or
    `tabindex=0` with a role) without changing its look.
  - Focus moves into the menu (existing `useOverlay`) and back on close (existing).
- **A-3 Drag-and-drop alternative.** Confirm the ref menu offers "merge into current branch" for
  the same refs that can be dragged; if not, add it. Record in the audit.
- **A-4 Modal dialogs.** Every dialog: `aria-modal="true"`, accessible name
  (`aria-labelledby` preferred), initial focus on the first field or the safe button, Tab and
  Shift+Tab cycle inside the dialog, Escape closes (existing), focus restored (existing). Either
  `HTMLDialogElement.showModal()` or a `trapFocus` helper in `overlay.ts`; choose one, use it
  everywhere, record why. Content behind must not be reachable by Tab while a dialog is open.
- **A-5 Names, roles, states.** Every interactive element has an accessible name (icon-only
  buttons: `aria-label` from i18n); toggles expose `aria-pressed`/`aria-expanded`; decorative
  icons `aria-hidden`; form fields have labels; toasts/banners use `role="status"`/`alert` as
  appropriate (verify current toast markup in `App.tsx`).
- **A-6 Focus.** Visible focus indicator on every focusable element in both themes; logical
  order (toolbar → sidebar → graph → details/WIP → diff); no focus loss after an operation
  re-renders a list (focus returns to the list or a sensible neighbour).
- **A-7 Motion.** If CSS transitions/animations exist, disable or shorten them under
  `prefers-reduced-motion: reduce`.
- **A-8 Audit report** `docs/a11y/audit.md`: method, WCAG 2.1 AA criteria checked (at least
  1.1.1, 1.3.1, 1.4.3, 1.4.11, 2.1.1, 2.1.2, 2.4.3, 2.4.7, 3.3.2, 4.1.2, 4.1.3), table per UI
  area (pass / fixed in this phase with commit / deferred with reason), keyboard map of the app
  (used by `release-docs`), and the owner's manual checks (Orca + packaged app).

### 4.2 Non-functional

- **Performance:** hidden rows are bounded by the viewport (≈ height/28 + overscan ≤ 2×20);
  no per-row work for unloaded rows; no change to chunk loading. The dev perf overlay
  (`features/graph/perf.ts`, toggled as today) shows no worse draw time on the dev mock; record
  numbers before/after.
- `node scripts/contrast.mjs` passes (add a check if a new colour pair is introduced).
- Every new string exists in `en` and pt-BR (the type check enforces pt-BR parity).

## 5. Technical design

```
features/graph/a11y.ts        new, pure: visibleRange(scrollTop, viewH, rowH, offset, rowCount,
                              overscan, selected) → indices; rowText(meta, refs, glyphs, locale)
                              → cell texts; menuAnchor(rowIndex, rect, scrollTop, ...) → {x,y}
features/graph/a11y.test.ts   new: unit tests
features/graph/GraphView.tsx  render rows from visibleRange; activedescendant; A-2 keys
features/overlay.ts           A-4 helper (if trap chosen) + shared `isMenuKey(e)` for A-2
App.tsx, features/stash/*,    A-2/A-4/A-5 wiring
features/*/**Dialog/Menu*
styles.css                    sr-only rows container, reduced motion
docs/a11y/audit.md            A-8
```

`isMenuKey(e)`: `e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")` — move the
existing inline check in `SharedStashPanel.tsx` to use it.

## 6. Implementation steps

1. Audit first: walk every component in `app/ui/src` against A-1…A-7 and draft
   `docs/a11y/audit.md` with findings (before fixing).
2. TDD `a11y.ts` (`visibleRange`, `rowText`, `menuAnchor`, `isMenuKey`) with vitest.
3. A-1 in `GraphView.tsx`; check the dev mock in a browser (`pnpm --dir app/ui exec vite`):
   Tab to the graph, arrows move `aria-activedescendant`, rows exist only for the window.
4. A-2 graph, refs, local stash; A-3 check.
5. A-4 dialogs (one mechanism), A-5…A-7 fixes.
6. Strings in both tables; `pnpm --dir app/ui test`, `pnpm --dir app/ui build`,
   `node scripts/contrast.mjs`, `cargo build -p atlasgit --features custom-protocol`.
7. Finalize the audit report (status per finding, keyboard map, owner checks) and §11 notes.

## 7. Test plan

| Test | Kind |
|---|---|
| `visibleRange`: top, middle, end, WIP offset, selected outside window, rowCount 0, tiny viewport | unit |
| `rowText`: refs/glyphs/locale date, missing author | unit |
| `menuAnchor` clamps inside the viewport | unit |
| `isMenuKey` | unit |
| focus-trap helper index logic (if trap chosen) as a pure function over a list of focusables | unit |
| Browser walkthrough with the dev mock: keyboard-only path through the core flows (§1) | manual, recorded in the audit |
| Existing tests and the i18n parity tests still pass | regression |

## 8. Errors and edge cases

- Selected row not loaded yet → no activedescendant until loaded (never point to a missing id).
- Repo switch/tab close → ids must not collide between tabs (prefix with the repo id or keep one
  graph mounted; verify).
- Row count 0 (empty repo) → grid without rows, no activedescendant.
- Context menu key with nothing selected → no-op.
- Read-only repositories: menus still open; disabled items exposed as disabled.
- Dialog opened from a menu: focus returns to the original row, not to the closed menu.

## 9. Impact on existing architecture

UI only. `GraphView` keeps canvas rendering; the DOM rows are an accessibility mirror fed by the
same `rowAt` data. Overlay behaviour is centralized in `overlay.ts`. No Tauri/core change.

## 10. Definition of Done

- [ ] A-1…A-8 implemented or, for A-5…A-7 findings, fixed or recorded as deferred with reason.
- [ ] Unit tests in §7 pass; total UI test count ≥ previous; `pnpm build` passes.
- [ ] Keyboard-only walkthrough of the core flows recorded in `docs/a11y/audit.md`.
- [ ] Perf numbers before/after recorded; no regression.
- [ ] New strings in en and pt-BR; contrast check passes; Tauri crate builds.
- [ ] No file outside the ownership list changed; no new dependency.
- [ ] 5 screenshots in `docs/screenshots/phase5/accessibility/`: graph focused with the selected row and its hidden-row DOM/accessibility tree, commit menu opened with Shift+F10, ref or stash menu opened from the keyboard, modal dialog with focus inside, focus ring visible in the light theme.

## 11. Implementation notes

(Filled by the implementing agent.)

## 12. Instructions for the implementing agent

- Read `AGENTS.md`, `docs/HANDOFF.md`, `docs/specs/mvp-phase5.md`, `docs/specs/i18n-pt-br.md`
  (its §5 contract and §6 glossary) and this spec; read `GraphView.tsx`, `overlay.ts`,
  `SharedStashPanel.tsx` (reference keyboard pattern) and every menu/dialog before coding.
- Keep the visual design unchanged; prefer small, local changes; reuse `menuKeys`/`useOverlay`.
- Do not rename i18n keys or change existing English texts; add new keys to both tables.
- Commit in small steps; do not push. Report files changed, tests before/after, perf numbers,
  findings deferred, open questions.
