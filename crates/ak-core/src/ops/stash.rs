// SPDX-License-Identifier: Apache-2.0
//! Stash (Phase 3): list, push, apply, pop, drop. Push and drop are journaled and undoable;
//! apply and pop are not (their result is ordinary working-tree changes).

use serde::Serialize;

use super::journal::JournalEntry;
use super::lock::{self, Lock};
use super::{new_id, now, Engine, OpError, OpOutcome};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StashInfo {
    pub index: usize,
    pub sha: String,
    /// Commit the stash was taken on (its first parent): where the graph shows it.
    pub base: String,
    pub message: String,
    pub time: i64,
}

fn bad_index() -> OpError {
    OpError::new("INVALID_STASH", "That stash no longer exists.", Some("Refresh and try again."))
}

impl Engine {
    pub fn stashes(&self) -> Result<Vec<StashInfo>, OpError> {
        let out = self.git.run(&self.root, ["stash", "list", "--format=%H%x00%P%x00%gs%x00%ct"])?;
        Ok(String::from_utf8_lossy(&out)
            .lines()
            .enumerate()
            .filter_map(|(index, l)| {
                let mut p = l.split('\0');
                let sha = p.next()?.into();
                let base = p.next()?.split(' ').next()?.into();
                Some(StashInfo { index, sha, base, message: p.next()?.into(), time: p.next()?.parse().ok()? })
            })
            .collect())
    }

    fn stash_at(&self, index: usize) -> Result<StashInfo, OpError> {
        self.stashes()?.into_iter().find(|s| s.index == index).ok_or_else(bad_index)
    }

    fn top_stash(&self) -> Option<String> {
        self.git
            .run(&self.root, ["rev-parse", "-q", "--verify", "refs/stash"])
            .ok()
            .map(|o| String::from_utf8_lossy(&o).trim().to_owned())
    }

    pub(super) fn stash_push(&self, message: Option<&str>, untracked: bool) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let before = self.top_stash();
        let mut args = vec!["stash", "push", "-q"];
        if untracked {
            args.push("-u");
        }
        if let Some(m) = message.filter(|m| !m.trim().is_empty()) {
            args.extend(["-m", m]);
        }
        self.git.run(&self.root, args)?;
        let after = self.top_stash();
        let sha = match after {
            Some(s) if Some(&s) != before.as_ref() => s,
            _ => return Err(OpError::new("NOTHING_TO_STASH", "There are no local changes to stash.", None)),
        };
        let id = new_id();
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: "stash_push".into(),
            summary: message.filter(|m| !m.trim().is_empty()).map_or_else(|| "Stash changes".to_owned(), |m| format!("Stash: {m}")),
            branch: self.branch(),
            head_before: self.head(),
            head_after: self.head(),
            commands: vec![format!("git stash push{}", if untracked { " -u" } else { "" })],
            args: vec![sha, if untracked { "-u" } else { "" }.into()],
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head: self.head(), ..Default::default() })
    }

    pub(super) fn stash_apply(&self, index: usize, pop: bool) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        self.stash_at(index)?;
        let name = format!("stash@{{{index}}}");
        let (code, out, err) = self.git.run_status(&self.root, ["stash", if pop { "pop" } else { "apply" }, name.as_str()])?;
        if code == Some(0) {
            return Ok(OpOutcome { head: self.head(), ..Default::default() });
        }
        // Conflicts leave the changes in the tree (and the stash in place) for the user to resolve.
        if out.contains("CONFLICT") {
            return Ok(OpOutcome { head: self.head(), conflicted: true, ..Default::default() });
        }
        let detail = if err.is_empty() { out } else { err };
        Err(OpError::new("GIT_FAILED", "git stash failed.", Some("Commit or stash your current changes that overlap, then try again.")).with_detail(detail))
    }

    pub(super) fn stash_drop(&self, index: usize) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let s = self.stash_at(index)?;
        self.git.run(&self.root, ["stash", "drop", "-q", format!("stash@{{{index}}}").as_str()])?;
        let id = new_id();
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: "stash_drop".into(),
            summary: format!("Drop stash: {}", s.message),
            branch: self.branch(),
            head_before: self.head(),
            head_after: self.head(),
            commands: vec![format!("git stash drop stash@{{{index}}}")],
            args: vec![s.sha, s.message],
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head: self.head(), ..Default::default() })
    }

    /// Undo a push: put the changes back and drop that stash entry.
    pub(super) fn undo_stash_push(&self, e: &JournalEntry) -> Result<(), OpError> {
        let s = self.stashes()?.into_iter().find(|s| s.sha == e.args[0]).ok_or_else(bad_index)?;
        let name = format!("stash@{{{}}}", s.index);
        let (code, out, err) = self.git.run_status(&self.root, ["stash", "pop", "--index", name.as_str()])?;
        if code != Some(0) {
            let detail = if err.is_empty() { out } else { err };
            return Err(OpError::new("GIT_FAILED", "The stash could not be put back.", Some("Commit or stash your current changes that overlap, then try again.")).with_detail(detail));
        }
        Ok(())
    }

    pub(super) fn undo_stash_drop(&self, e: &JournalEntry) -> Result<(), OpError> {
        if self.stashes()?.iter().any(|s| s.sha == e.args[0]) {
            return Err(OpError::new("REF_MOVED", "That stash is already back.", None));
        }
        self.git.run(&self.root, ["stash", "store", "-m", e.args[1].as_str(), e.args[0].as_str()])?;
        Ok(())
    }

    pub(super) fn redo_stash_drop(&self, e: &JournalEntry) -> Result<(), OpError> {
        let s = self.stashes()?.into_iter().find(|s| s.sha == e.args[0]).ok_or_else(bad_index)?;
        self.git.run(&self.root, ["stash", "drop", "-q", format!("stash@{{{}}}", s.index).as_str()])?;
        Ok(())
    }
}
