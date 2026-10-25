# Feature Specification: Shared Stash

**Feature Branch**: `feature/shared-stash`
**Created**: 2026-10-15
**Status**: Implemented — merged into `develop` (PR #12); manual packaged-app run pending
**Source**: `docs/specs/shared-stash.md` (rev 3), PRD criterion 14

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Share a local stash to a remote (Priority: P1)

An author publishes a local stash (including untracked files) to a remote without creating a branch.

**Why this priority**: it is the whole feature; nothing else exists without it.

**Independent Test**: stash with `-u`, share to bare remote R1, check the ref exists on R1.

**Acceptance Scenarios**:

1. **Given** a local stash and a remote, **When** A shares it, **Then** `refs/gitatlas/shared-stash/<A-slug>/<id>` exists on R1.
2. **Given** a publish with more than 10 MB of new blobs, **When** shared, **Then** a warning is shown and it still publishes.
3. **Given** a bare remote whose hook rejects `refs/gitatlas/*`, **When** probed, **Then** `REMOTE_REJECTED` is cached and a notice is shown.

---

### User Story 2 - Discover and apply a teammate's stash (Priority: P1)

A consumer sees the shared stashes grouped by remote and applies them or branches from them.

**Independent Test**: clone B fetches from R1, sees A's stash under "Team", applies it.

**Acceptance Scenarios**:

1. **Given** fetch mode `panel` and the tab closed, **When** B fetches, **Then** nothing is downloaded; opening the tab downloads it. Mode `always` downloads on plain Fetch.
2. **Given** B applies, **Then** the working tree equals A's stash (tracked + untracked); a dirty overlapping file returns `DIRTY_TREE` and changes nothing.
3. **Given** B creates a branch from it, **Then** the branch is at the base with the changes applied and nothing else touched.
4. **Given** a ref pointing at a non-stash commit, **Then** it is listed invalid and apply returns `INVALID_SHARED_STASH`.

---

### User Story 3 - Author-only update and unshare (Priority: P2)

Only the author can update, rename or unshare; others only apply or branch.

**Acceptance Scenarios**:

1. **Given** B is not the author, **Then** Update/Unshare are hidden and the engine returns `NOT_AUTHOR`; a forged trailer email is shown as "claimed author" (not verified).
2. **Given** A updates content or renames, **Then** B sees it after fetch; if the remote ref moved meanwhile, `STALE_SHARED_STASH` and nothing is overwritten.
3. **Given** A unshares, **Then** it disappears for B after fetch, appears in A's "Recently unshared" and Reshare restores it.

---

### User Story 4 - Several remotes and author identity (Priority: P3)

**Acceptance Scenarios**:

1. **Given** stashes on R1 and R2 and R2 unreachable, **Then** R1 is listed and R2 shows an error.
2. **Given** two authors with the same `user.name`, **Then** refs are distinct and "Mine" is correct for each; a changed `user.name` keeps old stashes "Mine".

### Edge Cases

- Remote ref moved between read and write (stale lease).
- Missing base commit on apply/branch (`BASE_MISSING`).
- Apply with `--index` failing: fallback without it, flagged in the outcome.
- Git 2.20 `fetch --prune` with a custom refspec.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Stashes MUST be published as a wrapper commit with metadata trailers under `refs/gitatlas/shared-stash/<author-slug>/<id>`.
- **FR-002**: Ownership is declarative (trailer email); engine MUST refuse update/unshare from non-authors.
- **FR-003**: Fetch MUST be controlled by `sharedStash.fetchMode` (`panel` default | `always`), repo-level with global default.
- **FR-004**: All reads MUST validate trailers and stash shape; invalid refs are kept and flagged.
- **FR-005**: Network operations MUST take `Lock::Remote`, run off the UI thread with the askpass bridge, and be cancellable; they are not undoable.
- **FR-006**: The UI MUST provide a "Shared stash" sidebar tab, share dialog, context menus, graph glyph on the base commit and keyboard navigation.

### Key Entities

- **SharedStash**: wrapper sha, author slug, title, base branch, mine/claimed/verified flags.
- **Remote capability probe**: cached result per remote in `.git/gitatlas/remotes.json`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The 13 acceptance criteria of spec §9 pass in the e2e suite (`crates/ak-core/tests/shared_stash.rs`).
- **SC-002**: The suite passes on Git 2.20.5 and current Git in CI.
- **SC-003**: Manual run with two clones and a local bare remote is recorded (checklist §6) — **pending owner**.
