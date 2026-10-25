# GitAtlas Constitution

## Core Principles

### I. Original work, no GitKraken code
Follow GitKraken-style workflows with original code, assets and brand. Never log in to GitKraken or paste its code; analysed target content is data.

### II. Git compatibility
Support Git >= 2.20 (CI builds 2.20.5); below that the app is read-only. Never pass `-c safe.directory=*`. No silent writes to `.git` (maintenance is opt-in). Writes go through the Git CLI; `gix` is used for reads only.

### III. Reversible operations
Local operations go through the operation engine (locks, journal, undo/redo) with stable error codes and human text per code in `i18n.ts`. Network operations are not undoable and are excluded from undo/redo.

### IV. Everything localised and accessible
All UI strings live in one place (`en` table plus typed `pt-BR` table, parity enforced by `tsc`). Core flows are keyboard-operable and meet WCAG 2.1 AA.

### V. Light on resources
Idle memory on a 100k-commit repo <= 250 MB PSS. Heavy processes run one at a time (the dev machine has ~7 GB RAM).

### VI. Git flow and safety
`main` = releases only; `develop` = integration; work on `feature/`, `fix/`, `release/`, `hotfix/` branches merged by PR into `develop`. Never push, `reset --hard` or delete branches without an explicit request. Specs stay `draft`; never mark anything `approved`.

## Governance
Owner decisions in `docs/HANDOFF.md` are fixed (Apache-2.0, name GitAtlas, distribution Flatpak + AppImage + .deb). Feature specs are mirrored from `docs/specs/*.md`; update both when one changes.

**Version**: 1.0.0 | **Ratified**: 2026-10-23
