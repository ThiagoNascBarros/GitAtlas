# Feature Specification: Commit graph visual parity with GitKraken

**Feature Branch**: `feat/graph-parity`
**Created**: 2026-10-25
**Status**: Draft
**Source**: GitKraken 12.6.1 theme data (Specwright note `gitkraken-12-6-1-graph-reference-collected-from`); reference only, no code or assets copied.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Dense, GitKraken-like graph rows (Priority: P1)

**Acceptance Scenarios**:

1. **Given** the graph, **Then** row height is 22px and lane spacing/node radius are exported constants with unit tests.
2. **Given** lane slot N, **Then** `laneColor(N % 10)` returns the GitKraken palette in both built-in themes.
3. **Given** a ref label, **Then** its background is the lane colour mixed 50% with the canvas background and text stays AA-contrast.

### User Story 2 - Pixel-checked rendering (Priority: P2)

**Acceptance Scenarios**:

1. **Given** a fixture repo, **Then** the pure geometry helpers (row to y, lane to x, edge control points) return fixed values asserted in tests.
2. **Given** HiDPI scale 1 and 2, **Then** strokes are crisp (half-pixel aligned) — asserted on the helper, verified by screenshot.

## Out of scope
Copying GitKraken code, icons, fonts or minified sources.
