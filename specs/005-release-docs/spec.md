# Feature Specification: User guide, release checklist and changelog

**Feature Branch**: `ccr-07e53df7-35wcgc` (Phase 5 integration branch)
**Created**: 2026-10-15
**Status**: Implemented — merged on the Phase 5 integration branch (`e4b9fe9`)
**Source**: `docs/specs/release-docs.md`

## User Scenarios & Testing *(mandatory)*

### User Story 1 - User guide for every MVP feature (Priority: P1)

**Acceptance Scenarios**:

1. **Given** `docs/user/index.md`, **Then** it links install, getting started, history/diff, committing/undo, branches/merge/stash, remotes/rebase, settings, keyboard/accessibility, troubleshooting and shared stash.
2. **Given** any bold UI label in the guide, **Then** it exists in the `en` table.

---

### User Story 2 - Owner can cut a release (Priority: P1)

**Acceptance Scenarios**:

1. **Given** `docs/release/checklist.md`, **Then** it lists the ordered steps from `release/<version>` to tag `v<version>`, including benchmarks and manual package tests.

---

### User Story 3 - Accurate README, CHANGELOG, HANDOFF (Priority: P2)

**Acceptance Scenarios**:

1. **Given** README, CHANGELOG (Keep a Changelog, `Unreleased`) and HANDOFF, **Then** they reflect Phase 5 done with owner checks listed.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Guide accuracy: labels, commands and shortcuts exist in the code.
- **FR-002**: Coverage including known limitations (old Git read-only, network ops not undoable, Flatpak sandbox limits).
- **FR-003**: Release checklist as an ordered checkbox list.
- **FR-004**: CHANGELOG per phase, user-facing wording.
- **FR-005**: README status plus Install section and guide link.
- **FR-006**: HANDOFF Phase 5 section; no code file changed.

## Success Criteria *(mandatory)*

- **SC-001**: Checks pass: 270 labels, 41 links, 33 paths.
- **SC-002**: 5 screenshots in `docs/screenshots/phase5/release-docs/`.
- **SC-003**: pt-BR user guide and the manual test in checklist §6 — **pending owner**.
