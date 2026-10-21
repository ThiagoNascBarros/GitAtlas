---
status: draft
feature: Accessibility audit (Phase 5, module accessibility)
spec: docs/specs/accessibility.md
date: 2026-10-16
---

# Git Atlas accessibility audit (WCAG 2.1 AA)

Audit of the Git Atlas UI before the MVP release, with the fixes made in Phase 5. The first draft
(findings before any fix) is commit `c1d1c3b`; this version records the status of every finding.

## 1. Method

- **Code review** of every component in `app/ui/src` (toolbar, tabs, refs/stash/shared-stash
  sidebar, graph, details, WIP, diff, every menu and dialog) against spec requirements A-1…A-7.
- **Dev mock in Chromium** (`pnpm --dir app/ui exec vite`, IPC answered by `devMock.ts`,
  Playwright 1.56 + Chromium 141 at 1280×800): ARIA snapshot of each view
  (`locator.ariaSnapshot()`), a Tab-order trace (70 presses), scripted keyboard-only flows
  (section 5) and checks of `aria-activedescendant` / hidden rows while moving through the graph.
- **Contrast**: `node scripts/contrast.mjs` (text and focus ring on the theme surfaces) plus a
  one-off computation of the pairs the script does not cover (focus ring on canvas, overlay and
  selection; muted/secondary text on the selected graph row).
- **Not done here** (owner, section 7): a real screen reader (Orca) on the packaged app, and
  WebKitGTK-specific behaviour (the dev mock runs in Chromium).

## 2. WCAG 2.1 AA criteria checked

| Criterion | Result |
|---|---|
| 1.1.1 Non-text content | Pass after fixes: decorative icons are `aria-hidden` (F-09); the verified badge is a named `role="img"` (F-10); the canvas graph has a text alternative per row (F-01). |
| 1.3.1 Info and relationships | Pass after fixes: grid rows/cells mirror the canvas (F-01); dialogs are named by their visible heading (F-06); form fields are labelled (F-20). Graph column headers are not tied to the cells (F-23, deferred). |
| 1.4.3 Contrast (minimum) | Pass: `scripts/contrast.mjs` 0 failures. One extra failure found and fixed: SHA on the selected graph row in Atlas Dark (F-24). |
| 1.4.11 Non-text contrast | Pass: focus ring ≥ 4.3:1 on every surface in both themes (section 6); menu focus uses the accent fill (≥ 5.3:1). |
| 2.1.1 Keyboard | Pass after fixes: every context menu opens with Menu / Shift+F10 (F-02…F-04); drag-and-drop merge has the menu alternative (F-17). Graph column resize is pointer-only (F-14, deferred, also out of scope in the spec). |
| 2.1.2 No keyboard trap | Pass: dialogs and menus keep Tab inside on purpose and always close with Escape (and Cancel); focus then returns to where it was. |
| 2.4.3 Focus order | Pass after fixes: dialogs are modal for Tab (F-05), menus keep Tab (F-07), focus survives list re-renders (F-12). Order between panes follows the layout (F-15). |
| 2.4.7 Focus visible | Pass after fixes: `summary` (F-11) and the graph (F-22, its ring was hidden under the canvas). |
| 3.3.2 Labels or instructions | Pass: all fields labelled (F-20); the graph describes its menu key (F-16). |
| 4.1.2 Name, role, value | Pass after fixes: repository tabs (F-08), modal dialogs (F-05), grid rows with `aria-selected` and `aria-activedescendant` (F-01), `aria-haspopup="menu"` on rows that open a menu. |
| 4.1.3 Status messages | Pass: toasts and the watcher banner are `<output>` (status), errors `role="alert"`; the graph no longer repeats the selection in a live region (focus announces it). |

## 3. Findings per UI area

Status: **pass** (no change needed), **fixed** (commit), **deferred** (reason).

