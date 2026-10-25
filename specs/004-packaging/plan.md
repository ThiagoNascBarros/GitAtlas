# Implementation Plan: Packaging

**Spec**: `specs/004-packaging/spec.md` | **Source**: `docs/specs/packaging.md`

## Summary
Packaging files under `packaging/`, Tauri bundle config in `app/src-tauri/tauri.conf.json`, two scripts, CI validators and a tag-triggered release workflow. No new dependencies.

## Decisions
GNOME 51 runtime with bundled Git 2.56.0 (`NO_RUST`, libsecret helper), pinned by tag + commit; .deb/AppImage built on ubuntu-22.04 → minimum glibc 2.35. Deb package `git-atlas` and `GitAtlas.desktop` come from `productName`, so the metainfo `launchable` only resolves in the Flatpak.
