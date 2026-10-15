// SPDX-License-Identifier: Apache-2.0
//! Branch operations (Phase 3): create, rename, delete and checkout. Each one is journaled
//! with the arguments needed to undo and redo it (`JournalEntry::args`).

use ak_gitcli::GitError;

use super::journal::JournalEntry;
use super::lock::{self, Lock};
use super::{new_id, now, Engine, OpError, OpOutcome};

pub(super) fn is_ref_kind(kind: &str) -> bool {
    matches!(kind, "branch_create" | "branch_delete" | "branch_rename" | "checkout" | "merge" | "stash_push" | "stash_drop")
}

fn refs_moved() -> OpError {
    OpError::new(
        "REF_MOVED",
        "The branch changed since that operation, so it cannot be undone or redone safely.",
        Some("Undo the newer operations first."),
    )
}

impl Engine {
    fn valid_branch(&self, name: &str) -> Result<(), OpError> {
        let ok = !name.is_empty()
            && !name.starts_with('-')
            && self.git.run(&self.root, ["check-ref-format", "--branch", name]).is_ok();
        ok.then_some(()).ok_or_else(|| {
            OpError::new("INVALID_BRANCH_NAME", "That is not a valid branch name.", Some("Avoid spaces, \"..\", \"~\", \"^\", \":\" and a leading \"-\"."))
        })
    }

    fn tip(&self, branch: &str) -> Option<String> {
        let r = format!("refs/heads/{branch}");
        self.git
            .run(&self.root, ["rev-parse", "-q", "--verify", r.as_str()])
            .ok()
            .map(|o| String::from_utf8_lossy(&o).trim().to_owned())
    }

