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

Filled by the implementing agent (2026-10-16).

**Where things are**
- `i18n.ts`: `en` (now exported, for the parity test), `Locale`, `LocaleChoice`, `resolveLocale`,
  `activeLocale`, `localeChoice`, `setLocaleChoice`, `initLocale`, `t`, `errorText`, `errorFix`
  (contract of §5 unchanged). `initLocale` also listens to `languagechange` while the choice is
  `system`. Invalid stored values fall back to `system`; storage errors are ignored.
- `locales/pt-BR.ts`: `as const satisfies Record<Key, string>`, and `i18n.ts` assigns it to a
  `Record<Locale, Readonly<Record<Key, string>>>`. Checked: an extra key fails `tsc` (excess
  property), a missing key fails `tsc` (assignment); `i18n.test.ts` checks the same at runtime
  plus placeholder parity per key.
- `i18n.test.ts`: 16 tests (resolution, parity, placeholders, live switch, fallbacks,
  persistence with valid/invalid/throwing storage, `<html lang>`, locale-aware formatting).

**Decisions where the spec was silent**
1. `initLocale()` is called at the top of `App()` (before any text renders), not next to
   `initTheme()`: that call is in `main.tsx`, which is outside this module's file ownership.
2. FR-8 audit (`crates/ak-core/src/**`, `app/src-tauri/src/**`): `CONFLICT` is **not** an error
   code; it only appears as `contains("CONFLICT")` when parsing Git output (`ops/stash.rs`,
   `ops/shared_stash_ops.rs`), so no `err.CONFLICT` was added. Added: `INVALID_PATH`,
   `IO_ERROR` (+fix), `GIT_NOT_FOUND` (+fix), `NOT_A_REPO` (+fix, mentions `safe.directory`
   because `rev-parse` also fails on repositories owned by another user), `READ_FAILED`,
   `UNKNOWN_REPO` (+fix). No other code was missing. `UNKNOWN` (built by the UI for non-UiError
   failures) and `UNKNOWN_COMMAND` (dev mock) keep showing the raw message.
3. Core hints: the core sends an English `suggestion` with many codes, and `errorFix` shows it when
   the table has no `.fix` key, so pt-BR showed English hints. For the 16 codes whose suggestion
   is the same at every call site, an `err.<CODE>.fix` key was added with the **same English text**
   as the core (English output unchanged) and a pt-BR translation. Codes whose suggestion varies
   by call site keep the core's English hint as "core output": `REF_MOVED`, `READ_ONLY`,
   `NOTHING_TO_COMMIT`, `INVALID_TODO`, `INVALID_BRANCH_NAME`, `HEAD_MOVED`, `WORKTREE_CHANGED`,
   `NO_REMOTE`, `NO_UPSTREAM`, `GIT_FAILED`. Translating those needs a stable hint code from the
   core (contract change, not done).
4. FR-9 and the new codes: for `GIT_NOT_FOUND`, `NOT_A_REPO`, `READ_FAILED` and `IO_ERROR` the
   shell's raw message is the only diagnostic (no `detail`), so the UI shows it under "Details"
   (open-repo box: a `<details>`; error banner: the existing Details button). Other codes keep
   their behaviour.
5. Language picker: same markup as the theme picker (`label.theme-pick.lang-pick`, `sr-only`
   name, `Languages` icon). The select is as wide as "Português (Brasil)" in both locales, which
   squeezed the repository tabs (the "+" button was clipped at 1280 px in pt-BR, and at 900 px
   in both). Below 1400 px it shows the icon and the active language code (EN/PT) over a
   transparent select; the open list still shows the names; at 620 px it is icon-only like the
   theme picker. Measured: the tab crumb keeps its full width at 1280, 1100 and 900 px in both
   locales; the toolbar never scrolls.
6. Formatting: `formatBytes` uses `toLocaleString(locale, {min,max}FractionDigits)`, so en output
   is unchanged ("12.0 MB") and pt-BR gets "12,1 MB" (grouping separators now appear above 999).
   Toasts keep the language they were created in (they are plain strings, gone after ≤10 s).
   The perf overlay stays English (dev only, out of scope).
7. pt-BR wording beyond §6: "diff", "checkout", "reset", "fast-forward", "pop" (stash) and
   "amend" (in parentheses after "Corrigir o commit anterior") kept as Git terms; interactive
   rebase actions are Portuguese with the Git verb in parentheses ("Usar (pick)", "Juntar ao
   anterior (squash)"…); "Remote" panel → "Remotas", "Local" → "Locais"; theme names "Atlas
   Escuro"/"Atlas Claro"; "Dismiss" → "Dispensar"; "detached HEAD" → "HEAD destacado"; ours/theirs
   → "a nossa"/"a deles"; counts often equal to 1 use "(s)" ("1 preparado(s)").
8. Shortened for space (checked at 1280 px): the uncommitted-changes row is "Alterações locais"
   with chips "{n} a preparar" / "{n} preparado(s)" (longer texts made the canvas drop the staged
   chip); the sidebar tab is "Compartilhados" (the panel title keeps "Stash compartilhado").

**Verification**: `pnpm --dir app/ui test` 48 passed (baseline 32 + 16); `pnpm --dir app/ui
build`, `node scripts/contrast.mjs`, `cargo build -p atlasgit --features custom-protocol` pass.
Browser check with the dev mock (Playwright, 1280×800): live switch without reload, choice kept
after reload, `system` gives pt-BR for browser locales pt-BR/pt-PT and en for fr-FR/en-GB, the
canvas date column repaints on switch, no English UI text left in pt-BR across the main view,
commit/ref/stash menus, journal, new-branch, share dialog, shared-stash tab, rebase editor and
error banner (remaining English: fixture data, Git terms, Git's raw output). Screenshots:
`docs/screenshots/phase5/i18n-pt-br/`.

**Open questions for the owner**
- Glossary choices in item 7, especially "Remotas", "a preparar", "HEAD destacado", "Pop".
- The shared-stash glyph label "compartilhado" is twice as long as "shared" and truncates a
  branch label on the same row in the default refs column width ("mai" for "main"). Keep, or use
  a shorter word?

## 13. Instructions for the implementing agent

- Read `AGENTS.md`, `docs/HANDOFF.md`, `docs/specs/mvp-phase5.md` and this spec first; then read
  `i18n.ts`, `App.tsx` (toolbar), `theme/theme.ts`, `features/split/Splitter.tsx` before coding.
- Work only in the files listed in `mvp-phase5.md` §5 for `i18n-pt-br`.
- Do not rename existing keys or change English texts (other phases and docs quote them), except
  where FR-8/FR-9 require it.
- Commit in small steps with clear messages; do not push; report: files changed, test counts
  before/after, any deviation and why, open questions.
