// SPDX-License-Identifier: Apache-2.0
//! History operations (Phase 4): cherry-pick, revert, reset and tags. All journaled and undoable;
//! a hard reset first saves uncommitted work with `git stash create` (NFR-5).

use ak_gitcli::GitError;

use super::journal::JournalEntry;
use super::lock::{self, Lock};
use super::{new_id, now, Engine, OpError, OpOutcome, SNAPSHOT_PREFIX};

fn moved() -> OpError {
    OpError::new("REF_MOVED", "The branch changed since that operation, so it cannot be undone or redone safely.", Some("Undo the newer operations first."))
}

impl Engine {
    fn commit_sha(&self, rev: &str) -> Result<String, OpError> {
        let spec = format!("{rev}^{{commit}}");
        self.git
            .run(&self.root, ["rev-parse", "-q", "--verify", spec.as_str()])
            .map(|o| String::from_utf8_lossy(&o).trim().to_owned())
            .map_err(|_| OpError::new("INVALID_REF", "That commit does not exist.", None))
    }

    fn worktree_dirty(&self) -> bool {
        matches!(self.git.run(&self.root, ["diff", "--quiet", "HEAD"]), Err(GitError::Failed { code: Some(1), .. }))
    }

    /// Cherry-pick (`kind` = "cherry_pick") or revert ("revert") one commit onto the current branch.
    pub(super) fn pick_like(&self, kind: &str, rev: &str) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.in_progress().is_some() {
            return Err(Self::op_in_progress());
        }
        let (Some(before), Some(branch)) = (self.head(), self.branch()) else {
            return Err(OpError::new("INVALID_REF", "Check out a branch with at least one commit first.", None));
        };
        let sha = self.commit_sha(rev)?;
        let parents = self.git.run(&self.root, ["rev-list", "--parents", "-n", "1", sha.as_str()])?;
        let is_merge = String::from_utf8_lossy(&parents).split_whitespace().count() > 2;
        let sub = if kind == "cherry_pick" { "cherry-pick" } else { "revert" };
        let mut args = vec![sub];
        if kind == "revert" {
            args.push("--no-edit");
        }
        if is_merge {
            // Relative to the first parent, the usual meaning of "the changes of this merge".
            args.extend(["-m", "1"]);
        }
        args.push(sha.as_str());
        let (code, out, err) = self.git.run_status(&self.root, &args)?;
        if code != Some(0) {
            let text = format!("{out}\n{err}");
            if text.contains("is now empty") || text.contains("nothing to commit") {
                let _ = self.git.run(&self.root, [sub, "--abort"]);
                return Err(OpError::new("NOTHING_TO_COMMIT", "Those changes are already on this branch.", None));
            }
            if self.in_progress().is_some() {
                return Ok(OpOutcome { head: self.head(), conflicted: true, ..Default::default() });
            }
            return Err(OpError::new("GIT_FAILED", &format!("git {sub} failed."), Some("Commit or stash your local changes first.")).with_detail(text.trim().to_owned()));
        }
        let after = self.head();
        let id = new_id();
        if let Some(a) = &after {
            self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}-redo").as_str(), a.as_str()])?;
        }
        let verb = if kind == "cherry_pick" { "Cherry-pick" } else { "Revert" };
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: kind.into(),
            summary: format!("{verb} {} onto {branch}", &sha[..7.min(sha.len())]),
            branch: Some(branch),
            head_before: Some(before),
            head_after: after.clone(),
            commands: vec![format!("git {}", args.join(" "))],
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head: after, ..Default::default() })
    }

    /// `git reset --<mode>`; a hard reset keeps a snapshot of the discarded changes for Undo.
    pub(super) fn reset(&self, rev: &str, mode: &str) -> Result<OpOutcome, OpError> {
        if !matches!(mode, "soft" | "mixed" | "hard") {
            return Err(OpError::new("INVALID_REF", "Unknown reset mode.", None));
        }
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let before = self.head().ok_or_else(|| OpError::new("INVALID_REF", "There is no commit to reset yet.", None))?;
        let target = self.commit_sha(rev)?;
        let id = new_id();
        self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}").as_str(), before.as_str()])?;
        let mut wip = String::new();
        if mode == "hard" {
            let out = self.git.run(&self.root, ["stash", "create"])?;
            wip = String::from_utf8_lossy(&out).trim().to_owned();
            if !wip.is_empty() {
                self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}-wip").as_str(), wip.as_str()])?;
            }
        }
        let flag = format!("--{mode}");
        self.git.run(&self.root, ["reset", "-q", flag.as_str(), target.as_str()])?;
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: "reset".into(),
            summary: format!("Reset ({mode}) to {}", &target[..7.min(target.len())]),
            branch: self.branch(),
            head_before: Some(before),
            head_after: self.head(),
            snapshot_ref: Some(format!("{SNAPSHOT_PREFIX}{id}")),
            commands: vec![format!("git reset {flag} {target}")],
            args: vec![mode.into(), wip],
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head: self.head(), ..Default::default() })
    }

    pub(super) fn apply_reset_entry(&self, e: &JournalEntry, forward: bool) -> Result<(), OpError> {
        let (here, go) = if forward { (&e.head_before, &e.head_after) } else { (&e.head_after, &e.head_before) };
        if self.head() != *here || self.branch() != e.branch {
            return Err(moved());
        }
        let (mode, wip) = (e.args[0].as_str(), e.args[1].as_str());
        if mode == "hard" && self.worktree_dirty() {
            return Err(OpError::new("WORKTREE_CHANGED", "You have uncommitted changes that this would overwrite.", Some("Commit or stash them first.")));
        }
        let flag = format!("--{mode}");
        self.git.run(&self.root, ["reset", "-q", flag.as_str(), go.as_deref().unwrap_or_default()])?;
        if !forward && !wip.is_empty() && self.git.run(&self.root, ["stash", "apply", "-q", "--index", wip]).is_err() {
            self.git.run(&self.root, ["stash", "apply", "-q", wip])?;
        }
        Ok(())
    }

    fn tag_object(&self, name: &str) -> Option<String> {
        let r = format!("refs/tags/{name}");
        self.git
            .run(&self.root, ["rev-parse", "-q", "--verify", r.as_str()])
            .ok()
            .map(|o| String::from_utf8_lossy(&o).trim().to_owned())
    }

    pub(super) fn tag_create(&self, name: &str, at: &str, message: Option<&str>) -> Result<OpOutcome, OpError> {
        let r = format!("refs/tags/{name}");
        if name.is_empty() || name.starts_with('-') || self.git.run(&self.root, ["check-ref-format", r.as_str()]).is_err() {
            return Err(OpError::new("INVALID_BRANCH_NAME", "That is not a valid tag name.", None));
        }
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.tag_object(name).is_some() {
            return Err(OpError::new("TAG_EXISTS", "A tag with that name already exists.", None));
        }
        let sha = self.commit_sha(at)?;
        match message.filter(|m| !m.trim().is_empty()) {
            Some(m) => self.git.run_with_input(&self.root, ["tag", "-a", "-F", "-", name, sha.as_str()], m.as_bytes())?,
            None => self.git.run(&self.root, ["tag", name, sha.as_str()])?,
        };
        let obj = self.tag_object(name).unwrap_or_default();
        let id = new_id();
        // Keeps an annotated tag object alive for Redo after an Undo.
        self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}").as_str(), obj.as_str()])?;
        self.record_tag(id, "tag_create", format!("Create tag {name}"), name, obj, format!("git tag {name} {sha}"))
    }

    pub(super) fn tag_delete(&self, name: &str) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let obj = self.tag_object(name).ok_or_else(|| OpError::new("INVALID_REF", "That tag does not exist.", None))?;
        let id = new_id();
        self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}").as_str(), obj.as_str()])?;
        self.git.run(&self.root, ["tag", "-d", name])?;
        self.record_tag(id, "tag_delete", format!("Delete tag {name}"), name, obj, format!("git tag -d {name}"))
    }

    fn record_tag(&self, id: String, kind: &str, summary: String, name: &str, obj: String, cmd: String) -> Result<OpOutcome, OpError> {
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: kind.into(),
            summary,
            branch: self.branch(),
            head_before: self.head(),
            head_after: self.head(),
            snapshot_ref: Some(format!("{SNAPSHOT_PREFIX}{id}")),
            commands: vec![cmd],
            args: vec![name.into(), obj],
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head: self.head(), ..Default::default() })
    }

    /// Undo/redo of tag create/delete: `add` puts the tag back on its original object.
    pub(super) fn apply_tag_entry(&self, e: &JournalEntry, add: bool) -> Result<(), OpError> {
        let (name, obj) = (&e.args[0], &e.args[1]);
        let r = format!("refs/tags/{name}");
        let current = self.tag_object(name);
        if add {
            if current.is_some() {
                return Err(moved());
            }
            self.git.run(&self.root, ["update-ref", r.as_str(), obj.as_str()])?;
        } else {
            if current.as_deref() != Some(obj.as_str()) {
                return Err(moved());
            }
            self.git.run(&self.root, ["update-ref", "-d", r.as_str()])?;
        }
        Ok(())
    }
}
