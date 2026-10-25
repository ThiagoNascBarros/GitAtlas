# Implementation Plan: Accessibility

**Spec**: `specs/003-accessibility/spec.md` | **Source**: `docs/specs/accessibility.md` | **Depends on**: `002-i18n-pt-br`

## Summary
Audit first, then pure helpers (`features/graph/a11y.ts`), hidden row mirror in `GraphView.tsx`, keyboard menus, and a Tab trap via `useOverlay`/`keepFocus` in `features/overlay.ts`.

## Deviation
Focus trap instead of `HTMLDialogElement.showModal()`: the top layer breaks the layout and makes banners/toasts inert.

## Deferred
F-14 (column resize pointer-only), F-15 (pane order), F-23 (graph headers outside the grid).
