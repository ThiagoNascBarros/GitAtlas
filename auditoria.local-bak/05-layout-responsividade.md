# 05 - Layout and responsiveness

## Limitations
- Code-reading audit only: App.tsx, styles.css, features/split/Splitter.tsx, features/graph/columns.ts, GraphView.tsx, tauri.conf.json, docs/a11y/audit.md, REA PRD and `03-design/layout-direction.md`.
- I did not run the vite/devMock UI nor resize the Browser pane: no screenshots exist; runtime effects are inferred from CSS/JS.
- WebKitGTK behaviour (system font scale, per-monitor DPR, Wayland fractional scaling) was not tested.

### [LAY-1] Pane size limits are static and stored sizes are not clamped
- Severidade: ALTA
- Item da matriz: UI-003
- Local: app/ui/src/App.tsx:468,511; features/split/Splitter.tsx:11; styles.css:329,359
- Evidência: `min={140} max={420}` (left), `min={240} max={600}` (right); `storedSize` accepts any stored value `saved > 0 ? saved : initial`. tauri.conf.json `minWidth: 900`. Left max + right max = 1020 px > 900 px. `.body` is `var(--left-w) 0 minmax(0, 1fr) 0 var(--right-w)`; the width cap exists only at `@media (max-width: 860px)`, unreachable with minWidth 900.
- Por que importa: at the minimum window, or with large saved sizes (or an edited value such as 5000), the centre pane with graph and diff can collapse to 0 px.
- Sugestão de correção: clamp on load and on window resize against the live width; reserve a centre minimum (`minmax(320px, 1fr)`).
- Esforço estimado: P

### [LAY-2] Responsive breakpoints below the minimum window size are dead code
- Severidade: MÉDIA
- Item da matriz: UI-003
- Local: app/ui/src/styles.css:305-316,359-363; src-tauri/tauri.conf.json (minWidth 900, minHeight 600)
- Evidência: `@media (max-width: 860px)` and `(max-width: 620px)` (stack, hide refs, hide splitters) are below `minWidth: 900`; only the 1100 and 1400 px rules can fire. At 900 px the default 232 + 360 px side panes leave about 308 px for the centre.
- Por que importa: the documented "below 620 px refs pane hidden" (layout-direction.md:86) never occurs in the app, while the real minimum (900 px) is untuned.
- Sugestão de correção: add a breakpoint at 900-1000 px (narrower panes) or lower minWidth consistently; document or remove unreachable rules.
- Esforço estimado: P

### [LAY-3] No UI zoom (80-200 %) and no system font scaling
- Severidade: ALTA
- Item da matriz: NF-009, NF-008 (PRD item 11 zoom 80-200 %; NFR-8)
- Local: app/ui/src (no occurrences); GraphView.tsx:150; styles.css:25,85
- Evidência: grep for zoom/font-scale in app/ui/src finds nothing. Fixed px: `ctx.font = "13px Inter, Cantarell, system-ui, sans-serif"`, `.graph-rows [role="row"] { height: 28px }`, `grid-template-rows: 34px 44px`. Matrix NF-009 already says "font scaling not handled". Whether WebKitGTK applies the system text scale to px sizes: Not verified.
- Por que importa: the PRD zoom and font-scaling requirements are unmet; low-vision users cannot enlarge the UI, and fixed canvas row height would not follow even if CSS did.
- Sugestão de correção: add a persisted UI zoom (webview zoom or a root `--ui-scale` driving sizes, ROW_H and canvas font) with Ctrl+= / Ctrl+- / Ctrl+0.
- Esforço estimado: G

### [LAY-4] Graph canvas does not redraw when devicePixelRatio changes
- Severidade: MÉDIA
- Item da matriz: NF-002, NF-009
- Local: app/ui/src/features/graph/GraphView.tsx:122,399
- Evidência: `const dpr = window.devicePixelRatio || 1;` is read only in `draw()`; the only size trigger is `new ResizeObserver(() => setSize({ w: scroller.clientWidth, h: scroller.clientHeight }))`. No `matchMedia("(resolution: ...)")` listener (grep: matchMedia only in theme/theme.ts for colour scheme). Fractional DPR itself is handled: `Math.round(w * dpr)` plus `setTransform(dpr, ...)`.
- Por que importa: moving the window between monitors of different scale, or changing scale, without a CSS size change can leave the canvas blurry until the next scroll or resize. Not reproduced (Not verified).
- Sugestão de correção: listen to a re-armed `matchMedia` resolution query and bump the redraw signal.
- Esforço estimado: P

