#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Builds the UI and runs GitAtlas with the UI embedded (no dev server needed).
# Usage: scripts/run.sh [--release] [path/to/repo]
set -euo pipefail
cd "$(dirname "$0")/.."
profile=debug
cargo_flags=()
if [ "${1:-}" = "--release" ]; then
  profile=release
  cargo_flags=(--release)
  shift
fi
# Few jobs: a full-parallel release build can exhaust RAM on small machines (override with CARGO_BUILD_JOBS).
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
pnpm --dir app/ui install --frozen-lockfile >/dev/null
pnpm --dir app/ui build >/dev/null
cargo build "${cargo_flags[@]}" -p gitatlas --features custom-protocol
exec "target/$profile/gitatlas" ${1:+"$1"}
