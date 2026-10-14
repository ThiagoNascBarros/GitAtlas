// SPDX-License-Identifier: Apache-2.0
//! Single-pass lane assignment.
//!
//! `active[l]` holds the row a lane is waiting for (a reserved parent).
//! For each row: the node takes the leftmost lane reserved for it (other
//! lanes reserved for it merge in and free up); otherwise the leftmost free
//! lane. The first parent continues on the node's lane; additional parents
//! reuse a lane already reserved for them or take the leftmost free lane.
//!
//! Only In/Out segments are stored. Pass segments are derived from the lane
//! state before each row, which is replayed from checkpoints taken every
//! `CHECKPOINT_EVERY` rows (SDD §6), so memory does not grow with lane count.

use super::{Layout, RowIdx, SegKind, Segment, Topology};

pub const CHECKPOINT_EVERY: usize = 1024;

/// Lane state between rows. Cloned into checkpoints.
#[derive(Debug, Clone, Default)]
pub struct LaneState {
    pub active: Vec<Option<RowIdx>>,
    pub lane_color: Vec<u16>,
    pub next_color: u16,
}

impl LaneState {
    /// Pass segments of `row`: lanes active before the row and not ending in it.
    pub fn pass_segments(&self, row: RowIdx, out: &mut Vec<Segment>) {
        for (l, a) in self.active.iter().enumerate() {
            if matches!(a, Some(x) if *x != row) {
                out.push(Segment { kind: SegKind::Pass, from: l as u16, to: l as u16, color: self.lane_color[l] });
            }
        }
    }

    fn take_free(&mut self, avoid: Option<usize>) -> usize {
        match self.active.iter().enumerate().position(|(l, a)| a.is_none() && Some(l) != avoid) {
            Some(l) => l,
            None => {
                self.active.push(None);
                self.lane_color.push(0);
                self.active.len() - 1
            }
        }
    }

    fn new_color(&mut self) -> u16 {
        let c = self.next_color;
        self.next_color = self.next_color.wrapping_add(1);
        c
    }

    /// Advances over `row`, pushing its In/Out segments. Returns (node lane, node color).
    pub fn step(&mut self, t: &Topology, row: RowIdx, io: &mut Vec<Segment>) -> (u16, u16) {
        let incoming: Vec<usize> =
            self.active.iter().enumerate().filter(|(_, a)| **a == Some(row)).map(|(l, _)| l).collect();
        let node = match incoming.first() {
            Some(&l) => l,
            None => {
                let l = self.take_free(None);
                self.lane_color[l] = self.new_color();
                l
            }
        };
        let node_color = self.lane_color[node];
        for &l in &incoming {
            io.push(Segment { kind: SegKind::In, from: l as u16, to: node as u16, color: self.lane_color[l] });
            self.active[l] = None;
        }
        for (k, &p) in t.parents_of(row).iter().enumerate() {
            let target = match self.active.iter().position(|a| *a == Some(p)) {
                Some(l) => l,
                None if k == 0 && self.active[node].is_none() => {
                    self.active[node] = Some(p);
                    self.lane_color[node] = node_color;
                    node
                }
                None => {
                    let l = self.take_free(Some(node));
                    self.active[l] = Some(p);
                    self.lane_color[l] = self.new_color();
                    l
                }
            };
            let color = if target == node { node_color } else { self.lane_color[target] };
            io.push(Segment { kind: SegKind::Out, from: node as u16, to: target as u16, color });
        }
        // Trailing free lanes are dropped so the graph narrows again.
        while matches!(self.active.last(), Some(None)) {
            self.active.pop();
            self.lane_color.pop();
        }
        (node as u16, node_color)
    }
}

pub fn layout(t: &Topology) -> Layout {
    let n = t.len();
    let mut out = Layout {
        lane: Vec::with_capacity(n),
        color: Vec::with_capacity(n),
        seg_start: Vec::with_capacity(n + 1),
        segments: Vec::with_capacity(n * 2),
        checkpoints: Vec::with_capacity(n / CHECKPOINT_EVERY + 1),
        lane_count: 0,
    };
    let mut state = LaneState::default();
    for row in 0..n {
        if row % CHECKPOINT_EVERY == 0 {
            out.checkpoints.push(state.clone());
        }
        out.lane_count = out.lane_count.max(state.active.len() as u16);
        out.seg_start.push(out.segments.len() as u32);
        let (lane, color) = state.step(t, row as RowIdx, &mut out.segments);
        out.lane.push(lane);
        out.color.push(color);
        out.lane_count = out.lane_count.max(state.active.len() as u16).max(lane + 1);
    }
    out.seg_start.push(out.segments.len() as u32);
    out
}
