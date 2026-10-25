---
id: build-pitfalls-after-atlasgit-to-gitatlas-rename
type: pitfall
title: Build pitfalls after AtlasGit to GitAtlas rename
tags: []
date: 2026-10-25
---
Cargo tauri build script failed with stale path AtlasGit/target/... ; fix = cargo clean -p gitatlas -p tauri. app/ui `pnpm build` runs tsc and failed because features/diagnostics/logic.test.ts had no logic.ts (Phase 5 left half-landed); launch.sh then exits 1 silently (see ~/.cache/gitatlas/launch.log). main.rs must declare `mod diagnostics;` and call diagnostics::init(); use crate::diagnostics::lock(&m) instead of .lock().unwrap(). Use CARGO_BUILD_JOBS=2 on this 7GB machine. Stale .claude/worktrees/auditoria/.git broke git status; renamed to .git.stale.
