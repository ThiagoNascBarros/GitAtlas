# Feature Specification: Linux packaging

**Feature Branch**: `ccr-07e53df7-35wcgc` (Phase 5 integration branch)
**Created**: 2026-10-15
**Status**: Implemented — merged on the Phase 5 integration branch (`a1c00ed`); Flatpak not built locally
**Source**: `docs/specs/packaging.md`

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Install from a .deb or AppImage (Priority: P1)

**Independent Test**: build with `scripts/package.sh`, install the .deb, launch from the menu.

**Acceptance Scenarios**:

1. **Given** the .deb, **Then** `dpkg-deb -I` shows `git (>= 1:2.20)` in Depends and `-c` shows desktop entry, icons and metainfo.
2. **Given** the AppImage, **Then** it launches using the host Git (Git is not bundled).

---

### User Story 2 - Install the Flatpak (Priority: P2)

**Acceptance Scenarios**:

1. **Given** `scripts/package-flatpak.sh`, **Then** it produces `GitAtlas-<version>.flatpak` with bundled Git and ssh available in the sandbox.

---

### User Story 3 - CI validates and builds artifacts (Priority: P2)

**Acceptance Scenarios**:

1. **Given** a PR, **Then** `ci.yml` runs `desktop-file-validate` and `appstreamcli validate --no-net` with no errors.
2. **Given** a `v*` tag or manual dispatch, **Then** `release.yml` uploads `.deb`, `.AppImage` and `.flatpak` and never publishes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `packaging/linux/dev.gitatlas.app.desktop` valid, with pt_BR names.
- **FR-002**: `packaging/linux/dev.gitatlas.app.metainfo.xml` (AppStream, EN + pt-BR).
- **FR-003**: hicolor icons (32–512) and Tauri bundle `icon` list.
- **FR-004**: .deb via Tauri with the `git (>= 1:2.20)` dependency.
- **FR-005**: AppImage via Tauri using the host Git.
- **FR-006**: `packaging/flatpak/dev.gitatlas.app.yml` (GNOME runtime, bundled Git, commented finish-args).
- **FR-007**: `scripts/package.sh` and `scripts/package-flatpak.sh`.
- **FR-008**: CI validators in the `app` job.
- **FR-009**: `.github/workflows/release.yml` (artifacts only, no secrets).

## Success Criteria *(mandatory)*

- **SC-001**: .deb and AppImage built and launched under Xvfb; validators pass.
- **SC-002**: Flatpak first build happens in `release.yml` — **not verified here** (Flathub blocked by the proxy).
- **SC-003**: 5 screenshots in `docs/screenshots/phase5/packaging/`.
- **SC-004**: Owner questions in `mvp-phase5.md` §8.1 (deb/desktop naming, `.app` identifier warning, Flatpak Git pin, narrowing `--filesystem=home`) — **pending owner**.
