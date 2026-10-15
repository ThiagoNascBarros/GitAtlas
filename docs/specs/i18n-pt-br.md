---
status: draft
feature: Brazilian Portuguese UI (pt-BR) and locale selection
phase: 5 (module i18n-pt-br)
overview: docs/specs/mvp-phase5.md
date: 2026-10-15
---

# i18n — pt-BR translation and locale selection

## 1. Objective

The owner and the first users speak Brazilian Portuguese, and the roadmap ships pt-BR with the
MVP (HANDOFF "Phase 5", PRD NFR-9 "all UI strings in one place"). Today every string is English
and there is no way to choose a language. Result: the whole UI is available in English and
pt-BR, follows the system language by default, can be switched live from the toolbar, and dates
follow the chosen language.

## 2. Scope

**In scope**
- Locale model: `en` and `pt-BR`; choice `system` | `en` | `pt-BR`; live switching.
- Complete pt-BR table for every key of `en`.
- Toolbar language picker next to the theme picker.
- Locale-aware date/number formatting where the UI formats them today.
- `<html lang>` follows the active locale.
- UI text for error codes that have none (§4.4) in both locales; open-repo error routed through
  `errorText`.
- Unit tests for the locale logic and table parity.

**Out of scope**
- Translating Rust/core messages (`OpError.message` stays English; the UI maps codes to text).
- Translating Git's own output shown verbatim (e.g. `GIT_FAILED` details).
- Other languages, RTL, plural rules beyond the existing per-key approach.
- User docs in pt-BR (see `release-docs.md`), desktop/metainfo translations (done by `packaging`).
- Any new npm/Rust dependency (no i18n library).

## 3. Repository context

- `app/ui/src/i18n.ts`: `const en = {...} as const`, `type Key = keyof typeof en`, `t()`,
  `errorText()`, `errorFix()` (lookup of `err.<CODE>` / `err.<CODE>.fix`).
- 277 `t("…")` calls in TSX; strings already centralized (verified: no user-visible literal
  strings in TSX outside i18n besides comments).
- Theme picker pattern: `App.tsx` toolbar `<label class="theme-pick">` with an `sr-only` label and
  a `<select>`; `theme/theme.ts` holds a Solid signal + `setTheme`.
- Preferences persistence pattern: `localStorage` in try/catch with defaults
  (`features/split/Splitter.tsx`, `features/wip/WipPanel.tsx`, `features/shared-stash/model.ts`).
- Date formatting today: `App.tsx` (commit details `toLocaleString()`, status bar commit count
  `toLocaleString()`), `features/journal/JournalPanel.tsx` (`toLocaleString()`),
  `features/graph/GraphView.tsx` (canvas date column `toLocaleDateString()`),
  `features/shared-stash/logic.ts` (`relativeTime` fallback `toLocaleDateString()`,
  `formatBytes`). The perf overlay (dev only) is out of scope.
- `app/ui/index.html` has a fixed `lang`.

## 4. Requirements

### 4.1 Functional

- **FR-1 Locale resolution.** `resolveLocale(choice, systemLanguages)`: `en` / `pt-BR` choices
  return themselves; `system` returns `pt-BR` when the first entry of `navigator.languages`
  (fallback `navigator.language`) whose primary subtag is `pt` or `en` is `pt` (any region:
  `pt`, `pt-BR`, `pt-PT`); otherwise `en`. Pure function, unit-tested.
- **FR-2 Persistence.** Choice stored under `localStorage["ak.lang"]` (`system` | `en` |
  `pt-BR`), default `system`; unreadable/invalid value → `system`; storage errors ignored.
- **FR-3 Live switch.** The active locale is a Solid signal read inside `t()`, so every
  reactive text re-renders on change without reload. The canvas graph redraws (its date column
  and any canvas text) when the locale changes.
- **FR-4 Picker.** Toolbar `<select>` after the theme picker, same markup/styling pattern,
  accessible name `t("toolbar.language")`; options: follow system (`lang.system`), `English`,
  `Português (Brasil)` (language names written in their own language, same in both tables).
