# Implementation Plan: pt-BR and locale selection

**Spec**: `specs/002-i18n-pt-br/spec.md` | **Source**: `docs/specs/i18n-pt-br.md` | **Overview**: `docs/specs/mvp-phase5.md`

## Summary
Locale signal and resolution in `app/ui/src/i18n.ts`; pt-BR table in `app/ui/src/locales/pt-BR.ts` typed against `Key`; toolbar picker; locale-aware formatting. No new dependencies. TDD (`i18n.test.ts`).

## Structure
`app/ui/src/{i18n.ts,locales/pt-BR.ts,App.tsx}`, `features/graph/GraphView.tsx` (redraw on locale), `app/ui/index.html`.

## Deviations (spec §12)
`initLocale()` runs in `App()`; 16 core hints duplicated as `err.<CODE>.fix` keys; 10 codes with variable hints keep the core's English hint (needs a hint code from the core).
