// SPDX-License-Identifier: Apache-2.0
//! Runs the `git` CLI with a fixed, safe environment and parses its
//! machine-readable output formats (see SDD §5).

mod caps;
mod exec;
pub mod parse;

pub use caps::{GitCaps, GitVersion};
pub use exec::{Git, GitError, Mode};
