// SPDX-License-Identifier: Apache-2.0
//! Merge (Phase 3): conflict preview, merge, abort and per-file conflict resolution.

use serde::Serialize;

use super::journal::JournalEntry;
use super::lock::{self, Lock};
use super::{literal, new_id, now, Engine, OpError, OpOutcome, SNAPSHOT_PREFIX};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MergePreview {
    pub up_to_date: bool,
    pub fast_forward: bool,
    /// Files that will conflict; `None` when this Git cannot predict it (before 2.38).
    pub conflicts: Option<Vec<String>>,
}

impl Engine {
    pub fn merge_in_progress(&self) -> bool {
        self.git_dir.join("MERGE_HEAD").exists()
    }

    fn is_ancestor(&self, a: &str, b: &str) -> bool {
        self.git.run_status(&self.root, ["merge-base", "--is-ancestor", a, b]).map(|(c, ..)| c == Some(0)).unwrap_or(false)
    }

    /// What merging `source` into HEAD would do, without touching the repository.
    pub fn merge_preview(&self, source: &str) -> Result<MergePreview, OpError> {
        let rev = format!("{source}^{{commit}}");
        if self.git.run(&self.root, ["rev-parse", "-q", "--verify", rev.as_str()]).is_err() {
            return Err(OpError::new("INVALID_REF", "That branch or commit does not exist.", None));
        }
        if self.is_ancestor(source, "HEAD") {
            return Ok(MergePreview { up_to_date: true, ..Default::default() });
        }
        if self.is_ancestor("HEAD", source) {
            return Ok(MergePreview { fast_forward: true, conflicts: Some(vec![]), ..Default::default() });
        }
        if !self.git.caps().merge_tree_write_tree {
            return Ok(MergePreview::default());
        }
        let (code, out, err) = self.git.run_status(&self.root, ["merge-tree", "--write-tree", "--name-only", "HEAD", source])?;
        let conflicts = match code {
            Some(0) => vec![],
            // Line 1 is the result tree; file names follow until the first blank line.
            Some(1) => out.lines().skip(1).take_while(|l| !l.is_empty()).map(str::to_owned).collect(),
            _ => return Err(OpError::new("GIT_FAILED", "git merge-tree failed.", None).with_detail(err)),
        };
        Ok(MergePreview { conflicts: Some(conflicts), ..Default::default() })
    }

    pub(super) fn merge(&self, source: &str) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.merge_in_progress() {
            return Err(OpError::new("MERGE_IN_PROGRESS", "A merge is already in progress.", Some("Finish it or abort it first.")));
        }
        let (Some(before), Some(branch)) = (self.head(), self.branch()) else {
            return Err(OpError::new("INVALID_REF", "Merging needs a branch with at least one commit checked out.", None));
        };
        let preview = self.merge_preview(source)?;
        if preview.up_to_date {
            return Err(OpError::new("ALREADY_UP_TO_DATE", "That branch is already merged.", None));
        }
        let (code, out, err) = self.git.run_status(&self.root, ["merge", "--no-edit", source])?;
        if code != Some(0) {
            if self.merge_in_progress() {
                return Ok(OpOutcome { head: self.head(), conflicted: true, ..Default::default() });
            }
            return Err(OpError::new("GIT_FAILED", "git merge failed.", None).with_detail(if err.is_empty() { out } else { err }));
        }
        let after = self.head();
        let id = new_id();
        if let Some(a) = &after {
            // Keep the result reachable so Redo can bring it back after an Undo.
            self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}-redo").as_str(), a.as_str()])?;
        }
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: "merge".into(),
            summary: format!("Merge {source} into {branch}"),
            branch: Some(branch),
            head_before: Some(before),
            head_after: after.clone(),
            commands: vec![format!("git merge --no-edit {source}")],
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head: after, ..Default::default() })
    }

    /// Undo/redo of a merge: move the branch with `reset --merge`, which refuses to
    /// clobber local changes. Only allowed while the branch is where the merge left it.
    pub(super) fn apply_merge_entry(&self, e: &JournalEntry, forward: bool) -> Result<(), OpError> {
        let (here, go) = if forward { (&e.head_before, &e.head_after) } else { (&e.head_after, &e.head_before) };
        if self.head() != *here || self.branch() != e.branch {
            return Err(OpError::new("REF_MOVED", "The branch changed since that merge, so it cannot be undone or redone safely.", Some("Undo the newer operations first.")));
        }
        let go = go.as_deref().unwrap_or_default();
        self.git.run(&self.root, ["reset", "-q", "--merge", go])?;
        Ok(())
    }

    pub(super) fn merge_abort(&self) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if !self.merge_in_progress() {
            return Err(OpError::new("NO_MERGE_IN_PROGRESS", "There is no merge to abort.", None));
        }
        self.git.run(&self.root, ["merge", "--abort"])?;
        Ok(OpOutcome { head: self.head(), ..Default::default() })
    }

    pub(super) fn resolve_conflict(&self, path: &str, theirs: bool) -> Result<OpOutcome, OpError> {
        super::check_rel(path)?;
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let spec = literal(path);
        let side = if theirs { "--theirs" } else { "--ours" };
        self.git.run(&self.root, ["checkout", side, "--", spec.as_str()])?;
        self.git.run(&self.root, ["add", "--", spec.as_str()])?;
        Ok(OpOutcome::default())
    }
}
