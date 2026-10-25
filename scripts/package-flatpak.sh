#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Builds the GitAtlas Flatpak from the release binary and exports a single-file bundle:
#   target/flatpak/GitAtlas-<version>.flatpak
# Usage: scripts/package-flatpak.sh
# Needs flatpak + flatpak-builder and the GNOME runtime/SDK named in the manifest. When they are
# missing and a "flathub" remote is configured, flatpak-builder installs them from it.
# Runs scripts/package.sh first when target/release/gitatlas does not exist yet.
set -euo pipefail
cd "$(dirname "$0")/.."
manifest=packaging/flatpak/dev.gitatlas.app.yml
app_id=dev.gitatlas.app
out=target/flatpak
flathub_url=https://dl.flathub.org/repo/flathub.flatpakrepo

for tool in flatpak flatpak-builder; do
  command -v "$tool" >/dev/null || {
    echo "error: '$tool' is required (Debian/Ubuntu: sudo apt-get install flatpak flatpak-builder)" >&2
    exit 1
  }
done

if [ ! -x target/release/gitatlas ]; then
  echo "target/release/gitatlas not found: running scripts/package.sh deb first"
  scripts/package.sh deb
fi

runtime="$(sed -n 's/^runtime: *//p' "$manifest")"
sdk="$(sed -n 's/^sdk: *//p' "$manifest")"
branch="$(sed -n "s/^runtime-version: *'\{0,1\}\([^']*\)'\{0,1\}/\1/p" "$manifest")"
deps_args=()
missing=()
for ref in "$runtime" "$sdk"; do
  flatpak info "$ref//$branch" >/dev/null 2>&1 || missing+=("$ref//$branch")
done
if [ "${#missing[@]}" -gt 0 ]; then
  if flatpak remotes --user --columns=name | grep -qx flathub; then
    deps_args=(--user --install-deps-from=flathub)
  elif flatpak remotes --system --columns=name | grep -qx flathub; then
    deps_args=(--install-deps-from=flathub)
  else
    echo "error: missing Flatpak runtime: ${missing[*]}" >&2
    echo "Add Flathub and run this script again (the runtime is then installed automatically):" >&2
    echo "  flatpak remote-add --user --if-not-exists flathub $flathub_url" >&2
    exit 1
  fi
fi

# Containers and CI runners often have no FUSE; rofiles-fuse is only an optimisation.
fuse_args=()
[ -e /dev/fuse ] || fuse_args=(--disable-rofiles-fuse)

version="$(sed -n 's/^ *"version": *"\([^"]*\)".*/\1/p' app/src-tauri/tauri.conf.json | head -n1)"
mkdir -p "$out"
flatpak-builder --force-clean --sandbox "${fuse_args[@]}" "${deps_args[@]}" \
  --state-dir="$out/.flatpak-builder" --repo="$out/repo" "$out/build-dir" "$manifest"
bundle="$out/GitAtlas-$version.flatpak"
flatpak build-bundle --runtime-repo="$flathub_url" "$out/repo" "$bundle" "$app_id"
echo
echo "Flatpak bundle: $bundle"
echo "Install it with: flatpak install --user $bundle"
