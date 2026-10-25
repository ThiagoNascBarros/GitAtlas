#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Desktop launcher entry: opens GitAtlas with no repository. Desktop sessions do not load the
# shell profile, so tool paths are added here; output goes to ~/.cache/gitatlas/launch.log.
set -uo pipefail
# The installer copies this file to ~/.local/bin and bakes the project path into GITATLAS_ROOT,
# so the shortcut keeps working whichever Git branch is checked out.
root="${GITATLAS_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$(ls -d "$HOME"/.nvm/versions/node/*/bin 2>/dev/null | tail -1):$PATH"
log="$HOME/.cache/gitatlas/launch.log"
mkdir -p "$(dirname "$log")"
bin="$root/target/release/gitatlas"
if ! cd "$root" 2>/dev/null; then
  msg="Project folder not found: $root. Run scripts/install-shortcut.sh from the current folder."
  echo "$msg" >"$log"
  command -v notify-send >/dev/null && notify-send "GitAtlas failed to start" "$msg"
  exit 1
fi
# Rebuild only when a source file is newer than the binary.
if [ ! -x "$bin" ] || [ -n "$(find app/ui/src app/src-tauri/src crates -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.css' \) -newer "$bin" -print -quit)" ]; then
  scripts/run.sh --release >"$log" 2>&1
else
  "$bin" >"$log" 2>&1
fi
status=$?
if [ $status -ne 0 ]; then
  command -v notify-send >/dev/null && notify-send "GitAtlas failed to start" "See $log"
fi
exit $status
