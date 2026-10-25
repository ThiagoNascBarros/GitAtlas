---
id: gitkraken-12-6-1-graph-reference-collected-from
type: learning
title: GitKraken 12.6.1 graph reference (collected from asar, not REA)
tags: []
date: 2026-10-25
---
Source: /usr/share/gitkraken/resources/app.asar (v12.6.1). Extracts in ~/Documentos/Develop/AtlasEcoSystem/_gk-ref/extract (outside repo; reference only, never copy code/assets). Lane palette (dark=light): 0 #15a0bf, 1 #0669f7, 2 #8e00c2, 3 #c517b6, 4 #d90171, 5 #cd0101, 6 #f25d2e, 7 #f2ca33, 8 #7bd938, 9 #2ece9d. graph-row-height 22px (GitAtlas: ROW_H 28, LANE_W 16 in GraphView.tsx). Ref label bg = mixLess(lane color, app bg0, 50%/45%); row fades f10/f50. Stats bar radius 3px. Dimmed text = text-normal 20% alpha. Font Open Sans. Graph drawing code is minified under /src/render/static, not yet extracted. PITFALL: user machine has 7GB RAM; REA mcp holds ~1.3GB and full-asar analysis (296MB) needs ~1GB heap; never run it, extract single files with @electron/asar instead. Close GitKraken when not capturing.
