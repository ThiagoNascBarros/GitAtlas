// SPDX-License-Identifier: Apache-2.0
//! Structural checks used by tests and debug builds.

use super::{Layout, RowIdx, SegKind, Topology};

/// Verifies that every parent edge is drawn as Out → Pass* → In on one lane
/// and that no Pass segment crosses a node. Returns the first violation.
pub fn check(t: &Topology, l: &Layout) -> Result<(), String> {
    if l.rows() != t.len() {
        return Err(format!("layout has {} rows, topology {}", l.rows(), t.len()));
    }
    for row in 0..t.len() as RowIdx {
        let node = l.lane[row as usize];
        for s in l.segments_of(row) {
            if s.kind == SegKind::Pass && s.from == node {
                return Err(format!("row {row}: pass segment through node lane {node}"));
            }
        }
        for &p in t.parents_of(row) {
            if p <= row {
                return Err(format!("row {row}: parent {p} is not below its child"));
            }
            let lane = l
                .segments_of(row)
                .iter()
                .find(|s| s.kind == SegKind::Out && edge_target_ok(t, l, row, s.to, p))
                .map(|s| s.to)
                .ok_or_else(|| format!("row {row}: no out-edge reaching parent {p}"))?;
            let _ = lane;
        }
    }
    Ok(())
}

/// True if following `lane` down from `row` reaches parent `p` through Pass segments.
fn edge_target_ok(_t: &Topology, l: &Layout, row: RowIdx, lane: u16, p: RowIdx) -> bool {
    for r in row + 1..p {
        if !l.segments_of(r).iter().any(|s| s.kind == SegKind::Pass && s.from == lane) {
            return false;
        }
    }
    l.segments_of(p).iter().any(|s| s.kind == SegKind::In && s.from == lane && s.to == l.lane[p as usize])
}
