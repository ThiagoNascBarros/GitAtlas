#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Builds the release Linux packages (.deb and AppImage) with the Tauri bundler and prints their paths.
# Usage: scripts/package.sh [deb|appimage|deb,appimage]   (default: deb,appimage)
# The release binary it leaves in target/release/atlasgit is what scripts/package-flatpak.sh repacks.
set -euo pipefail
cd "$(dirname "$0")/.."
bundles="${1:-deb,appimage}"
case "$bundles" in
  deb | appimage | deb,appimage | appimage,deb) ;;
  *) echo "usage: $0 [deb|appimage|deb,appimage]" >&2; exit 2 ;;
esac
for tool in pnpm cargo; do
  command -v "$tool" >/dev/null || { echo "error: '$tool' is required but was not found in PATH" >&2; exit 1; }
done
# linuxdeploy (AppImage) is itself an AppImage: without FUSE (containers, CI) it must self-extract.
if [[ "$bundles" == *appimage* ]] && ! { [ -e /dev/fuse ] && command -v fusermount >/dev/null; }; then
  export APPIMAGE_EXTRACT_AND_RUN=1
fi
pnpm --dir app/ui install --frozen-lockfile >/dev/null
# `tauri build` runs `beforeBuildCommand` (UI build), compiles the release binary with the UI
# embedded and bundles it. Bundle settings live in app/src-tauri/tauri.conf.json (bundle.linux).
# The CLI is the devDependency of app/ui. It only finds tauri.conf.json in subfolders of where it
# starts, and runs `beforeBuildCommand` (`pnpm --dir ../ui build`) in the frontend directory, so
# both directories are given explicitly.
TAURI_APP_PATH="$PWD/app/src-tauri" TAURI_FRONTEND_PATH="$PWD/app/ui" \
  pnpm --dir app/ui tauri build --features custom-protocol --bundles "$bundles"
version="$(sed -n 's/^ *"version": *"\([^"]*\)".*/\1/p' app/src-tauri/tauri.conf.json | head -n1)"
echo
echo "AtlasGit $version packages:"
found=0
for f in target/release/bundle/deb/*_"$version"_*.deb target/release/bundle/appimage/*_"$version"_*.AppImage; do
  [ -e "$f" ] || continue
  echo "  $f"
  found=1
done
echo "  target/release/atlasgit (release binary)"
[ "$found" = 1 ] || { echo "error: no package was produced" >&2; exit 1; }
