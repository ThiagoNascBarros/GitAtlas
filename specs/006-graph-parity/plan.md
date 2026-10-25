# Implementation Plan: Graph parity

**Spec**: `specs/006-graph-parity/spec.md`

## Summary
Extract geometry and palette from `app/ui/src/features/graph/GraphView.tsx` (ROW_H 28, LANE_W 16) into a pure `geometry.ts` with tests (TDD), switch palette in `themes/atlas-*.theme.json`, then use the helpers in the canvas draw.

## Constraints
Max 2 concurrent workers (user machine has 7GB RAM). Never run REA on the whole GitKraken asar.
