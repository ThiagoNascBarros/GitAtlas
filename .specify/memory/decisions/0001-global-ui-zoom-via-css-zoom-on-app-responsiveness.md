---
id: "0001"
feature: 007-ui-review
title: Global UI zoom via CSS zoom on .app, responsiveness measured in JS
date: 2026-10-25
status: accepted
---
## Decision
The 80–150% UI zoom is CSS `zoom: var(--ui-zoom)` on .app (stored in localStorage ak.split.zoom). Pointer math compensates with visualScale(el) in Splitter and GraphView, and fixed menus (menu.ctx) undo it with `zoom: var(--ui-unzoom); scale: var(--ui-zoom)`. Drawer mode (data-drawers none/left/both at 900/640 css px) and toolbar collapse (data-squeeze 0–4) are measured in App.tsx from .app clientWidth and toolbar scrollWidth, not with media queries.

## Rationale
Native webview zoom needs the core:webview:allow-set-webview-zoom capability in a file this role does not own. Media queries see on-screen px and ignore CSS zoom, so they would pick the wrong layout at 80% or 150%. Measuring the toolbar collapses it only when it really overflows, whatever the language or counts.

## Alternatives considered
Tauri setZoom (needs a capability change; zooms the whole webview including menus correctly); container queries (WebKitGTK evaluates them, but the squeeze levels depend on content width, not just the container width); transform: scale on .app (breaks fixed positioning and hit-testing).
