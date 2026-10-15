// SPDX-License-Identifier: Apache-2.0
//! AtlasGit core: repository reading and commit graph (SDD §3).
//! Has no UI dependency.

pub mod cache;
pub mod graph;
pub mod ops;
pub mod read;
#[cfg(feature = "gix-reader")]
pub mod read_gix;
