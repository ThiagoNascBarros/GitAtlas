// SPDX-License-Identifier: Apache-2.0
//! AtlasKraken core: repository reading and commit graph (SDD §3).
//! Has no UI dependency.

pub mod graph;
pub mod read;
#[cfg(feature = "gix-reader")]
pub mod read_gix;
