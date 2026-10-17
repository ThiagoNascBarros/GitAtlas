// SPDX-License-Identifier: Apache-2.0
//! Branch operations (Phase 3): create, rename, delete and checkout. Each one is journaled
//! with the arguments needed to undo and redo it (`JournalEntry::args`).

use ak_gitcli::GitError;

use super::journal::JournalEntry;
use super::lock::{self, Lock};
use super::{new_id, now, Engine, OpError, OpOutcome};

pub(super) fn is_ref_kind(kind: &str) -> bool {
    matches!(kind, "shared_stash_branch" | "branch_create" | "branch_delete" | "branch_rename" | "checkout" | "merge" | "stash_push" | "stash_drop" | "cherry_pick" | "revert" | "reset" | "tag_create" | "tag_delete" | "rebase")
}

/// Shared Stash entries that are history only: they can never be undone, and they do not clear Redo.
pub(super) fn is_passive_kind(kind: &str) -> bool {
    kind.starts_with("shared_stash_") && kind != "shared_stash_branch"
}

fn ref_of(branch: &str) -> String {
    format!("refs/heads/{branch}")
}

fn refs_moved() -> OpError {
    OpError::new(
        "REF_MOVED",
        "The branch changed since that operation, so it cannot be undone or redone safely.",
        Some("Undo the newer operations first."),
    )
}

impl Engine {
    pub(super) fn valid_branch(&self, name: &str) -> Result<(), OpError> {
        let ok = !name.is_empty()
            && !name.starts_with('-')
            && self.git.run(&self.root, ["check-ref-format", "--branch", name]).is_ok();
        ok.then_some(()).ok_or_else(|| {
            OpError::new("INVALID_BRANCH_NAME", "That is not a valid branch name.", Some("Avoid spaces, \"..\", \"~\", \"^\", \":\" and a leading \"-\"."))
        })
    }

    pub(super) fn tip(&self, branch: &str) -> Option<String> {
        let r = format!("refs/heads/{branch}");
        self.git
            .run(&self.root, ["rev-parse", "-q", "--verify", r.as_str()])
            .ok()
            .map(|o| String::from_utf8_lossy(&o).trim().to_owned())
    }

    pub(super) fn record(&self, kind: &str, summary: String, args: Vec<String>, before: Option<String>, commands: Vec<String>) -> Result<OpOutcome, OpError> {
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
        let sha = self.commit_sha(at.unwrap_or("HEAD"))?;
        self.git.run(&self.root, ["branch", name, sha.as_str()])?;
        self.record("branch_create", format!("Create branch {name}"), vec![name.into(), sha.clone()], self.head(), vec![format!("git branch {name} {sha}")])
    }

    /// New branch at `base` (default HEAD). With `checkout`, it is switched to as well, and
    /// both steps are one journal entry: Undo returns to the previous branch and deletes it.
    pub(super) fn branch_create_from(&self, name: &str, base: Option<&str>, checkout: bool) -> Result<OpOutcome, OpError> {
        if !checkout {
            return self.branch_create(name, base);
        }
        self.valid_branch(name)?;
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.tip(name).is_some() {
            return Err(OpError::new("BRANCH_EXISTS", "A branch with that name already exists.", None));
        }
        let before = self.head().ok_or_else(|| OpError::new("INVALID_REF", "Make a first commit before switching to a new branch.", None))?;
        let sha = self.commit_sha(base.unwrap_or("HEAD"))?;
        // Where to go back to: the current branch, or the detached commit we are on.
        let (from, from_is_branch) = match self.branch() {
            Some(b) => (b, true),
            None => (before.clone(), false),
        };
        self.git.run(&self.root, ["branch", name, sha.as_str()])?;
        if let Err(e) = self.git.run(&self.root, ["checkout", "-q", name, "--"]) {
            // Nothing else changed yet: remove the new ref so the repository is as it was.
            let _ = self.git.run(&self.root, ["update-ref", "-d", ref_of(name).as_str()]);
            return Err(e.into());
        }
        let args = vec![name.into(), sha.clone(), from.clone(), from_is_branch.to_string()];
        let cmd = vec![format!("git branch {name} {sha}"), format!("git checkout {name}")];
        self.record("branch_create", format!("Create branch {name} and check it out"), args, Some(before), cmd)
    }

    /// True when HEAD is where `target` says it is (a branch, or a detached commit).
    pub(super) fn is_at(&self, target: &str, is_branch: bool) -> bool {
        if is_branch {
            self.branch().as_deref() == Some(target)
        } else {
            self.branch().is_none() && self.head().as_deref() == Some(target)
        }
    }

    /// Moves HEAD to a branch, or detaches it at a commit.
    pub(super) fn switch_to(&self, target: &str, is_branch: bool) -> Result<(), OpError> {
        if is_branch {
            self.git.run(&self.root, ["checkout", "-q", target, "--"])?;
        } else {
            self.git.run(&self.root, ["checkout", "-q", "--detach", target])?;
        }
        Ok(())
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
        match (e.kind.as_str(), forward) {
            // "Create and check out" entries: [name, sha, from, from_is_branch].
            ("branch_create", false) if a.len() == 4 => {
                if self.branch().as_deref() != Some(a[0].as_str()) || self.tip(&a[0]).as_deref() != Some(a[1].as_str()) {
                    return Err(refs_moved());
                }
                self.switch_to(&a[2], a[3] == "true")?;
                self.git.run(r, ["update-ref", "-d", ref_of(&a[0]).as_str()])?;
            }
            ("branch_create", true) if a.len() == 4 => {
                if self.tip(&a[0]).is_some() || !self.is_at(&a[2], a[3] == "true") {
                    return Err(refs_moved());
                }
                self.git.run(r, ["branch", a[0].as_str(), a[1].as_str()])?;
                self.switch_to(&a[0], true)?;
            }
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
            ("merge" | "cherry_pick" | "revert" | "rebase", _) => self.apply_merge_entry(e, forward)?,
            ("reset", _) => self.apply_reset_entry(e, forward)?,
            ("tag_create", f) => self.apply_tag_entry(e, f)?,
            ("tag_delete", f) => self.apply_tag_entry(e, !f)?,
            ("shared_stash_branch", forward) => self.apply_shared_branch_entry(e, forward)?,
            ("stash_push", false) => self.undo_stash_push(e)?,
            ("stash_drop", false) => self.undo_stash_drop(e)?,
            ("stash_drop", true) => self.redo_stash_drop(e)?,
            _ => return Err(OpError::new("UNDO_NOT_SUPPORTED", "This operation cannot be undone.", None)),
        }
        Ok(())
    }
}
