#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Builds the UI and runs AtlasKraken with the UI embedded (no dev server needed).
# Usage: scripts/run.sh [path/to/repo]
set -euo pipefail
cd "$(dirname "$0")/.."
pnpm --dir app/ui install --frozen-lockfile >/dev/null
pnpm --dir app/ui build >/dev/null
cargo build -p atlaskraken --features custom-protocol
exec target/debug/atlaskraken "${1:-}"