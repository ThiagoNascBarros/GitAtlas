---
id: webkitgtk-css-zoom-and-glass-containing-blocks
type: pitfall
title: WebKitGTK CSS zoom and glass containing blocks
tags:
  - webkitgtk
  - css-zoom
  - layout
  - toolbar
  - testing
date: 2026-10-25
---
Under CSS `zoom` on .app (WebKitGTK 2.52):
- getBoundingClientRect and clientX/Y are on-screen px.
- offsetWidth, clientWidth and scrollTop are css px. Divide pointer deltas by visualScale(el) (src/features/split/size.ts).
- position:fixed left/top are multiplied by the zoom. menu.ctx undoes it with `zoom: var(--ui-unzoom); scale: var(--ui-zoom); transform-origin: 0 0`.
- Media queries ignore CSS zoom.
- With a fractional zoom, scrollWidth and clientWidth can differ by 1px with no real overflow, so compare with +1.

Other pitfalls:
- The bundled Open Sans loads after the first layout, so re-measure after document.fonts.ready.
- `.toolbar` and `.toolbar .group` have backdrop-filter, which makes them containing blocks for fixed and absolute children. Popovers from the toolbar (the More panel) must render outside the header, as children of .app.
- To probe layout headlessly, use Gtk.OffscreenWindow with WebKit2 4.1 and WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11. Take screenshots with win.get_pixbuf() (python cairo foreign is not installed). Backdrop blur does not render in that mode.
- The dev mock needs public/mock/*.json, which is gitignored; copy it from the main checkout.
