# Tasks: Packaging

**Spec**: `specs/004-packaging/spec.md`

## Phase 1: Metadata
- [X] T001 [US1] Read the Tauri 2 schema for deb/appimage keys
- [X] T002 [US1] Desktop entry and AppStream metainfo validated
- [X] T003 [US1] hicolor icons and bundle `icon` list

## Phase 2: Bundles
- [X] T004 [US1] .deb and AppImage config plus `scripts/package.sh`; built and launched under Xvfb
- [X] T005 [US2] Flatpak manifest and `scripts/package-flatpak.sh`
- [ ] T006 [US2] First real Flatpak build (done by `release.yml`)

## Phase 3: CI
- [X] T007 [US3] Validators in `ci.yml`
- [X] T008 [US3] `.github/workflows/release.yml`
- [X] T009 [US1] Implementation notes and 5 screenshots
- [ ] T010 [US1] Owner decisions on §8.1 naming, Flatpak Git pin and `--filesystem=home`
