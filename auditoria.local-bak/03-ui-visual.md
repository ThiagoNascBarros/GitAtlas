# Audit 03 — Visual UI, accessibility and i18n

## Limitações
- Static analysis (code, CSS, theme JSON, docs) plus numeric contrast computations. No screenshots were taken and the UI was not run in this pass; focus order, overflow and visual rendering were not observed live.
- Contrast ratios were computed with the same WCAG formula as `scripts/contrast.mjs` (extra throw-away script in the scratchpad, nothing written to the repo).
- Screen reader (Orca) and WebKitGTK behaviour not tested (the project's own audit says the same).
- Matrix IDs from `auditoria/00-matriz-documentacao-vs-codigo.md`.

## Findings

### [UIV-1] Theme choice is not persisted
- Severidade: MÉDIA
- Item da matriz: F-028
- Local: app/ui/src/theme/theme.ts:19-20,46-50; app/ui/src/App.tsx:350-352
- Evidência: `const [choice, setChoice] = createSignal<ThemeChoice>("system");` and `initTheme()` always calls `setTheme("system")`. No `localStorage` in theme.ts, while the language does persist (`i18n.ts:391 STORAGE_KEY = "ak.lang"`).
- Por que importa: A user who picks Atlas Dark/Light on a system set to the other must re-pick on every launch; inconsistent with the language picker.
- Sugestão de correção: Store the choice under `ak.theme` (try/catch like i18n.ts) and read it in `initTheme()`.
- Esforço estimado: P

### [UIV-2] Shipped palette does not match design-tokens.md
- Severidade: MÉDIA
- Item da matriz: F-028
- Local: themes/atlas-dark.theme.json, themes/atlas-light.theme.json vs Obsidian `03-design/design-tokens.md` §3–4
- Evidência: doc: dark canvas `#0E1116`, surface `#151A21`, accent `#F2A93B` (amber), focus ring `#6CB6FF`. Code: canvas `#17151F`, surface `#1D1A27`, accent `#A78BFA` (violet), ring `#8DB8F2`; light accent `#6D3FD1` vs doc `#9A5B00`. Lane colors, status and diff colors do match. Token key sets of both themes are identical (parity OK).
- Por que importa: The spec'd identity ("slate neutrals + Atlas Amber") is not what ships; any doc/screenshot consumer is misled, and lane.0 amber no longer equals the accent.
- Sugestão de correção: Either update design-tokens.md (and record the decision) or realign the themes; re-run contrast.mjs.
- Esforço estimado: P

### [UIV-3] No high-contrast themes or forced-colors support
- Severidade: MÉDIA
- Item da matriz: NF-008
- Local: themes/ (2 files); app/ui/src/styles.css:448; contrast.mjs:35
- Evidência: design-tokens.md §8 defines kinds `high-contrast-dark|light` (7:1 text); `ls themes` shows only `atlas-dark` and `atlas-light`. CSS has `prefers-contrast: more` that only swaps `--lg-edge`; `grep forced-colors styles.css` finds nothing. The script's 7:1 branch (`hc`) is never exercised.
- Por que importa: Low-vision users on Linux/GNOME high-contrast get no support; the tested 7:1 path is dead code.
- Sugestão de correção: Add two high-contrast themes and a `forced-colors` block, or document the descope.
- Esforço estimado: M

### [UIV-4] Contrast gaps not covered by contrast.mjs
- Severidade: MÉDIA
- Item da matriz: NF-008
- Local: scripts/contrast.mjs:6-12; styles.css:93,134,261 (selected rows), :.st-* status letters
- Evidência: computed ratios not in PAIRS. Atlas Light, status color on `bg.selected`: added 3.85, modified 4.25, deleted 4.23, renamed 4.39 (all < 4.5); status letters `.st-A/M/D/R` render on `.files button.active` (bg-selected). Atlas Dark: `text.muted` on `bg.selected` 3.86, on `bg.overlay` 4.22 (< 4.5). Light lane.1 on `bg.selected` 3.55 (≥3, OK for graphics). Focus ring is fine: dark 8.83/6.48/7.09 (canvas/selected/overlay), light 4.97/4.34/5.67.
- Por que importa: The audit claims 1.4.3 "pass", but the script only tests the surface/canvas backgrounds; the SHA fix (F-24) addressed one instance only.
- Sugestão de correção: Add selected and overlay backgrounds to PAIRS; darken light status colors or use the secondary text color on selected rows.
- Esforço estimado: P

### [UIV-5] Input borders use border.default (1.3:1)
- Severidade: MÉDIA
- Item da matriz: NF-008
- Local: styles.css:42,48 (`input`, `select`), themes `color.border.default`
- Evidência: `border.default` on `bg.surface`: dark `#332E40`/`#1D1A27` = 1.31, light 1.37. The script checks `border.strong` ≥3 only. Fields sit on `bg.raised` over a surface.
- Por que importa: WCAG 1.4.11 requires 3:1 for the boundary identifying a control; audit row 1.4.11 reports "Pass" without these.
- Sugestão de correção: Use `border-strong` for inputs/selects (as `.seg` already does) or add a fill that reaches 3:1.
- Esforço estimado: P

### [UIV-6] Fonts differ from tokens and none is bundled
- Severidade: BAIXA
- Item da matriz: fora da documentação
- Local: styles.css:5-6 and 201-202; app/ui/public (only favicon.png, mock)
- Evidência: first `:root` declares Inter/JetBrains Mono (design-tokens §6 "bundled Inter"); the later override uses `"Ubuntu Sans", Ubuntu, Cantarell`. `grep ttf|woff` in app/ui finds no font files.
- Por que importa: Rendering depends on system fonts; glyph widths (canvas `clipText` in the graph) vary between machines; the first declaration is dead.
- Sugestão de correção: Pick one stack, delete the dead declaration, bundle or document the dependency.
- Esforço estimado: P

### [UIV-7] Raw error text shown in graph; hard-coded English in perf overlay
- Severidade: MÉDIA
- Item da matriz: UI-006, NF-010
- Local: app/ui/src/features/graph/GraphView.tsx:99,517,520-523
- Evidência: `setLoadError(String(e?.message ?? e));` rendered as `<div class="graph-error" role="alert">{loadError()}</div>` (bypasses `errorText()`/`err.<CODE>`). Overlay (Ctrl+Shift+P) has literals `first paint`, `draw`, `painted frames/s`, `commits · lanes`, and `rowCount.toLocaleString()` without locale. Elsewhere dates/numbers use `activeLocale()` correctly.
- Por que importa: A failing chunk shows untranslated engine text to the user; UI-006 promises human messages with stable codes.
- Sugestão de correção: Run `e` through `errorText()`; i18n-ify (or explicitly exempt) the dev overlay.
- Esforço estimado: P

### [UIV-8] Deferred keyboard gaps: graph column resize, WIP list
- Severidade: MÉDIA
- Item da matriz: NF-008, UI-007
- Local: docs/a11y/audit.md F-14, F-23; features/wip/WipPanel.tsx
- Evidência: audit: "F-14 … Column resize handles are pointer-only … deferred", "F-23 … Header labels … text outside the grid … deferred"; matrix UI-007 (from HANDOFF): no keyboard nav in WIP. Splitters do have keyboard support (audit §4).
- Por que importa: 2.1.1 is "Pass after fixes" in the summary table but F-14 is a known violation; F-15 (tab order toolbar→sidebar→graph→diff→details) marked pass although it differs from some reading orders.
- Sugestão de correção: Arrow-key resize on the header handles (reuse Splitter logic); add roving focus to WIP rows; add header row semantics after Orca testing.
- Esforço estimado: M

### [UIV-9] Accessibility verified only in Chromium; target is WebKitGTK
- Severidade: MÉDIA
- Item da matriz: NF-008
- Local: docs/a11y/audit.md §1, §7
- Evidência: "Not done here: a real screen reader (Orca) on the packaged app, and WebKitGTK-specific behaviour (the dev mock runs in Chromium)". Audit targets WCAG 2.1 AA while PRD NFR-7 / contrast.mjs header cite 2.2.
- Por que importa: Accessible name/role mapping and `aria-activedescendant` on a canvas grid are the riskiest parts and are unconfirmed on the real engine.
- Sugestão de correção: Run an Orca pass on the AppImage; add 2.2 criteria (2.4.11 focus not obscured, 2.5.8 target size — controls are `--ctl-h: 28px`, icon buttons 24px at styles.css:30).
- Esforço estimado: M

### [UIV-10] Fixed px sizing: no font-scale support
- Severidade: BAIXA
- Item da matriz: NF-009
- Local: styles.css:7,202 (`font-size: 13px`), many `font-size: 11/12px`; canvas row height fixed
- Evidência: `:root { font-size: 13px; }` and px units throughout; matrix NF-009 notes "font scaling not handled".
- Por que importa: GNOME text-scaling / zoom ignores or breaks fixed-height canvas rows.
- Sugestão de correção: Use rem for text and derive graph row height from the root size.
- Esforço estimado: M

### [UIV-11] pt-BR labels up to 3.5x longer than English
- Severidade: BAIXA
- Item da matriz: NF-010
- Local: app/ui/src/locales/pt-BR.ts
- Evidência: keys parity: 374 en / 374 pt-BR, no missing or extra. 62 keys are >1.4x longer, e.g. `rebase.reword` 6→28, `wip.unstage` 7→21, `ss.unshare.submit` 7→21, `ss.tabs` 12→29, `wip.unstageAll` 11→26. Toolbar has `overflow-x: auto` (styles.css:34); `.tool` is `white-space: nowrap`.
- Por que importa: Buttons and tabs in narrow sidebar/WIP can clip or force horizontal scroll; not visually tested.
- Sugestão de correção: Screenshot pt-BR at 1024 px width for toolbar, WIP, shared-stash tabs and rebase editor; add `title`/ellipsis where needed.
- Esforço estimado: P

## O que já está bom
- Key parity en/pt-BR is complete (374/374), enforced by `satisfies` + `i18n.test.ts`; no literal `aria-label=`, `title=`, `placeholder=` strings in TSX (grep clean); dates/numbers use `activeLocale()`; `html lang` follows the locale.
- Components use `--ak-*` tokens; the only literal colours in CSS are black shadows/scrims (`rgb(0 0 0 / .x)`) and none in TSX; the graph canvas reads colors from the theme.
- `node scripts/contrast.mjs`: atlas-dark, atlas-light, 0 failures; lane colors ≥ 3:1 vs canvas; focus ring ≥ 4.3:1 on all surfaces.
- Strong a11y baseline: visible `:focus-visible` rings, modal Tab traps, menus on Menu/Shift+F10, `prefers-reduced-motion` and `prefers-reduced-transparency` honoured, lucide icons `aria-hidden`.

## Não verificado
- Live rendering, overflow in pt-BR, focus order in the running app; Orca; WebKitGTK; whether `text.muted` is actually used on selected rows besides the graph SHA; docs/screenshots/phase5 content; docs/specs/accessibility.md beyond the audit summary.
