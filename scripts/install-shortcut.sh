#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Adds an "GitAtlas" launcher to the application menu (and the desktop, if there is one).
# It opens the app without any repository (release build, rebuilt when sources changed).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
file="$HOME/.local/share/applications/gitatlas.desktop"
bin="$HOME/.local/bin/gitatlas-launch"
# Drop launchers installed before the AtlasKraken -> AtlasGit -> GitAtlas renames.
rm -f "$HOME/.local/share/applications/atlaskraken.desktop" "$HOME/.local/bin/atlaskraken-launch" \
  "$HOME/.local/share/applications/atlasgit.desktop" "$HOME/.local/bin/atlasgit-launch"
mkdir -p "$(dirname "$bin")"
sed "s|^root=.*|root=\"$root\"|" "$root/scripts/launch.sh" >"$bin"
chmod +x "$bin"
# Install the icon under a theme name: the shell caches icons by file path, so a path icon goes stale.
icon="$HOME/.local/share/icons/hicolor/512x512/apps/gitatlas.png"
mkdir -p "$(dirname "$icon")"
cp "$root/app/src-tauri/icons/icon.png" "$icon"
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
mkdir -p "$(dirname "$file")"
# No Path= key: the launcher cd's into the project itself, so a moved or renamed folder can no longer
# make the desktop shell fail before the launcher runs. Written to a temp file and renamed so the
# shell sees a fresh entry rather than an in-place edit.
tmp="$(mktemp "$file.XXXXXX")"
cat >"$tmp" <<EOF
[Desktop Entry]
Type=Application
Name=GitAtlas
Comment=Git client
Exec=$bin
Icon=gitatlas
Terminal=false
Categories=Development;RevisionControl;
StartupWMClass=gitatlas
EOF
chmod +x "$tmp" && mv -f "$tmp" "$file"
desk="$(xdg-user-dir DESKTOP 2>/dev/null || true)"
if [ -n "$desk" ] && [ -d "$desk" ]; then
  rm -f "$desk/AtlasKraken.desktop"
  cp "$file" "$desk/GitAtlas.desktop" && chmod +x "$desk/GitAtlas.desktop"
  command -v gio >/dev/null && gio set "$desk/GitAtlas.desktop" metadata::trusted true 2>/dev/null || true
fi
update-desktop-database "$(dirname "$file")" 2>/dev/null || true
echo "Installed: $file"