    fn record(&self, kind: &str, summary: String, args: Vec<String>, before: Option<String>, commands: Vec<String>) -> Result<OpOutcome, OpError> {
        let id = new_id();
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: kind.to_owned(),
            summary,
            branch: self.branch(),
            head_before: before,
            head_after: self.head(),
            commands,
            args,
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head: self.head(), ..Default::default() })
    }

    pub(super) fn branch_create(&self, name: &str, at: Option<&str>) -> Result<OpOutcome, OpError> {
        self.valid_branch(name)?;
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.tip(name).is_some() {
            return Err(OpError::new("BRANCH_EXISTS", "A branch with that name already exists.", None));
        }
        let at = at.unwrap_or("HEAD");
        let rev = format!("{at}^{{commit}}");
        let sha = self
            .git
            .run(&self.root, ["rev-parse", "-q", "--verify", rev.as_str()])
            .map(|o| String::from_utf8_lossy(&o).trim().to_owned())
            .map_err(|_| OpError::new("INVALID_REF", "That commit does not exist.", None))?;
        self.git.run(&self.root, ["branch", name, sha.as_str()])?;
        self.record("branch_create", format!("Create branch {name}"), vec![name.into(), sha.clone()], self.head(), vec![format!("git branch {name} {sha}")])
    }

    pub(super) fn branch_rename(&self, old: &str, new: &str) -> Result<OpOutcome, OpError> {
        self.valid_branch(new)?;
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.tip(old).is_none() {
            return Err(OpError::new("INVALID_REF", "That branch does not exist.", None));
        }
        if self.tip(new).is_some() {
            return Err(OpError::new("BRANCH_EXISTS", "A branch with that name already exists.", None));
        }
        self.git.run(&self.root, ["branch", "-m", old, new])?;
        self.record("branch_rename", format!("Rename {old} to {new}"), vec![old.into(), new.into()], self.head(), vec![format!("git branch -m {old} {new}")])
    }

    pub(super) fn branch_delete(&self, name: &str, force: bool) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let sha = self.tip(name).ok_or_else(|| OpError::new("INVALID_REF", "That branch does not exist.", None))?;
        if self.branch().as_deref() == Some(name) {
            return Err(OpError::new("BRANCH_IS_CURRENT", "You cannot delete the branch you are on.", Some("Check out another branch first.")));
        }
        let flag = if force { "-D" } else { "-d" };
        match self.git.run(&self.root, ["branch", flag, name]) {
            Ok(_) => {}
            Err(GitError::Failed { stderr, .. }) if stderr.contains("not fully merged") => {
                return Err(OpError::new("BRANCH_NOT_MERGED", "That branch has commits that are not merged anywhere else.", Some("Delete it anyway; Undo brings it back.")));
            }
            Err(e) => return Err(e.into()),
        }
        self.record("branch_delete", format!("Delete branch {name}"), vec![name.into(), sha], self.head(), vec![format!("git branch {flag} {name}")])
    }

    pub(super) fn checkout(&self, target: &str, is_branch: bool) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let (from_branch, from_sha) = (self.branch(), self.head());
        if is_branch {
            if self.tip(target).is_none() {
                return Err(OpError::new("INVALID_REF", "That branch does not exist.", None));
            }
            self.git.run(&self.root, ["checkout", "-q", target, "--"])?;
        } else {
            self.git.run(&self.root, ["checkout", "-q", "--detach", target])?;
        }
        let to_sha = self.head().unwrap_or_default();
        // from = (name or sha, is_branch), to = (target or sha, is_branch)
        let from = from_branch.clone().or_else(|| from_sha.clone()).unwrap_or_default();
        let to = if is_branch { target.to_owned() } else { to_sha.clone() };
        let args = vec![from, from_branch.is_some().to_string(), to, is_branch.to_string()];
        let cmd = if is_branch { format!("git checkout {target}") } else { format!("git checkout --detach {target}") };
        self.record("checkout", format!("Check out {target}"), args, from_sha, vec![cmd])
    }

    /// Replays an entry forwards (redo) or backwards (undo) after checking the repository
    /// is exactly in the state the other direction left it in.
    pub(super) fn apply_ref_entry(&self, e: &JournalEntry, forward: bool) -> Result<(), OpError> {
        let a = &e.args;
        let r = &self.root;
        let ref_of = |b: &str| format!("refs/heads/{b}");
        match (e.kind.as_str(), forward) {
            ("branch_create", true) | ("branch_delete", false) => {
                if self.tip(&a[0]).is_some() {
                    return Err(refs_moved());
                }
                self.git.run(r, ["branch", a[0].as_str(), a[1].as_str()])?;
            }
            ("branch_create", false) | ("branch_delete", true) => {
                if self.tip(&a[0]).as_deref() != Some(a[1].as_str()) || self.branch().as_deref() == Some(a[0].as_str()) {
                    return Err(refs_moved());
                }
                self.git.run(r, ["update-ref", "-d", ref_of(&a[0]).as_str()])?;
            }
            ("branch_rename", _) => {
                let (from, to) = if forward { (&a[0], &a[1]) } else { (&a[1], &a[0]) };
                if self.tip(from).is_none() || self.tip(to).is_some() {
                    return Err(refs_moved());
                }
                self.git.run(r, ["branch", "-m", from.as_str(), to.as_str()])?;
            }
            ("checkout", _) => {
                // `here` is where the repository must be now; `go` is where to move.
                let (go, go_is_branch, here, here_is_branch) = if forward { (&a[2], &a[3], &a[0], &a[1]) } else { (&a[0], &a[1], &a[2], &a[3]) };
                let at_here = if here_is_branch == "true" { self.branch().as_deref() == Some(here.as_str()) } else { self.branch().is_none() && self.head().as_deref() == Some(here.as_str()) };
                if !at_here {
                    return Err(refs_moved());
                }
                if go_is_branch == "true" {
                    self.git.run(r, ["checkout", "-q", go.as_str(), "--"])?;
                } else {
                    self.git.run(r, ["checkout", "-q", "--detach", go.as_str()])?;
                }
            }
            ("merge", _) => self.apply_merge_entry(e, forward)?,
            ("stash_push", false) => self.undo_stash_push(e)?,
            ("stash_drop", false) => self.undo_stash_drop(e)?,
            ("stash_drop", true) => self.redo_stash_drop(e)?,
            _ => return Err(OpError::new("UNDO_NOT_SUPPORTED", "This operation cannot be undone.", None)),
        }
        Ok(())
    }
}
