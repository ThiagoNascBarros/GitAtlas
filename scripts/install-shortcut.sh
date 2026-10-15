#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Adds an "AtlasKraken" launcher to the application menu (and the desktop, if there is one).
# It opens the app without any repository (release build, rebuilt when sources changed).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
file="$HOME/.local/share/applications/atlaskraken.desktop"
bin="$HOME/.local/bin/atlaskraken-launch"
mkdir -p "$(dirname "$bin")"
sed "s|^root=.*|root=\"$root\"|" "$root/scripts/launch.sh" >"$bin"
chmod +x "$bin"
mkdir -p "$(dirname "$file")"
cat >"$file" <<EOF
[Desktop Entry]
Type=Application
Name=AtlasKraken
Comment=Git client
Exec=$bin
Path=$root
Icon=$root/app/src-tauri/icons/icon.png
Terminal=false
Categories=Development;RevisionControl;
StartupWMClass=atlaskraken
EOF
chmod +x "$file"
desk="$(xdg-user-dir DESKTOP 2>/dev/null || true)"
if [ -n "$desk" ] && [ -d "$desk" ]; then
  cp "$file" "$desk/AtlasKraken.desktop" && chmod +x "$desk/AtlasKraken.desktop"
  command -v gio >/dev/null && gio set "$desk/AtlasKraken.desktop" metadata::trusted true 2>/dev/null || true
fi
update-desktop-database "$(dirname "$file")" 2>/dev/null || true
echo "Installed: $file"
