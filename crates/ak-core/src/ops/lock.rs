// SPDX-License-Identifier: Apache-2.0
//! Named per-repository locks (SDD §1.4). An operation declares the resources
//! it touches; if any is already held the operation fails fast with `REPO_BUSY`
//! instead of racing another operation on the same index or config.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lock {
    Index,
    Config,
    RepoSettings,
    /// Shared Stash network operations (publish, update, unshare, reshare, fetch, probe).
    Remote,
}

type Table = Mutex<HashMap<PathBuf, HashSet<Lock>>>;

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(Default::default)
}

/// Releases its locks when dropped.
#[derive(Debug)]
pub struct LockGuard {
    repo: PathBuf,
    locks: Vec<Lock>,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let mut t = table().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(held) = t.get_mut(&self.repo) {
            for l in &self.locks {
                held.remove(l);
            }
            if held.is_empty() {
                t.remove(&self.repo);
            }
        }
    }
}

/// Takes all `wanted` locks for `repo` or none. `Err` carries the lock that is busy.
pub fn try_acquire(repo: &Path, wanted: &[Lock]) -> Result<LockGuard, Lock> {
    let mut t = table().lock().unwrap_or_else(|e| e.into_inner());
    let held = t.entry(repo.to_owned()).or_default();
    if let Some(busy) = wanted.iter().find(|l| held.contains(l)) {
        let busy = *busy;
        if held.is_empty() {
            t.remove(repo);
        }
        return Err(busy);
    }
    held.extend(wanted.iter().copied());
    Ok(LockGuard { repo: repo.to_owned(), locks: wanted.to_vec() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusive_per_repo_and_released_on_drop() {
        let a = Path::new("/tmp/ak-lock-a");
        let b = Path::new("/tmp/ak-lock-b");
        let g = try_acquire(a, &[Lock::Index]).unwrap();
        assert_eq!(try_acquire(a, &[Lock::Index, Lock::Config]).unwrap_err(), Lock::Index);
        assert!(try_acquire(a, &[Lock::Config]).is_ok(), "different lock is free");
        assert!(try_acquire(b, &[Lock::Index]).is_ok(), "other repo is independent");
        drop(g);
        assert!(try_acquire(a, &[Lock::Index]).is_ok());
    }

    #[test]
    fn remote_operations_conflict_with_each_other_but_not_with_index_ones() {
        let r = Path::new("/tmp/ak-lock-d");
        let g = try_acquire(r, &[Lock::Remote]).unwrap();
        assert_eq!(try_acquire(r, &[Lock::Remote]).unwrap_err(), Lock::Remote, "two network operations conflict");
        assert!(try_acquire(r, &[Lock::Index]).is_ok(), "an Index operation is not blocked by a network one");
        drop(g);
        let _idx = try_acquire(r, &[Lock::Index]).unwrap();
        assert!(try_acquire(r, &[Lock::Remote]).is_ok(), "and the other way round");
    }

    #[test]
    fn failed_acquire_takes_nothing() {
        let r = Path::new("/tmp/ak-lock-c");
        let _g = try_acquire(r, &[Lock::Index]).unwrap();
        assert!(try_acquire(r, &[Lock::Config, Lock::Index]).is_err());
        assert!(try_acquire(r, &[Lock::Config]).is_ok(), "Config must not be left held");
    }
}