- **FR-5 Formatting.** Every date/number formatted for display passes the active locale
  (`toLocaleString(locale())`, `toLocaleDateString(locale())`, `Intl` with the locale). Machine
  values (`datetime=` ISO attributes) unchanged.
- **FR-6 `<html lang>`** is set to the active locale on start and on change.
- **FR-7 pt-BR table.** `app/ui/src/locales/pt-BR.ts` exports `ptBR` typed so that a missing or
  extra key is a compile error (e.g. `satisfies Record<Key, string>` with `import type { Key }`).
  `t`, `errorText`, `errorFix` look up the active table and fall back to `en` per key only as a
  defensive runtime path (the type check makes it unreachable).
- **FR-8 Missing error texts.** Add `err.<CODE>` (and `.fix` where a user action exists) in both
  tables for codes the core/shell can return but the UI does not know: `CONFLICT`,
  `INVALID_PATH`, `IO_ERROR`, `GIT_NOT_FOUND`, `NOT_A_REPO`, `READ_FAILED`, `UNKNOWN_REPO`.
  Verify the list against `crates/ak-core/src/**` (`OpError` constructors) and
  `app/src-tauri/src/**` (`UiError` codes) before adding; add any other code found missing.
  Wording must match the meaning in the Rust source.
- **FR-9** The open-repository error (`App.tsx`, `error.title` block) shows `errorText(err)`
  (+ `errorFix` when present) instead of the raw message.

### 4.2 Non-functional

- No new dependencies. Bundle size growth ≈ size of the pt-BR table only.
- No change to startup behaviour other than reading one localStorage key.
- Placeholders (`{n}`, `{s}`, `{v}`, `{path}` …) identical per key in both tables.
- `node scripts/contrast.mjs` still passes (picker uses existing tokens).

## 5. Technical design

```
app/ui/src/i18n.ts          en table (source), Key, locale signal, t/errorText/errorFix,
                            setLocaleChoice, localeChoice, activeLocale, resolveLocale, initLocale
app/ui/src/locales/pt-BR.ts ptBR table (import type { Key } from "../i18n")
app/ui/src/i18n.test.ts     new: resolution, parity, placeholders, fallback, persistence
app/ui/src/App.tsx          picker, initLocale() call next to initTheme(), FR-9
features/**                 FR-5 call sites only
```

Contract (exported from `i18n.ts`; other phases rely on it):

```ts
export type Locale = "en" | "pt-BR";
export type LocaleChoice = "system" | Locale;
export function resolveLocale(choice: LocaleChoice, langs: readonly string[]): Locale;
export const activeLocale: () => Locale;       // Solid accessor
export const localeChoice: () => LocaleChoice; // Solid accessor
export function setLocaleChoice(c: LocaleChoice): void; // persists + updates <html lang>
export function initLocale(): void;            // reads storage, applies
export function t(key: Key, vars?: Record<string, string | number>): string;
```

`initLocale` is called where `initTheme` is called today (find it in `main.tsx`/`App.tsx`).
Keep `en` in `i18n.ts` (no file move) to keep the diff small.

## 6. pt-BR glossary (owner review requested, non-blocking)

Git vocabulary as Brazilian developers use it: keep **commit, branch, tag, stash, merge,
rebase, cherry-pick, fetch, pull, push, HEAD, remote, upstream, hunk, SHA** as nouns in English;
verbs in UI actions are Portuguese imperatives built around them ("Fazer commit", "Fazer push",
"Criar branch", "Aplicar stash", "Fazer merge"). Translate everything else: "staged" →
"preparado(s)", "unstaged" → "não preparado(s)", "Stage" (action) → "Preparar", "Unstage" →
"Remover da preparação", "Discard" → "Descartar", "Undo/Redo" → "Desfazer/Refazer", "working
tree / working directory" → "diretório de trabalho", "conflict" → "conflito", "Shared stash" →
"Stash compartilhado", "Journal" → "Histórico de operações". Use "você", sentence case, no
trailing period on labels (match `en`). Keep keyboard key names (Esc, Enter, Shift+F10).

## 7. Implementation steps (TDD)

