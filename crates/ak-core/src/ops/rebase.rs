// SPDX-License-Identifier: Apache-2.0
//! Rebase and interactive rebase (Phase 4). The interactive todo is written by us
//! (`GIT_SEQUENCE_EDITOR=cp <file>`), so no editor ever opens; reword becomes
//! `pick` + `exec git commit --amend -F <file>`, which works on every Git >= 2.20.

use serde::{Deserialize, Serialize};

use super::journal::JournalEntry;
use super::lock::{self, Lock};
use super::{new_id, now, Engine, OpError, OpOutcome, SNAPSHOT_PREFIX};

/// One commit of the interactive rebase list, oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    pub sha: String,
    /// "pick", "reword", "squash", "fixup" or "drop".
    pub action: String,
    /// Summary shown in the editor; for "reword", the new full message.
    pub message: String,
}

/// Shell-quotes a path for GIT_SEQUENCE_EDITOR (run through `sh -c`).
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

impl Engine {
    /// Commits that a rebase onto `onto` would replay, oldest first.
    pub fn rebase_todo(&self, onto: &str) -> Result<Vec<TodoItem>, OpError> {
        let range = format!("{onto}..HEAD");
        let merges = self.git.run(&self.root, ["rev-list", "--merges", "--count", range.as_str()])?;
        if String::from_utf8_lossy(&merges).trim() != "0" {
            return Err(OpError::new("UNSUPPORTED_REBASE", "Those commits include merges, which the rebase editor does not handle.", Some("Use a terminal for this rebase.")));
        }
        let out = self.git.run(&self.root, ["log", "--reverse", "--format=%H%x00%s", range.as_str()])?;
        Ok(String::from_utf8_lossy(&out)
            .lines()
            .filter_map(|l| l.split_once('\0'))
            .map(|(sha, s)| TodoItem { sha: sha.into(), action: "pick".into(), message: s.into() })
            .collect())
    }

    /// Plain rebase of the current branch onto `onto` (`todo` = None) or interactive with `todo`.
    pub(super) fn rebase(&self, onto: &str, todo: Option<&[TodoItem]>) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.in_progress().is_some() {
            return Err(Self::op_in_progress());
        }
        let (Some(before), Some(branch)) = (self.head(), self.branch()) else {
            return Err(OpError::new("INVALID_REF", "Check out a branch to rebase it.", None));
        };
        let onto_sha = self.commit_sha(onto)?;
        let mut env: Vec<(String, String)> = vec![("GIT_EDITOR".into(), "true".into())];
        let mut args: Vec<String> = vec!["rebase".into()];
        let dir = self.git_dir.join("atlaskraken").join("rebase");
        if let Some(items) = todo {
            if items.iter().any(|i| i.action == "squash" || i.action == "fixup") && items.first().is_some_and(|i| i.action != "pick" && i.action != "reword") {
                return Err(OpError::new("INVALID_TODO", "The first commit cannot be squashed into a previous one.", Some("Make the first commit pick or reword.")));
            }
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir)?;
            let mut lines = String::new();
            for (n, i) in items.iter().enumerate() {
                if !i.sha.chars().all(|c| c.is_ascii_hexdigit()) || i.sha.is_empty() {
                    return Err(OpError::new("INVALID_TODO", "The rebase list is invalid.", None));
                }
                match i.action.as_str() {
                    "pick" | "squash" | "fixup" => lines += &format!("{} {}\n", i.action, i.sha),
                    "drop" => {}
                    "reword" => {
                        let msg = dir.join(format!("msg-{n}"));
                        std::fs::write(&msg, &i.message)?;
                        lines += &format!("pick {}\nexec git commit --amend -q --allow-empty -F {}\n", i.sha, sh_quote(&msg.to_string_lossy()));
                    }
                    _ => return Err(OpError::new("INVALID_TODO", "The rebase list is invalid.", None)),
                }
            }
            let file = dir.join("todo");
            std::fs::write(&file, lines)?;
            env.push(("GIT_SEQUENCE_EDITOR".into(), format!("cp {}", sh_quote(&file.to_string_lossy()))));
            args.push("-i".into());
        }
        args.push(onto_sha.clone());
        let (code, out, err) = self.git.run_status_env(&self.root, &args, &env)?;
        if code != Some(0) {
            if self.in_progress() == Some("rebase") {
                return Ok(OpOutcome { head: self.head(), conflicted: true, ..Default::default() });
            }
            return Err(OpError::new("GIT_FAILED", "git rebase failed.", Some("Commit or stash your local changes first.")).with_detail(format!("{out}\n{err}").trim().to_owned()));
        }
        let _ = std::fs::remove_dir_all(&dir);
        self.record_rebase(before, branch, &onto_sha, todo.is_some())
    }

    /// Continue a rebase stopped on conflicts (after resolving and staging them).
    pub(super) fn rebase_continue(&self) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.in_progress() != Some("rebase") {
            return Err(OpError::new("NO_MERGE_IN_PROGRESS", "There is no rebase to continue.", None));
        }
        let env = [("GIT_EDITOR".to_owned(), "true".to_owned())];
        let (code, out, err) = self.git.run_status_env(&self.root, ["rebase", "--continue"], &env)?;
        if code != Some(0) {
            if self.in_progress() == Some("rebase") {
                // Either the next commit conflicts too, or files are still unresolved.
                return Ok(OpOutcome { head: self.head(), conflicted: true, ..Default::default() });
            }
            return Err(OpError::new("GIT_FAILED", "git rebase --continue failed.", None).with_detail(format!("{out}\n{err}").trim().to_owned()));
        }
        let _ = std::fs::remove_dir_all(self.git_dir.join("atlaskraken").join("rebase"));
        // ORIG_HEAD is the branch tip from before the rebase started.
        let before = self.git.run(&self.root, ["rev-parse", "ORIG_HEAD"]).map(|o| String::from_utf8_lossy(&o).trim().to_owned())?;
        let branch = self.branch().unwrap_or_default();
        self.record_rebase(before, branch, "", false)
    }

    fn record_rebase(&self, before: String, branch: String, onto: &str, interactive: bool) -> Result<OpOutcome, OpError> {
        let after = self.head();
        if after.as_deref() == Some(before.as_str()) {
            return Ok(OpOutcome { head: after, ..Default::default() });
        }
        let id = new_id();
        self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}").as_str(), before.as_str()])?;
        if let Some(a) = &after {
            self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}-redo").as_str(), a.as_str()])?;
        }
        let onto_txt = if onto.is_empty() { String::new() } else { format!(" onto {}", &onto[..7.min(onto.len())]) };
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: "rebase".into(),
            summary: format!("{} {branch}{onto_txt}", if interactive { "Interactive rebase of" } else { "Rebase" }),
            branch: Some(branch),
            head_before: Some(before),
            head_after: after.clone(),
            snapshot_ref: Some(format!("{SNAPSHOT_PREFIX}{id}")),
            commands: vec![format!("git rebase{} {onto}", if interactive { " -i" } else { "" })],
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head: after, ..Default::default() })
    }
}