### [LAY-5] Splitters: no collapse, no reset, no pointer cancel handling
- Severidade: MÉDIA
- Item da matriz: UI-003, NF-008
- Local: app/ui/src/features/split/Splitter.tsx:33-60; styles.css:361
- Evidência: keyboard arrows 16 px and Home/End to min/max exist, with `role="separator"` and aria-value*. Home goes to `min` (140/240 px), never 0; no double-click reset; drag adds `window` listeners for `pointermove`/`pointerup` only (no `pointercancel`, no pointer capture); `.splitter-x { display: none }` at <=620 px.
- Por que importa: panes cannot be hidden to give the graph full width; a lost `pointerup` (focus loss) leaves the `resizing-x` body class and listeners stuck. Collapse itself is beyond layout-direction.md:69.
- Sugestão de correção: add pointer capture and `pointercancel`; double-click to reset default; optional persisted collapse.
- Esforço estimado: M

### [LAY-6] Graph column resize is pointer-only; auto-hidden columns have no control
- Severidade: MÉDIA
- Item da matriz: F-14, F-23
- Local: app/ui/src/features/graph/GraphView.tsx:64-76,430-436; features/graph/columns.ts:17-27; docs/a11y/audit.md:35,62,71
- Evidência: `<span class="col-resize" ... onPointerDown={(e) => startResize(e, "refs")} />` with no tabindex/role/key handler. Audit marks F-14 and F-23 deferred. Widths persist (`ak.graph.columns.v1`, clamps 60-480/320/200); author, then date, auto-hide when the message would be under `MIN_MESSAGE_W = 160`.
- Por que importa: keyboard-only users cannot resize (WCAG 2.1.1, known deferral); header text is outside the grid (1.3.1); hidden author/date cannot be restored by the user at narrow widths.
- Sugestão de correção: make handles `role="separator" tabindex="0"` reusing Splitter's key logic; add column visibility toggles.
- Esforço estimado: M

### [LAY-7] Diff splitter max is read non-reactively and differs from the CSS cap
- Severidade: BAIXA
- Item da matriz: UI-003
- Local: app/ui/src/App.tsx:485; styles.css:343
- Evidência: `max={Math.max(160, innerHeight - 260)}` versus CSS `.center > .diff { max-height: calc(100% - 120px) }`.
- Por que importa: aria-valuemax goes stale after a window resize and End may set a value that CSS clips.
- Sugestão de correção: derive the max from the `.center` height via ResizeObserver and share the bound with CSS.
- Esforço estimado: P

### [LAY-8] Canvas font and bar height disagree with the layout direction
- Severidade: BAIXA
- Item da matriz: fora da documentação
- Local: app/ui/src/features/graph/GraphView.tsx:150; styles.css:392; REA layout-direction.md:55-57
- Evidência: doc says Ubuntu Sans UI font and top bar 48 px; code draws canvas text with `Inter, Cantarell, system-ui` hardcoded and the final `.app` row is `56px`.
- Por que importa: graph text and the rest of the UI can diverge in font if the CSS token changes; docs and code disagree.
- Sugestão de correção: read the canvas font from a CSS token; update the doc or the CSS.
- Esforço estimado: P

## O que já está bom
- Splitter has `role="separator"`, aria-value*, keyboard arrows/Home/End, and sizes persist with try/catch fallbacks (Splitter.tsx).
- Long names use ellipsis (styles.css:36,66,136,347,505) and the toolbar scrolls (`overflow-x: auto`) with labels shed at 1100/860 px.
- Graph columns adapt to width with a 160 px message minimum and persisted clamped widths (columns.ts); canvas handles fractional DPR with rounding and `setTransform`.
- `prefers-reduced-motion`, `prefers-contrast`, `prefers-reduced-transparency` are honoured.

## Não verificado
- Runtime layouts at 900x600, 1280x800, 1920x1080; no screenshots taken.
- WebKitGTK system font scale and DPR change behaviour; Wayland fractional scaling.
- Whether the centre pane really reaches 0 px (LAY-1) and the 861-900 px look (LAY-2): inferred from CSS only.
- Status bar (UI-008) not located in this area.
