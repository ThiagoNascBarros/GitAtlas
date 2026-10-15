// SPDX-License-Identifier: Apache-2.0
//! Remotes (Phase 4): fetch, pull and push. Credentials come from the user's Git setup
//! (credential helpers, ssh-agent); when Git needs to ask, it runs the askpass program
//! given in `env` (see the shell's askpass bridge). Pull is journaled like a merge.

use super::journal::JournalEntry;
use super::lock::{self, Lock};
use super::{new_id, now, Engine, OpError, OpOutcome, SNAPSHOT_PREFIX};

type Env = [(String, String)];

/// Maps network failures to stable codes; everything else is GIT_FAILED with the raw text.
fn net_error(what: &str, stderr: String) -> OpError {
    let low = stderr.to_lowercase();
    let (code, msg, fix) = if low.contains("authentication failed")
        || low.contains("permission denied")
        || low.contains("could not read username")
        || low.contains("could not read password")
        || low.contains("invalid username or password")
    {
        ("AUTH_FAILED", "The remote refused your credentials.", "Check your username/password or SSH key, then try again.")
    } else if low.contains("[rejected]") || low.contains("non-fast-forward") || low.contains("fetch first") {
        ("PUSH_REJECTED", "The remote has commits you do not have yet.", "Pull first, then push again.")
    } else if low.contains("could not resolve host") || low.contains("unable to access") || low.contains("connection") {
        ("NETWORK_FAILED", "Could not reach the remote.", "Check your network connection and the remote URL.")
    } else if low.contains("no such remote") || low.contains("does not appear to be a git repository") {
        ("NO_REMOTE", "That remote does not exist or is not a Git repository.", "Check the remote name and URL.")
    } else {
        return OpError::new("GIT_FAILED", &format!("git {what} failed."), None).with_detail(stderr);
    };
    OpError::new(code, msg, Some(fix)).with_detail(stderr)
}

impl Engine {
    fn remotes(&self) -> Result<Vec<String>, OpError> {
        let out = self.git.run(&self.root, ["remote"])?;
        Ok(String::from_utf8_lossy(&out).lines().map(str::to_owned).collect())
    }

    fn upstream(&self) -> Option<String> {
        self.git
            .run(&self.root, ["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"])
            .ok()
            .map(|o| String::from_utf8_lossy(&o).trim().to_owned())
            .filter(|s| !s.is_empty())
    }

    /// The remote to push a branch without upstream to: `origin`, else the only remote.
    fn default_remote(&self) -> Result<String, OpError> {
        let r = self.remotes()?;
        if r.iter().any(|x| x == "origin") {
            return Ok("origin".into());
        }
        match r.as_slice() {
            [one] => Ok(one.clone()),
            [] => Err(OpError::new("NO_REMOTE", "This repository has no remote.", Some("Add one with git remote add."))),
            _ => Err(OpError::new("NO_UPSTREAM", "This branch has no upstream and there is no \"origin\" remote.", Some("Push it once from a terminal with -u <remote>."))),
        }
    }

    pub(super) fn fetch(&self, env: &Env) -> Result<OpOutcome, OpError> {
        let (code, _, err) = self.git.run_status_env(&self.root, ["fetch", "--all", "--prune", "--quiet"], env)?;
        if code != Some(0) {
            return Err(net_error("fetch", err));
        }
        Ok(OpOutcome { head: self.head(), ..Default::default() })
    }

    pub(super) fn pull(&self, env: &Env) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.merge_in_progress() {
            return Err(OpError::new("MERGE_IN_PROGRESS", "A merge is already in progress.", Some("Finish it or abort it first.")));
        }
        let (Some(branch), Some(up)) = (self.branch(), self.upstream()) else {
            return Err(OpError::new("NO_UPSTREAM", "This branch does not track a remote branch.", Some("Push it first to set the upstream.")));
        };
        let before = self.head();
        let (code, out, err) = self.git.run_status_env(&self.root, ["pull", "--no-rebase", "--no-edit"], env)?;
        if code != Some(0) {
            if self.merge_in_progress() {
                return Ok(OpOutcome { head: self.head(), conflicted: true, ..Default::default() });
            }
            return Err(net_error("pull", if err.is_empty() { out } else { err }));
        }
        let after = self.head();
        if after == before {
            return Ok(OpOutcome { head: after, ..Default::default() });
        }
        // Undo/redo reuse the merge entry logic (`reset --merge` between the two heads).
        let id = new_id();
        if let Some(a) = &after {
            self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}-redo").as_str(), a.as_str()])?;
        }
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: "merge".into(),
            summary: format!("Pull {up} into {branch}"),
            branch: Some(branch),
            head_before: before,
            head_after: after.clone(),
            commands: vec!["git pull --no-rebase --no-edit".into()],
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head: after, ..Default::default() })
    }

    /// Pushes the current branch; without an upstream it is pushed to the default remote and tracked.
    /// `force` uses `--force-with-lease`, which refuses if the remote moved since the last fetch.
    pub(super) fn push(&self, force: bool, env: &Env) -> Result<OpOutcome, OpError> {
        let branch = self
            .branch()
            .ok_or_else(|| OpError::new("INVALID_REF", "Check out a branch to push it.", None))?;
        let mut args: Vec<String> = vec!["push".into(), "--porcelain".into()];
        if force {
            args.push("--force-with-lease".into());
        }
        if self.upstream().is_none() {
            args.extend(["-u".into(), self.default_remote()?, format!("refs/heads/{branch}")]);
        }
        let (code, out, err) = self.git.run_status_env(&self.root, &args, env)?;
        if code != Some(0) {
            // `--porcelain` reports rejections on stdout.
            return Err(net_error("push", format!("{out}\n{err}").trim().to_owned()));
        }
        Ok(OpOutcome { head: self.head(), ..Default::default() })
    }
}
