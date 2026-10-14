// SPDX-License-Identifier: Apache-2.0
//! Structural checks used by tests and benchmarks.

use super::{Layout, RowIdx, SegKind, Segment, Topology};

/// Verifies that every parent edge is drawn as Out → Pass* → In on one lane
/// and that no Pass segment crosses a node. Returns the first violation.
pub fn check(t: &Topology, l: &Layout) -> Result<(), String> {
    if l.rows() != t.len() {
        return Err(format!("layout has {} rows, topology {}", l.rows(), t.len()));
    }
    let all = l.segments_window(t, 0, t.len() as RowIdx);
    for row in 0..t.len() as RowIdx {
        let node = l.lane[row as usize];
        for s in &all[row as usize] {
            if s.kind == SegKind::Pass && s.from == node {
                return Err(format!("row {row}: pass segment through node lane {node}"));
            }
        }
        for &p in t.parents_of(row) {
            if p <= row {
                return Err(format!("row {row}: parent {p} is not below its child"));
            }
            if !all[row as usize].iter().any(|s| s.kind == SegKind::Out && reaches(&all, l, row, s.to, p)) {
                return Err(format!("row {row}: no out-edge reaching parent {p}"));
            }
        }
    }
    Ok(())
}

/// True if following `lane` down from `row` reaches parent `p` through Pass segments.
fn reaches(all: &[Vec<Segment>], l: &Layout, row: RowIdx, lane: u16, p: RowIdx) -> bool {
    for r in row + 1..p {
        if !all[r as usize].iter().any(|s| s.kind == SegKind::Pass && s.from == lane) {
            return false;
        }
    }
    all[p as usize].iter().any(|s| s.kind == SegKind::In && s.from == lane && s.to == l.lane[p as usize])
}
