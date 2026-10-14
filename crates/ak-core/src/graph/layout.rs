// SPDX-License-Identifier: Apache-2.0
//! Single-pass lane assignment.
//!
//! `active[l]` holds the row a lane is waiting for (a reserved parent).
//! For each row: the node takes the leftmost lane reserved for it (other
//! lanes reserved for it merge in and free up); otherwise the leftmost free
//! lane. The first parent continues on the node's lane; additional parents
//! reuse a lane already reserved for them or take the leftmost free lane.

use super::{Layout, RowIdx, SegKind, Segment, Topology};

pub fn layout(t: &Topology) -> Layout {
    let n = t.len();
    let mut out = Layout {
        lane: Vec::with_capacity(n),
        color: Vec::with_capacity(n),
        seg_start: Vec::with_capacity(n + 1),
        segments: Vec::with_capacity(n * 3),
        lane_count: 0,
    };
    let mut active: Vec<Option<RowIdx>> = Vec::new();
    let mut lane_color: Vec<u16> = Vec::new();
    let mut next_color: u16 = 0;

    let take_free = |active: &mut Vec<Option<RowIdx>>, lane_color: &mut Vec<u16>, avoid: Option<usize>| -> usize {
        match active.iter().enumerate().position(|(l, a)| a.is_none() && Some(l) != avoid) {
            Some(l) => l,
            None => {
                active.push(None);
                lane_color.push(0);
                active.len() - 1
            }
        }
    };

    for row in 0..n as RowIdx {
        out.seg_start.push(out.segments.len() as u32);

        // Lanes arriving at this node from children above.
        let incoming: Vec<usize> = active.iter().enumerate().filter(|(_, a)| **a == Some(row)).map(|(l, _)| l).collect();
        let node = match incoming.first() {
            Some(&l) => l,
            None => {
                let l = take_free(&mut active, &mut lane_color, None);
                lane_color[l] = next_color;
                next_color = next_color.wrapping_add(1);
                l
            }
        };
        let node_color = lane_color[node];
        for &l in &incoming {
            out.segments.push(Segment { kind: SegKind::In, from: l as u16, to: node as u16, color: lane_color[l] });
            active[l] = None;
        }

        // Lanes passing by untouched (state before parents are reserved).
        for (l, a) in active.iter().enumerate() {
            if a.is_some() && l != node {
                out.segments.push(Segment { kind: SegKind::Pass, from: l as u16, to: l as u16, color: lane_color[l] });
            }
        }

        // Reserve lanes for parents.
        for (k, &p) in t.parents_of(row).iter().enumerate() {
            let existing = active.iter().position(|a| *a == Some(p));
            let target = match existing {
                Some(l) => l,
                None if k == 0 && active[node].is_none() => {
                    active[node] = Some(p);
                    lane_color[node] = node_color;
                    node
                }
                None => {
                    let l = take_free(&mut active, &mut lane_color, Some(node));
                    active[l] = Some(p);
                    lane_color[l] = next_color;
                    next_color = next_color.wrapping_add(1);
                    l
                }
            };
            let color = if target == node { node_color } else { lane_color[target] };
            out.segments.push(Segment { kind: SegKind::Out, from: node as u16, to: target as u16, color });
        }

        out.lane.push(node as u16);
        out.color.push(node_color);
        out.lane_count = out.lane_count.max(active.len() as u16);
        // Trailing free lanes can be dropped so the graph narrows again.
        while matches!(active.last(), Some(None)) {
            active.pop();
            lane_color.pop();
        }
    }
    out.seg_start.push(out.segments.len() as u32);
    out
}
