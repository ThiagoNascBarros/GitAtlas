#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Adds an "AtlasGit" launcher to the application menu (and the desktop, if there is one).
# It opens the app without any repository (release build, rebuilt when sources changed).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
file="$HOME/.local/share/applications/atlasgit.desktop"
bin="$HOME/.local/bin/atlasgit-launch"
# Drop the launcher installed before the AtlasKraken -> AtlasGit rename.
rm -f "$HOME/.local/share/applications/atlaskraken.desktop" "$HOME/.local/bin/atlaskraken-launch"
mkdir -p "$(dirname "$bin")"
sed "s|^root=.*|root=\"$root\"|" "$root/scripts/launch.sh" >"$bin"
chmod +x "$bin"
mkdir -p "$(dirname "$file")"
cat >"$file" <<EOF
[Desktop Entry]
Type=Application
Name=AtlasGit
Comment=Git client
Exec=$bin
Path=$root
Icon=$root/app/src-tauri/icons/icon.png
Terminal=false
Categories=Development;RevisionControl;
StartupWMClass=atlasgit
EOF
chmod +x "$file"
desk="$(xdg-user-dir DESKTOP 2>/dev/null || true)"
if [ -n "$desk" ] && [ -d "$desk" ]; then
  rm -f "$desk/AtlasKraken.desktop"
  cp "$file" "$desk/AtlasGit.desktop" && chmod +x "$desk/AtlasGit.desktop"
  command -v gio >/dev/null && gio set "$desk/AtlasGit.desktop" metadata::trusted true 2>/dev/null || true
fi
update-desktop-database "$(dirname "$file")" 2>/dev/null || true
echo "Installed: $file"
