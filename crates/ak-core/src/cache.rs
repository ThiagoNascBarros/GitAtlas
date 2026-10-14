// SPDX-License-Identifier: Apache-2.0
//! On-disk cache of the commit list (first-paint path, SDD §6 step 1).
//!
//! The cache stores the rows in display order plus a *fingerprint* of the refs
//! they were built from. A cache is used only if the fingerprint still matches,
//! so a moved branch, a new commit or a rewritten history invalidates it. The
//! cache is a convenience: any read or parse problem means "no cache".

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use ak_gitcli::parse::CommitRecord;
use serde::{Deserialize, Serialize};

const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Stored {
    version: u32,
    fingerprint: u64,
    commits: Vec<CommitRecord>,
}

/// Hash of every ref name and target, plus HEAD. Changes whenever a ref moves.
pub fn fingerprint(refs: &[(String, String)], head: Option<&str>) -> u64 {
    let mut sorted = refs.to_vec();
    sorted.sort();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    VERSION.hash(&mut h);
    sorted.hash(&mut h);
    head.hash(&mut h);
    h.finish()
}

/// Where the cache for a repo lives: `$XDG_CACHE_HOME/atlaskraken/graph/<hash-of-git-dir>.json`.
pub fn cache_path(git_dir: &Path) -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    git_dir.hash(&mut h);
    Some(base.join("atlaskraken").join("graph").join(format!("{:016x}.json", h.finish())))
}

/// Returns the cached commits when the fingerprint matches; `None` otherwise.
pub fn load(path: &Path, fingerprint_now: u64) -> Option<Vec<CommitRecord>> {
    let text = std::fs::read_to_string(path).ok()?;
    let stored: Stored = serde_json::from_str(&text).ok()?;
    (stored.version == VERSION && stored.fingerprint == fingerprint_now).then_some(stored.commits)
}

/// Writes the cache atomically (temp file + rename). Errors are ignored by design.
pub fn save(path: &Path, fingerprint_now: u64, commits: &[CommitRecord]) {
    let stored = Stored { version: VERSION, fingerprint: fingerprint_now, commits: commits.to_vec() };
    let Ok(text) = serde_json::to_string(&stored) else { return };
    if let Some(dir) = path.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
    }
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_changes_when_a_ref_moves() {
        let a = vec![("refs/heads/main".to_owned(), "aaa".to_owned())];
        let b = vec![("refs/heads/main".to_owned(), "bbb".to_owned())];
        assert_ne!(fingerprint(&a, Some("aaa")), fingerprint(&b, Some("aaa")));
        assert_ne!(fingerprint(&a, Some("aaa")), fingerprint(&a, Some("bbb")));
        assert_eq!(fingerprint(&a, None), fingerprint(&a, None));
    }

    #[test]
    fn roundtrip_and_invalidation() {
        let dir = std::env::temp_dir().join(format!("ak-cache-test-{}", std::process::id()));
        let path = dir.join("g.json");
        let commits = vec![CommitRecord {
            sha: "a".repeat(40),
            parents: vec![],
            author: "Ada".into(),
            email: "ada@x".into(),
            time: 1,
            summary: "root".into(),
        }];
        save(&path, 7, &commits);
        assert_eq!(load(&path, 7), Some(commits));
        assert_eq!(load(&path, 8), None, "stale fingerprint must not be used");
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(load(&path, 7), None, "corrupt cache is ignored");
        let _ = std::fs::remove_dir_all(dir);
    }
}
