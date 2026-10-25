# Implementation Plan: Release documentation

**Spec**: `specs/005-release-docs/spec.md` | **Source**: `docs/specs/release-docs.md` | **Depends on**: 002, 003, 004

## Summary
Documentation only: `docs/user/*.md`, `docs/release/checklist.md`, `CHANGELOG.md`, README and HANDOFF updates. Cross-check labels against `app/ui/src/i18n.ts` and verify relative links.

## Gaps documented, not fixed
Theme choice not remembered; graph shows commits only reachable via `refs/atlaskraken/snapshots/*` or `refs/gitatlas/*`; `journal_prune` leaves `-redo`/`-wip` refs; no UI for the global shared-stash fetch mode; journal "Keep last" resets to 50.
