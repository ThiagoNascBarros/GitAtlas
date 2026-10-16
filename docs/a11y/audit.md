---
status: draft
feature: Accessibility audit (Phase 5, module accessibility)
spec: docs/specs/accessibility.md
date: 2026-10-16
---

# AtlasGit accessibility audit (WCAG 2.1 AA)

Draft written before any fix (spec §6 step 1). Statuses are updated as fixes land.

## Method

- Code review of every component in `app/ui/src` against requirements A-1…A-7.
- Dev mock in Chromium (Playwright, 1280×800): ARIA snapshot of the main view and a Tab-order
  trace (70 presses from the graph).

## Findings (before fixes)

| Id | Area | Finding | WCAG |
|---|---|---|---|
| F-01 | Graph | `role="grid"` has no rows; screen readers only get a live region that repeats the selection | 1.3.1, 4.1.2 |
| F-02 | Graph | Commit context menu opens with the mouse only | 2.1.1 |
| F-03 | Refs panel | Branch/tag context menu opens with the mouse only | 2.1.1 |
| F-04 | Stashes | Local stash rows are not focusable; their menu (share) opens with the mouse only | 2.1.1 |
| F-05 | Dialogs | All dialogs are `<dialog open>` without `aria-modal`; Tab leaves the dialog and reaches the content behind | 2.4.3, 4.1.2 |
| F-06 | Dialogs | Journal, merge, conflict and rebase dialogs are named by `aria-label` while a visible heading exists | 1.3.1 |
| F-07 | Menus | Tab moves focus out of an open menu and leaves it open | 2.4.3 |
| F-08 | Toolbar | Repository tabs: `role="tab"` on a non-focusable `div` holding two buttons; the tablist also owns the "Open repository" button | 4.1.2 |
| F-09 | Many | Icons inside named buttons (toolbar, tab close, diff bar, rebase arrows, shared stash bar) are exposed as unnamed images | 1.1.1 |
| F-10 | Shared stash | "Verified" badge: `aria-label` on a generic `span` (prohibited, ignored by some AT) | 4.1.2 |
| F-11 | Refs panel | `summary` elements get the browser's default outline instead of the theme focus ring | 2.4.7 |
| F-12 | WIP, stashes | Focus falls to `body` when an operation re-renders the list that held the focused button (stage, unstage, stash apply/pop/drop) | 2.4.3 |
| F-13 | WIP | Repeated "Stage", "Discard", "Unstage", "Keep ours"… buttons have no file context | 2.4.6 |
| F-14 | Graph | Column resize handles are pointer-only | 2.1.1 |
| F-15 | Layout | Tab order is toolbar → sidebar → graph → diff → details (DOM order) | 2.4.3 |
| F-16 | Graph | The grid's name tells about arrow keys but not about the menu key | 3.3.2 |
| F-17 | Refs | Drag a ref onto the current branch to merge: keyboard alternative? | 2.1.1 |
| F-18 | Motion | `prefers-reduced-motion` already disables transitions/animations globally | 2.3.3 (AAA, info) |
| F-19 | Status | Toasts and the watcher banner are `<output>` (status); errors use `role="alert"` | 4.1.3 |
| F-20 | Forms | Commit summary/description, name dialog, journal keep count, rebase selects are labelled | 3.3.2 |
| F-21 | Contrast | `scripts/contrast.mjs` covers text and focus ring on surfaces; focus ring on the graph canvas and on overlays is not in the script | 1.4.3, 1.4.11 |
