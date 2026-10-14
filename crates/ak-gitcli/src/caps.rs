// SPDX-License-Identifier: Apache-2.0
//! Feature detection from the installed git version (SDD §5, `GitCaps`).

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct GitVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl GitVersion {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Parses the output of `git version`, e.g. `git version 2.43.0` or
    /// `git version 2.39.3 (Apple Git-146)` or `git version 2.45.1.windows.1`.
    pub fn parse(output: &str) -> Option<Self> {
        let rest = output.trim().strip_prefix("git version ")?;
        let mut nums = rest
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<u32>().ok());
        Some(Self::new(nums.next()??, nums.next()??, nums.next().flatten().unwrap_or(0)))
    }
}

/// Minimum supported git. Below this, AtlasKraken opens repos read-only.
pub const MIN_SUPPORTED: GitVersion = GitVersion::new(2, 20, 0);

#[derive(Debug, Clone, Serialize)]
pub struct GitCaps {
    pub version: GitVersion,
    /// `git status --porcelain=v2` (2.11).
    pub porcelain_v2: bool,
    /// `--pathspec-from-file` / `--pathspec-file-nul` (2.26).
    pub pathspec_from_file: bool,
    /// `git merge-tree --write-tree` for conflict prediction (2.38).
    pub merge_tree_write_tree: bool,
    /// `git stash push -- <pathspec>` (2.13).
    pub stash_push_pathspec: bool,
}

impl GitCaps {
    pub fn from_version(v: GitVersion) -> Self {
        Self {
            version: v,
            porcelain_v2: v >= GitVersion::new(2, 11, 0),
            pathspec_from_file: v >= GitVersion::new(2, 26, 0),
            merge_tree_write_tree: v >= GitVersion::new(2, 38, 0),
            stash_push_pathspec: v >= GitVersion::new(2, 13, 0),
        }
    }

    pub fn supported(&self) -> bool {
        self.version >= MIN_SUPPORTED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_version_strings() {
        assert_eq!(GitVersion::parse("git version 2.43.0\n"), Some(GitVersion::new(2, 43, 0)));
        assert_eq!(GitVersion::parse("git version 2.39.3 (Apple Git-146)"), Some(GitVersion::new(2, 39, 3)));
        assert_eq!(GitVersion::parse("git version 2.45.1.windows.1"), Some(GitVersion::new(2, 45, 1)));
        assert_eq!(GitVersion::parse("git version 2.20"), Some(GitVersion::new(2, 20, 0)));
        assert_eq!(GitVersion::parse("nonsense"), None);
    }

    #[test]
    fn caps_follow_version() {
        let old = GitCaps::from_version(GitVersion::new(2, 20, 1));
        assert!(old.supported() && old.porcelain_v2);
        assert!(!old.pathspec_from_file && !old.merge_tree_write_tree);
        let new = GitCaps::from_version(GitVersion::new(2, 53, 0));
        assert!(new.pathspec_from_file && new.merge_tree_write_tree);
        assert!(!GitCaps::from_version(GitVersion::new(2, 19, 9)).supported());
    }
}
