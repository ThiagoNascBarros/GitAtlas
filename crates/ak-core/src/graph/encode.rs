// SPDX-License-Identifier: Apache-2.0
//! Binary `RowWindow` encoding (SDD §4.3), little-endian.
//!
//! Header: magic `AKRW`, version u16, start u32, count u32, lane_count u16.
//! Rows (count × 16 bytes): row u32, lane u16, color u16, flags u8, pad u8,
//!   seg_offset u32 (relative to window), seg_count u16.
//! Segments (8 bytes each): kind u8, pad u8, from u16, to u16, color u16.

use super::{Layout, RowIdx, SegKind, Topology};

pub const MAGIC: &[u8; 4] = b"AKRW";
pub const VERSION: u16 = 1;
pub const FLAG_MERGE: u8 = 1;
pub const FLAG_MISSING_PARENT: u8 = 2;

pub fn encode_window(t: &Topology, l: &Layout, start: RowIdx, count: u32) -> Vec<u8> {
    let start = start.min(l.rows() as u32);
    let end = (start + count).min(l.rows() as u32);
    let segs = l.segments_window(t, start, end);
    let total: usize = segs.iter().map(Vec::len).sum();
    let mut b = Vec::with_capacity(16 + (end - start) as usize * 16 + total * 8);
    b.extend_from_slice(MAGIC);
    b.extend_from_slice(&VERSION.to_le_bytes());
    b.extend_from_slice(&start.to_le_bytes());
    b.extend_from_slice(&(end - start).to_le_bytes());
    b.extend_from_slice(&l.lane_count.to_le_bytes());
    let mut offset = 0u32;
    for (k, r) in (start..end).enumerate() {
        let i = r as usize;
        let mut flags = 0u8;
        if t.parents_of(r).len() > 1 {
            flags |= FLAG_MERGE;
        }
        if t.has_missing_parent[i] {
            flags |= FLAG_MISSING_PARENT;
        }
        b.extend_from_slice(&r.to_le_bytes());
        b.extend_from_slice(&l.lane[i].to_le_bytes());
        b.extend_from_slice(&l.color[i].to_le_bytes());
        b.push(flags);
        b.push(0);
        b.extend_from_slice(&offset.to_le_bytes());
        b.extend_from_slice(&(segs[k].len() as u16).to_le_bytes());
        offset += segs[k].len() as u32;
    }
    for s in segs.iter().flatten() {
        b.push(match s.kind {
            SegKind::Pass => 0,
            SegKind::In => 1,
            SegKind::Out => 2,
        });
        b.push(0);
        b.extend_from_slice(&s.from.to_le_bytes());
        b.extend_from_slice(&s.to.to_le_bytes());
        b.extend_from_slice(&s.color.to_le_bytes());
    }
    b
}