1. Write `i18n.test.ts` with failing tests for `resolveLocale` (pt, pt-BR, pt-PT, en-US, fr then
   pt, empty list), table parity (`Object.keys(en)` vs `ptBR`), placeholder parity per key,
   `t` switching after `setLocaleChoice`, invalid stored value → `system`.
2. Implement locale signal, resolution, persistence, `<html lang>` in `i18n.ts`.
3. Create `locales/pt-BR.ts` with all keys (follow §6); make the type check enforce parity.
4. FR-8 audit and new error keys in both tables (+ test that each listed code has `err.<CODE>`).
5. FR-9 open-repo error.
6. FR-5 call sites; make the graph redraw depend on the locale signal.
7. Toolbar picker (FR-4) and `initLocale()` wiring.
8. `pnpm --dir app/ui test && pnpm --dir app/ui build && node scripts/contrast.mjs`;
   `cargo build -p atlasgit --features custom-protocol` must still pass.
9. Browser check with the dev mock (`pnpm --dir app/ui exec vite`, `devMock.ts`): switch
   language, confirm texts and dates update without reload (screenshot optional).

## 8. Test plan

| Test | Kind | File |
|---|---|---|
| `resolveLocale` cases (FR-1) | unit | `i18n.test.ts` |
| key parity en ↔ pt-BR (FR-7), plus `tsc` | unit + type | `i18n.test.ts`, `pnpm build` |
| placeholder parity per key | unit | `i18n.test.ts` |
| `t`/`errorText`/`errorFix` follow the active locale | unit | `i18n.test.ts` |
| persistence: valid, invalid, throwing storage | unit | `i18n.test.ts` |
| every FR-8 code has `err.<CODE>` in both tables | unit | `i18n.test.ts` |
| existing tests (32) still pass; `relativeTime` tests adjusted only if output format changes | unit | existing |

## 9. Errors and edge cases

- `localStorage` throws or holds garbage → `system`, no crash.
- `navigator.languages` empty/undefined → `navigator.language` → `en`.
- A var missing in `t` keeps the `{name}` placeholder (current behaviour, keep).
- Long pt-BR strings: toolbar labels use `.lbl` which already collapses at narrow widths; check
  the toolbar at 900 px (min window width) does not overflow; shorten the pt-BR text if needed.
- Strings built by concatenation in TSX (e.g. graph live region) must still read correctly in
  pt-BR; prefer a keyed template over concatenation when touching them.

## 10. Impact on existing architecture

UI only. `t()` becomes reactive (reads a signal); callers do not change. Non-reactive callers
(canvas drawing, pure logic in `logic.ts`) keep working and re-run when their effect re-runs.
No Tauri, core or settings-file change (the language is a per-viewer preference, like panel
sizes).

## 11. Definition of Done

- [ ] FR-1…FR-9 implemented; §8 tests exist and pass; `pnpm --dir app/ui build` passes (type parity).
- [ ] Toolbar picker switches live; choice survives reload; `system` follows the WebView language.
- [ ] No user-visible English left when pt-BR is active, except Git terms from §6, Git/core raw
      output and the language name "English".
- [ ] `node scripts/contrast.mjs` and `cargo build -p atlasgit --features custom-protocol` pass.
- [ ] No files outside the ownership list in `mvp-phase5.md` §5 changed; no new dependency.
- [ ] 5 screenshots in `docs/screenshots/phase5/i18n-pt-br/` (picker, main window pt-BR, WIP pt-BR, dates pt-BR, error text pt-BR).
- [ ] §12 "Implementation notes" filled with decisions taken where the spec was silent.

## 12. Implementation notes

(Filled by the implementing agent.)

## 13. Instructions for the implementing agent

- Read `AGENTS.md`, `docs/HANDOFF.md`, `docs/specs/mvp-phase5.md` and this spec first; then read
  `i18n.ts`, `App.tsx` (toolbar), `theme/theme.ts`, `features/split/Splitter.tsx` before coding.
- Work only in the files listed in `mvp-phase5.md` §5 for `i18n-pt-br`.
- Do not rename existing keys or change English texts (other phases and docs quote them), except
  where FR-8/FR-9 require it.
- Commit in small steps with clear messages; do not push; report: files changed, test counts
  before/after, any deviation and why, open questions.