| Id | Area | Finding | WCAG | Status |
|---|---|---|---|---|
| F-01 | Graph | `role="grid"` had no rows; a live region repeated the selection | 1.3.1, 4.1.2 | fixed `79040e8`: hidden `role="row"` per loaded row in the viewport ± 10 (+ selected row + uncommitted-changes row), `aria-rowindex`, `aria-selected`, 5 gridcells, `aria-activedescendant`, `aria-rowcount`/`aria-colcount`/`aria-multiselectable`; live region removed |
| F-02 | Graph | Commit menu opened with the mouse only | 2.1.1 | fixed `79040e8` (Menu / Shift+F10 on the selected commit; nothing on the uncommitted-changes row) |
| F-03 | Refs panel | Branch/tag menu opened with the mouse only | 2.1.1 | fixed `33ab345` |
| F-04 | Stashes | Local stash rows not focusable; menu mouse-only | 2.1.1 | fixed `33ab345` (the message is a focusable `role="button"` with `aria-haspopup`; Enter, Menu or Shift+F10 opens the menu; no visual change) |
| F-05 | Dialogs | `<dialog open>` without `aria-modal`; Tab reached the page behind | 2.4.3, 4.1.2 | fixed `33ab345`: all 8 dialogs (name, journal, merge, share, update, unshare, conflict, rebase) have `aria-modal="true"` and a Tab trap |
| F-06 | Dialogs | Named by `aria-label` although a heading exists | 1.3.1 | fixed `33ab345` (`aria-labelledby`; the name dialog points at its visible label) |
| F-07 | Menus | Tab left an open menu | 2.4.3 | fixed `33ab345` (Tab cycles in the menu; Escape closes) |
| F-08 | Toolbar | Repository tabs: `role="tab"` on a non-focusable `div` holding two buttons | 4.1.2 | fixed `33ab345` (a `nav` of buttons, the active one `aria-current="page"`) |
| F-09 | Many | Icons inside named buttons exposed as unnamed images | 1.1.1 | fixed `33ab345` |
| F-10 | Shared stash | `aria-label` on a generic `span` (verified badge) | 4.1.2 | fixed `33ab345` (`role="img"`) |
| F-11 | Refs panel | `summary` used the browser outline, not the theme ring | 2.4.7 | fixed `79040e8` |
| F-12 | WIP, stashes, refs | Focus fell to the page when an operation re-rendered the list holding it | 2.4.3 | fixed `33ab345`, `6477e33`: same item, else its neighbour, else the section's first control, else the graph; the graph no longer steals focus on reload from a control that still exists |
| F-13 | WIP, stashes | Repeated "Stage", "Discard", "Apply", "Keep ours"… without context | 2.4.6 | fixed `33ab345` (`aria-describedby` → the file path / stash message) |
| F-14 | Graph | Column resize handles are pointer-only | 2.1.1 | deferred: layout convenience, out of scope (spec §2); widths persist and the columns adapt to the window |
| F-15 | Layout | Tab order is toolbar → sidebar → graph → diff → details/WIP | 2.4.3 | pass (deferred change): the diff lives in the centre pane under the graph, so DOM order follows the visual reading order; putting details before the diff needs moving the diff out of the centre pane (layout change) |
| F-16 | Graph | The grid's name mentions arrows only | 3.3.2 | fixed `79040e8` (`aria-describedby` "Shift+F10 or the Menu key opens the actions of the selected commit.") |
| F-17 | Refs | Drag a ref onto the current branch to merge | 2.1.1 | pass: the ref menu offers "Merge into current branch…" for exactly the draggable refs (not tags, not the current branch), and opens the same dialog with the conflict preview (spec A-3) |
| F-18 | Motion | Transitions and animations | 2.3.3 | pass: `prefers-reduced-motion: reduce` already disables them globally (`styles.css`), the spinner too; no JS animation or smooth scrolling |
| F-19 | Status | Toasts, banners, errors | 4.1.3 | pass |
| F-20 | Forms | Commit summary/description, name dialog, journal keep count, share/update dialogs, rebase selects | 3.3.2 | pass |
| F-21 | Contrast | Script does not cover ring on canvas/overlay/selection | 1.4.11 | pass (computed, section 6) |
| F-22 | Graph | Focus ring drawn on the scroller was hidden under the sticky canvas | 2.4.7 | fixed `d79f28c` (ring drawn on the canvas box) |
| F-23 | Graph | Header labels (Branch / tag, Message…) are text outside the grid | 1.3.1 | deferred: rows read their cells in a fixed order; adding a header row changes row indexes for little gain; revisit after Orca testing |
| F-24 | Graph | SHA (muted) on the selected row: 3.86:1 in Atlas Dark | 1.4.3 | fixed `5d122bb` (secondary text on the selected row only) |
| F-25 | Dialogs | Merge dialog opened by drag-and-drop, conflict and rebase editors (while loading) had no initial focus | 2.4.3 | fixed `33ab345` (Cancel; the title holds focus while loading, then Cancel) |
| F-26 | Graph / WebKitGTK | Keyboard-raised `contextmenu` event would pick the row under the computed point | 2.1.1 | fixed `79040e8` (a `contextmenu` without the right button opens the selected commit's menu) |

## 4. Keyboard map

| Where | Keys | Action |
|---|---|---|
| Anywhere | Ctrl+Z / Ctrl+Shift+Z | Undo / Redo the last operation (not while typing in a field) |
| Anywhere | Ctrl+Shift+P | Toggle the performance overlay (developer aid) |
| Anywhere | Tab / Shift+Tab | Move between controls: toolbar → sidebar → graph → diff (when open) → details/WIP |
| Graph | ↑ / ↓, Page Up / Page Down, Home / End | Select the previous/next commit, a page up/down, the first/last commit |
| Graph | ↑ on the first commit, or Home | Select the uncommitted-changes row (opens the commit panel) |
| Graph | Menu key or Shift+F10 | Open the selected commit's menu (cherry-pick, revert, reset, branch, tag, rebase, checkout, copy SHA) |
| Refs panel | Tab to a branch/tag, Enter | Select its commit in the graph |
| Refs panel | Menu key or Shift+F10 | Open the branch/tag menu (check out, merge into current branch, rebase, new branch, rename, delete, push/delete tag) — the keyboard alternative to double-click checkout and drag-to-merge |
| Stashes | Tab to the stash message, then Enter, Menu key or Shift+F10 | Open the stash menu (share, apply, pop); Apply / Pop / Drop buttons follow |
| Shared stash | ↑ / ↓, Home / End | Move between shared stashes |
| Shared stash | Enter / Menu key or Shift+F10 | Show its changes / open its menu |
| Menus | ↑ / ↓, Home / End, Tab / Shift+Tab | Move between items (wraps) |
| Menus | Enter or Space / Escape | Run the item / close (focus returns to the row) |
| Dialogs | Tab / Shift+Tab | Cycle through the dialog's controls (the page behind is not reachable) |
| Dialogs | Enter / Escape | Submit (forms) / cancel; focus returns to where the dialog was opened from |
| Commit form | Ctrl+Enter in the description | Commit |
| Splitters | ← / → (or ↑ / ↓), Home / End | Resize the pane by 16 px / to its minimum / maximum |
| Diff | Escape | Leave full screen |
| Sidebar, files, diff mode | Enter / Space on toggle buttons | Switch view (state exposed with `aria-pressed` / `aria-expanded`) |

## 5. Keyboard-only walkthrough (dev mock, Chromium)

Done with Playwright pressing keys only (no mouse after the page loaded), on the `demo` fixture:

| Core flow | Keys | Result |
|---|---|---|
| Open repository | Tab to "+" (Open repository…), Enter | Second tab opens and is current (`aria-current`) |
| Browse history | ↓ ×3 in the graph | Details panel shows the commit; `aria-activedescendant` follows |
| Inspect a commit | Tab to a changed file, Enter; Shift+Tab | Diff opens; its controls are reachable |
| Stage / commit | Home in the graph, Tab to "Stage all", Enter, Tab to Summary, type, Tab, Ctrl+Enter | Staged (focus stays in the section), "Committed: …" toast |
| Branch | Tab to "New branch", Enter, type, Enter | "Created branch …" |
| Merge | Tab to a branch, Menu key, ↓ to "Merge into current branch…", Enter, Tab to "Merge", Enter | "Merged …" |
| Stash | Tab to "Stash", Enter, Enter; Tab to the stash, Tab to Apply, Enter | "Changes stashed", "Stash applied" |
| Fetch / pull / push | Tab to Fetch, Enter | "Fetched" |
| Shared stash | Enter on the sidebar tab, Tab to a row, Menu key, Enter | "Shared stash applied" |
| Commit menu | Shift+F10 on a selected commit, ↓, Enter on "New branch here…", Escape | Name dialog, then focus back on the graph |

## 6. Performance and contrast numbers

Dev mock, synthetic 2,000-commit fixture (200 loaded rows), headless Chromium, 3 runs of 4 s
scrolling 3 rows per frame; perf overlay (Ctrl+Shift+P) sampled every frame:

| | Before (`e443740`) | After |
|---|---|---|
| Canvas draw, median / p95 (overlay "draw") | 1.2 / 1.7–2.2 ms | 1.2–1.3 / 2.0–2.5 ms |
| Painted frames/s (headless caps at ~30) | 33–36 | 30–36 |
| First paint | 101–104 ms | 101–109 ms |
| Hidden rows in the DOM | 0 | 45–46 (viewport 25 + 2×10 + uncommitted row), of 2,001 |
| One scroll update incl. mirror, median / p95 | 0.8 / 1.3 ms | 1.4 / 2.5 ms |

The mirror costs ~0.6 ms per scroll step, well inside a 16 ms frame; the canvas draw is unchanged.

Focus ring contrast (computed from the theme tokens):

| Theme | ring | canvas | surface | raised | overlay | selected |
|---|---|---|---|---|---|---|
| Atlas Dark | #8DB8F2 | 8.83 | 8.36 | 7.68 | 7.09 | 6.48 |
| Atlas Light | #2563C9 | 4.97 | 5.35 | 5.67 | 5.67 | 4.34 |

## 7. Owner checks (manual, not possible here)

1. **Orca on the packaged app** (deb/AppImage/Flatpak, WebKitGTK): Tab into the graph, move with
   the arrows and listen for each commit ("refs, message, author, date, SHA"; "row N of M,
   selected"); the uncommitted-changes row; Shift+F10 / Menu key on the graph, a branch, a stash
   and a shared stash; each dialog announces its title and keeps Tab inside.
2. **WebKitGTK menu key**: Shift+F10 and the Menu key open exactly one in-app menu (no WebKit
   context menu on top) on the graph, ref rows and stash rows.
3. **Focus after operations** on a real repository: stage/unstage by keyboard, stash apply/pop/drop,
   branch checkout from the menu, commit — focus never ends on the page body.
4. **Themes**: focus ring visible on toolbar, sidebar, graph, dialogs in Atlas Dark and Atlas Light;
   with "Reduce animations" enabled in GNOME, no transitions.
5. **Screen magnifier** (GNOME zoom with focus tracking): it follows the selected graph row (the
   hidden rows sit where the canvas draws them).
