// SPDX-License-Identifier: Apache-2.0
//! Commit graph: compact topology and lane layout (SDD §6).
//!
//! Each row draws inside its own cell:
//! - `In` segments come from the top edge at `from` lane into the node,
//! - `Out` segments leave the node toward the bottom edge at `to` lane,
//! - `Pass` segments cross the cell vertically on a lane not touching the node.
//! A parent edge from row i to row j is therefore `Out(i)` + `Pass` on every
//! row strictly between + `In(j)`, all on the same lane.

mod encode;
mod layout;
pub mod verify;

pub use encode::encode_window;
pub use layout::layout;

use serde::Serialize;

/// Row index into `Topology` order (display order, newest first).
pub type RowIdx = u32;

#[derive(Debug, Clone, Default)]
pub struct Topology {
    /// Parents of row i are `parents[parent_start[i]..parent_start[i + 1]]`.
    /// Parents missing from the loaded history are dropped and flagged.
    pub parents: Vec<RowIdx>,
    pub parent_start: Vec<u32>,
    pub has_missing_parent: Vec<bool>,
}

impl Topology {
    /// Builds topology from commits in display order (children before parents).
    pub fn from_commits<'a, I>(commits: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, &'a [String])>,
    {
        let commits: Vec<_> = commits.into_iter().collect();
        let index: std::collections::HashMap<&str, RowIdx> =
            commits.iter().enumerate().map(|(i, (sha, _))| (*sha, i as RowIdx)).collect();
        let mut t = Topology { parent_start: Vec::with_capacity(commits.len() + 1), ..Default::default() };
        for (_, parents) in &commits {
            t.parent_start.push(t.parents.len() as u32);
            let mut missing = false;
            for p in parents.iter() {
                match index.get(p.as_str()) {
                    Some(&row) => t.parents.push(row),
                    None => missing = true,
                }
            }
            t.has_missing_parent.push(missing);
        }
        t.parent_start.push(t.parents.len() as u32);
        t
    }

    pub fn len(&self) -> usize {
        self.has_missing_parent.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn parents_of(&self, row: RowIdx) -> &[RowIdx] {
        let r = row as usize;
        &self.parents[self.parent_start[r] as usize..self.parent_start[r + 1] as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SegKind {
    Pass,
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Segment {
    pub kind: SegKind,
    pub from: u16,
    pub to: u16,
    /// Color slot (UI maps it to `color.lane.<color % 10>`).
    pub color: u16,
}

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub lane: Vec<u16>,
    pub color: Vec<u16>,
    pub seg_start: Vec<u32>,
    pub segments: Vec<Segment>,
    /// Maximum number of simultaneously used lanes.
    pub lane_count: u16,
}

impl Layout {
    pub fn rows(&self) -> usize {
        self.lane.len()
    }

    pub fn segments_of(&self, row: RowIdx) -> &[Segment] {
        let r = row as usize;
        &self.segments[self.seg_start[r] as usize..self.seg_start[r + 1] as usize]
    }
}
