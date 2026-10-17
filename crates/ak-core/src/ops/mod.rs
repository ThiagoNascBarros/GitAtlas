// SPDX-License-Identifier: Apache-2.0
//! Operation engine (SDD §1.4): every working-directory change goes through
//! here so it is locked, journaled, undoable and reported with a stable error code.

mod branch;
mod history;
mod merge;
mod remote;
mod rebase;
pub mod shared_stash;
mod shared_stash_ops;
mod stash;

pub use merge::{ConflictVersions, MergePreview};
pub use rebase::TodoItem;
pub use shared_stash::{InvalidShared, SharedStashInfo, SharedStashMeta, ValidShared};
pub use shared_stash_ops::{KeptSharedStash, ProbeResult, RemoteFetch, RemoteStatus, SharedStashOutcome};
pub use stash::StashInfo;
pub mod journal;
pub mod lock;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use ak_gitcli::{Git, GitError, Mode};
use serde::Serialize;

use journal::{Journal, JournalEntry};
use lock::Lock;

// Per-repo data keeps the pre-rename "atlaskraken" name so existing journals, snapshots and backups stay reachable.
pub const SNAPSHOT_PREFIX: &str = "refs/atlaskraken/snapshots/";
const PATH_CHUNK: usize = 100;

/// Error shown to the user: a stable `code` for the UI/i18n, plus the raw git text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{code}: {message}")]
pub struct OpError {
    pub code: &'static str,
    pub message: String,
    /// Raw git stderr, for "show details".
    pub detail: Option<String>,
    pub suggestion: Option<String>,
}

impl OpError {
    fn new(code: &'static str, message: &str, suggestion: Option<&str>) -> Self {
        Self { code, message: message.to_owned(), detail: None, suggestion: suggestion.map(str::to_owned) }
    }

    fn with_detail(mut self, detail: String) -> Self {
        self.detail = Some(detail);
        self
    }

    fn from_git(e: GitError) -> Self {
        match e {
            GitError::Failed { subcommand, stderr, .. } => {
                let low = stderr.to_lowercase();
                if low.contains("author identity unknown")
                    || low.contains("empty ident name")
                    || low.contains("unable to auto-detect email")
                    || low.contains("committer identity unknown")
                {
                    let mut err = Self::new(
                        "GIT_IDENTITY_MISSING",
                        "Git does not know who you are, so it cannot create the commit.",
                        Some("Set your name and email (git config user.name / user.email) and try again."),
                    );
                    err.detail = Some(stderr);
                    return err;
                }
                let mut err = Self::new("GIT_FAILED", &format!("git {subcommand} failed."), None);
                err.detail = Some(stderr);
                err
            }
            GitError::NotAllowed { .. } => Self::new("READ_ONLY", "This repository is open in read-only mode.", None),
            other => Self::new("GIT_FAILED", &other.to_string(), None),
        }
    }
}

impl From<GitError> for OpError {
    fn from(e: GitError) -> Self {
        Self::from_git(e)
    }
}

impl From<std::io::Error> for OpError {
    fn from(e: std::io::Error) -> Self {
        let mut err = Self::new("IO_ERROR", "Could not write the operation journal.", None);
        err.detail = Some(e.to_string());
        err
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Stage { paths: Vec<String> },
    Unstage { paths: Vec<String> },
    Commit { message: String, amend: bool },
    /// Stage (or unstage) one hunk of a tracked file; `hunk` is the 0-based index
    /// in `git diff` (or `git diff --cached` when unstaging).
    StageHunk { path: String, hunk: usize, unstage: bool },
    /// Throw away working-tree changes. A backup is kept so Undo can restore them.
    Discard { paths: Vec<String> },
    /// New local branch at `at` (default HEAD); it is not checked out.
    BranchCreate { name: String, at: Option<String> },
    BranchRename { old: String, new: String },
    /// `force` deletes even if the branch is not merged (the tip stays recoverable via Undo).
    BranchDelete { name: String, force: bool },
    /// Switch to a local branch, or detach HEAD at a commit when `is_branch` is false.
    Checkout { target: String, is_branch: bool },
    /// Merge `source` into the current branch. Conflicts leave the merge open (see `MergeAbort`).
    Merge { source: String },
    MergeAbort,
    /// Resolve one conflicted file by taking our or their version and staging it.
    ResolveConflict { path: String, theirs: bool },
    /// Resolve one conflicted file with content edited by the user, then stage it.
    ResolveWith { path: String, content: String },
    StashPush { message: Option<String>, include_untracked: bool },
    StashApply { index: usize },
    StashPop { index: usize },
    StashDrop { index: usize },
    CherryPick { sha: String },
    Revert { sha: String },
    /// `mode`: "soft", "mixed" or "hard".
    Reset { sha: String, mode: String },
    /// Annotated when `message` is given.
    TagCreate { name: String, at: String, message: Option<String> },
    TagDelete { name: String },
    Fetch,
    Pull,
    Push { force: bool },
    PushTag { name: String },
    /// Rebase the current branch onto `onto`; interactive when `todo` is given (oldest first).
    Rebase { onto: String, todo: Option<Vec<TodoItem>> },
    RebaseContinue,
    /// Mirror the shared stashes of one remote, or of all; prunes the ones that disappeared. Not journaled.
    SharedStashFetch { remote: Option<String> },
    /// Share a local stash (by sha). `remote` defaults to the upstream remote, else `origin`.
    SharedStashPublish { stash_sha: String, title: String, remote: Option<String> },
    /// Author only. New content (`stash_sha`) and/or new title; the other part is kept.
    SharedStashUpdate { id: String, remote: String, stash_sha: Option<String>, title: Option<String> },
    SharedStashUnshare { id: String, remote: String },
    SharedStashReshare { id: String, remote: String },
    SharedStashDiscardKeep { id: String },
    SharedStashApply { id: String, remote: String },
    SharedStashBranch { id: String, remote: String, name: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OpOutcome {
    /// Journal id when the operation can be undone.
    pub id: Option<String>,
    pub head: Option<String>,
    /// The operation stopped on conflicts that need resolving (merge / stash apply).
    pub conflicted: bool,
    /// Extra results of the Shared Stash operations.
    pub shared_stash: Option<SharedStashOutcome>,
}

pub struct Engine {
    git: Git,
    root: PathBuf,
    git_dir: PathBuf,
    journal: Journal,
}

fn new_id() -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}-{:x}", N.fetch_add(1, Ordering::Relaxed))
}

fn now_ns() -> i128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i128)
        .unwrap_or(0)
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn literal(p: &str) -> String {
    format!(":(literal){p}")
}

impl Engine {
    /// Opens the repository containing `path`. Needs a Git in read-write mode.
    pub fn open(git: Git, path: &Path) -> Result<Self, OpError> {
        if git.mode() != Mode::ReadWrite {
            return Err(OpError::new(
                "READ_ONLY",
                "Your Git is too old to edit this repository; AtlasGit opens it read-only.",
                Some("Install Git 2.20 or newer."),
            ));
        }
        let top = git.run(path, ["rev-parse", "--show-toplevel"])?;
        let root = PathBuf::from(String::from_utf8_lossy(&top).trim());
        let gd = git.run(&root, ["rev-parse", "--absolute-git-dir"])?;
        let git_dir = PathBuf::from(String::from_utf8_lossy(&gd).trim());
        Ok(Self { git, root, journal: Journal::new(&git_dir), git_dir })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn journal(&self) -> Vec<JournalEntry> {
        self.journal.list()
    }

    /// Opt-in cleanup: forgets all but the newest `keep` journal entries together with their
    /// discard backups and snapshot refs. Dropped entries can no longer be undone.
    pub fn prune_journal(&self, keep: usize) -> Result<usize, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let dropped = self.journal.prune(keep)?;
        for e in &dropped {
            let _ = std::fs::remove_dir_all(self.discard_dir(&e.id));
            if let Some(r) = &e.snapshot_ref {
                let _ = self.git.run(&self.root, ["update-ref", "-d", r.as_str()]);
            }
        }
        Ok(dropped.len())
    }

    fn head(&self) -> Option<String> {
        self.git
            .run(&self.root, ["rev-parse", "--verify", "--quiet", "HEAD"])
            .ok()
            .map(|o| String::from_utf8_lossy(&o).trim().to_owned())
    }

    fn branch(&self) -> Option<String> {
        self.git
            .run(&self.root, ["symbolic-ref", "-q", "--short", "HEAD"])
            .ok()
            .map(|o| String::from_utf8_lossy(&o).trim().to_owned())
    }

    fn busy() -> OpError {
        OpError::new(
            "REPO_BUSY",
            "Another operation is running on this repository.",
            Some("Wait for it to finish and try again."),
        )
    }

    pub fn run(&self, op: &Operation) -> Result<OpOutcome, OpError> {
        self.run_with_env(op, &[])
    }

    /// Like [`run`](Self::run), with extra environment for network operations (askpass bridge).
    pub fn run_with_env(&self, op: &Operation, env: &[(String, String)]) -> Result<OpOutcome, OpError> {
        match op {
            Operation::Fetch => self.fetch(env),
            Operation::Pull => self.pull(env),
            Operation::Push { force } => self.push(*force, env),
            Operation::PushTag { name } => self.push_tag(name, env),
            Operation::Stage { paths } => self.stage(paths, true),
            Operation::Unstage { paths } => self.stage(paths, false),
            Operation::Commit { message, amend } => self.commit(message, *amend),
            Operation::StageHunk { path, hunk, unstage } => self.stage_hunk(path, *hunk, *unstage),
            Operation::Discard { paths } => self.discard(paths),
            Operation::BranchCreate { name, at } => self.branch_create(name, at.as_deref()),
            Operation::BranchRename { old, new } => self.branch_rename(old, new),
            Operation::BranchDelete { name, force } => self.branch_delete(name, *force),
            Operation::Checkout { target, is_branch } => self.checkout(target, *is_branch),
            Operation::Merge { source } => self.merge(source),
            Operation::MergeAbort => self.merge_abort(),
            Operation::ResolveConflict { path, theirs } => self.resolve_conflict(path, *theirs),
            Operation::ResolveWith { path, content } => self.resolve_with(path, content),
            Operation::StashPush { message, include_untracked } => self.stash_push(message.as_deref(), *include_untracked),
            Operation::StashApply { index } => self.stash_apply(*index, false),
            Operation::StashPop { index } => self.stash_apply(*index, true),
            Operation::StashDrop { index } => self.stash_drop(*index),
            Operation::CherryPick { sha } => self.pick_like("cherry_pick", sha),
            Operation::Revert { sha } => self.pick_like("revert", sha),
            Operation::Reset { sha, mode } => self.reset(sha, mode),
            Operation::TagCreate { name, at, message } => self.tag_create(name, at, message.as_deref()),
            Operation::TagDelete { name } => self.tag_delete(name),
            Operation::Rebase { onto, todo } => self.rebase(onto, todo.as_deref()),
            Operation::RebaseContinue => self.rebase_continue(),
            Operation::SharedStashFetch { remote } => self.shared_fetch(remote.as_deref(), env),
            Operation::SharedStashPublish { stash_sha, title, remote } => self.shared_publish(stash_sha, title, remote.as_deref(), env),
            Operation::SharedStashUpdate { id, remote, stash_sha, title } => self.shared_update(id, remote, stash_sha.as_deref(), title.as_deref(), env),
            Operation::SharedStashUnshare { id, remote } => self.shared_unshare(id, remote, env),
            Operation::SharedStashReshare { id, remote } => self.shared_reshare(id, remote, env),
            Operation::SharedStashDiscardKeep { id } => self.shared_discard_keep(id),
            Operation::SharedStashApply { id, remote } => self.shared_apply(id, remote, env),
            Operation::SharedStashBranch { id, remote, name } => self.shared_branch(id, remote, name, env),
        }
    }

    fn stage(&self, paths: &[String], add: bool) -> Result<OpOutcome, OpError> {
        if paths.is_empty() || paths.iter().any(|p| p.is_empty() || p.contains('\0')) {
            return Err(OpError::new("INVALID_PATH", "No valid file was given.", None));
        }
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let unborn = self.head().is_none();
        for chunk in paths.chunks(PATH_CHUNK) {
            let specs: Vec<String> = chunk.iter().map(|p| literal(p)).collect();
            let mut args: Vec<&str> = match (add, unborn) {
                (true, _) => vec!["add", "--"],
                (false, false) => vec!["reset", "-q", "HEAD", "--"],
                (false, true) => vec!["rm", "--cached", "-r", "-q", "--"],
            };
            args.extend(specs.iter().map(String::as_str));
            self.git.run(&self.root, args)?;
        }
        Ok(OpOutcome::default())
    }

    fn has_staged(&self) -> Result<bool, OpError> {
        match self.git.run(&self.root, ["diff", "--cached", "--quiet"]) {
            Ok(_) => Ok(false),
            Err(GitError::Failed { code: Some(1), .. }) => Ok(true),
            Err(e) => Err(e.into()),
        }
    }

    fn snapshot(&self, id: &str, sha: &str) -> Result<String, OpError> {
        let name = format!("{SNAPSHOT_PREFIX}{id}");
        self.git.run(&self.root, ["update-ref", name.as_str(), sha])?;
        Ok(name)
    }

    fn commit(&self, message: &str, amend: bool) -> Result<OpOutcome, OpError> {
        if message.trim().is_empty() {
            return Err(OpError::new("EMPTY_MESSAGE", "The commit message is empty.", Some("Write a short summary of the change.")));
        }
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let head_before = self.head();
        if amend && head_before.is_none() {
            return Err(OpError::new("NOTHING_TO_COMMIT", "There is no commit to amend yet.", None));
        }
        if !amend && !self.has_staged()? {
            return Err(OpError::new(
                "NOTHING_TO_COMMIT",
                "There are no staged changes to commit.",
                Some("Stage at least one file first."),
            ));
        }
        let branch = self.branch();
        let id = new_id();
        let snapshot_ref = match (&head_before, amend) {
            (Some(h), true) => Some(self.snapshot(&id, h)?),
            _ => None,
        };
        let mut args = vec!["commit", "-q", "-F", "-"];
        if amend {
            args.push("--amend");
        }
        self.git.run_with_input(&self.root, args.iter().copied(), message.as_bytes())?;
        let head_after = self.head();
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: if amend { "commit_amend" } else { "commit" }.to_owned(),
            summary: message.lines().next().unwrap_or("").to_owned(),
            branch,
            head_before,
            head_after: head_after.clone(),
            snapshot_ref,
            commands: vec![format!("git {}", args.join(" "))],
            undone: false,
            undone_at: 0,
            paths: vec![],
            untracked: vec![],
            args: vec![],
        })?;
        Ok(OpOutcome { id: Some(id), head: head_after, ..Default::default() })
    }

    /// Reverts a journaled commit, keeping its changes staged.
    pub fn undo(&self, id: &str) -> Result<OpOutcome, OpError> {
        let entry = self
            .journal
            .list()
            .into_iter()
            .find(|e| e.id == id)
            .ok_or_else(|| OpError::new("UNDO_NOT_SUPPORTED", "That operation is not in the journal.", None))?;
        if entry.undone {
            return Err(OpError::new("ALREADY_UNDONE", "This operation was already undone.", None));
        }
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if entry.kind == "discard" {
            self.undo_discard(&entry)?;
            self.journal.set_undone(id, true, now_ns())?;
            return Ok(OpOutcome { id: None, head: self.head(), ..Default::default() });
        }
        if branch::is_ref_kind(&entry.kind) {
            self.apply_ref_entry(&entry, false)?;
            self.journal.set_undone(id, true, now_ns())?;
            return Ok(OpOutcome { id: None, head: self.head(), ..Default::default() });
        }
        if !matches!(entry.kind.as_str(), "commit" | "commit_amend") {
            return Err(OpError::new("UNDO_NOT_SUPPORTED", "This operation cannot be undone.", None));
        }
        if self.head() != entry.head_after {
            return Err(OpError::new(
                "HEAD_MOVED",
                "The branch has changed since this operation, so it cannot be undone safely.",
                Some("Undo the newer operations first."),
            ));
        }
        match (&entry.head_before, &entry.branch) {
            (Some(before), _) => {
                self.git.run(&self.root, ["reset", "-q", "--soft", before.as_str()])?;
            }
            (None, Some(branch)) => {
                // First commit: removing the branch ref returns to the unborn state.
                let name = format!("refs/heads/{branch}");
                self.git.run(&self.root, ["update-ref", "-d", name.as_str()])?;
            }
            (None, None) => return Err(OpError::new("UNDO_NOT_SUPPORTED", "This operation cannot be undone.", None)),
        }
        // Keep the undone commit reachable so Redo can bring it back.
        if let Some(after) = &entry.head_after {
            self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}-redo").as_str(), after.as_str()])?;
        }
        self.journal.set_undone(id, true, now_ns())?;
        Ok(OpOutcome { id: None, head: self.head(), ..Default::default() })
    }

    /// The operation Redo would bring back: the most recently undone one, as long
    /// as nothing newer has been done since (a new operation clears the redo stack).
    pub fn redo_candidate(&self) -> Option<JournalEntry> {
        let list = self.journal.list();
        let (pos, cand) = list.iter().enumerate().filter(|(_, e)| e.undone).max_by_key(|(_, e)| e.undone_at)?;
        // Network and apply entries of Shared Stash cannot be undone, so they do not end the redo chain.
        list[pos + 1..].iter().all(|e| e.undone || branch::is_passive_kind(&e.kind)).then(|| cand.clone())
    }

    pub fn redo_last(&self) -> Result<OpOutcome, OpError> {
        let entry = self
            .redo_candidate()
            .ok_or_else(|| OpError::new("NOTHING_TO_REDO", "There is nothing to redo.", None))?;
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if entry.kind == "discard" {
            self.redo_discard(&entry)?;
        } else if branch::is_ref_kind(&entry.kind) {
            self.apply_ref_entry(&entry, true)?;
        } else {
            let moved = || {
                OpError::new(
                    "HEAD_MOVED",
                    "The branch has changed since that operation, so it cannot be redone safely.",
                    None,
                )
            };
            if self.head() != entry.head_before {
                return Err(moved());
            }
            match (&entry.head_after, &entry.head_before, &entry.branch) {
                (Some(after), Some(_), _) => {
                    self.git.run(&self.root, ["reset", "-q", "--soft", after.as_str()])?;
                }
                (Some(after), None, Some(branch)) => {
                    let name = format!("refs/heads/{branch}");
                    self.git.run(&self.root, ["update-ref", name.as_str(), after.as_str()])?;
                }
                _ => return Err(OpError::new("UNDO_NOT_SUPPORTED", "This operation cannot be redone.", None)),
            }
        }
        self.journal.set_undone(&entry.id, false, 0)?;
        Ok(OpOutcome { id: Some(entry.id), head: self.head(), ..Default::default() })
    }

    fn redo_discard(&self, e: &JournalEntry) -> Result<(), OpError> {
        let dir = self.discard_dir(&e.id);
        // Only discard again if the files are still exactly what Undo restored.
        for p in e.paths.iter().chain(&e.untracked) {
            let (bak, cur) = (dir.join(p), self.root.join(p));
            let same = match (std::fs::read(&bak), std::fs::read(&cur)) {
                (Ok(a), Ok(b)) => a == b,
                (Err(_), Err(_)) => true,
                _ => false,
            };
            if !same {
                return Err(OpError::new(
                    "WORKTREE_CHANGED",
                    "These files changed after the undo, so discarding them again would lose newer work.",
                    None,
                ));
            }
        }
        for chunk in e.paths.chunks(PATH_CHUNK) {
            let specs: Vec<String> = chunk.iter().map(|p| literal(p)).collect();
            let mut args = vec!["checkout", "-q", "--"];
            args.extend(specs.iter().map(String::as_str));
            self.git.run(&self.root, args)?;
        }
        for p in &e.untracked {
            let _ = std::fs::remove_file(self.root.join(p));
        }
        Ok(())
    }

    fn stage_hunk(&self, path: &str, hunk: usize, unstage: bool) -> Result<OpOutcome, OpError> {
        check_rel(path)?;
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let spec = literal(path);
        let mut args = vec!["diff", "--no-color", "--no-ext-diff", "-U3"];
        if unstage {
            args.push("--cached");
        }
        args.extend(["--", spec.as_str()]);
        let diff = self.git.run(&self.root, args)?;
        let (header, hunks) = split_patch(&diff);
        let chosen = hunks
            .get(hunk)
            .ok_or_else(|| OpError::new("INVALID_HUNK", "That change no longer exists; the file was modified.", Some("Refresh and try again.")))?;
        let mut patch = header;
        patch.extend_from_slice(chosen);
        let mut apply = vec!["apply", "--cached", "--recount", "--whitespace=nowarn"];
        if unstage {
            apply.push("--reverse");
        }
        apply.push("-");
        self.git.run_with_input(&self.root, apply, &patch)?;
        Ok(OpOutcome::default())
    }

    fn discard_dir(&self, id: &str) -> PathBuf {
        self.git_dir.join("atlaskraken").join("discarded").join(id)
    }

    fn discard(&self, paths: &[String]) -> Result<OpOutcome, OpError> {
        if paths.is_empty() {
            return Err(OpError::new("INVALID_PATH", "No valid file was given.", None));
        }
        for p in paths {
            check_rel(p)?;
        }
        // A new folder is one status entry (`dir/`): discard the files in it, one by one.
        let mut expanded = vec![];
        for p in paths {
            if p.ends_with('/') {
                expanded.extend(crate::read::untracked_under(&self.git, &self.root, p)?);
            } else {
                expanded.push(p.clone());
            }
        }
        if expanded.is_empty() {
            return Err(OpError::new("INVALID_PATH", "No valid file was given.", None));
        }
        let paths = expanded.as_slice();
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let mut tracked = std::collections::HashSet::new();
        for chunk in paths.chunks(PATH_CHUNK) {
            let specs: Vec<String> = chunk.iter().map(|p| literal(p)).collect();
            let mut args = vec!["ls-files", "-z", "--"];
            args.extend(specs.iter().map(String::as_str));
            let out = self.git.run(&self.root, args)?;
            tracked.extend(String::from_utf8_lossy(&out).split('\0').filter(|s| !s.is_empty()).map(str::to_owned));
        }
        let id = new_id();
        let dir = self.discard_dir(&id);
        let (mut keep_tracked, mut keep_untracked) = (vec![], vec![]);
        // Back everything up first; nothing is touched if a backup fails.
        for p in paths {
            let src = self.root.join(p);
            match std::fs::symlink_metadata(&src) {
                Ok(m) if m.is_file() => {
                    let dst = dir.join(p);
                    std::fs::create_dir_all(dst.parent().unwrap_or(&dir))?;
                    std::fs::copy(&src, &dst)?;
                }
                Ok(_) => return Err(OpError::new("UNSUPPORTED_PATH", "Only regular files can be discarded safely.", Some("Discard it from a terminal."))),
                Err(_) => {} // deleted tracked file: nothing to back up
            }
            if tracked.contains(p) { keep_tracked.push(p.clone()) } else { keep_untracked.push(p.clone()) }
        }
        let mut commands = vec![];
        for chunk in keep_tracked.chunks(PATH_CHUNK) {
            let specs: Vec<String> = chunk.iter().map(|p| literal(p)).collect();
            let mut args = vec!["checkout", "-q", "--"];
            args.extend(specs.iter().map(String::as_str));
            self.git.run(&self.root, args)?;
        }
        if !keep_tracked.is_empty() {
            commands.push("git checkout -q -- <files>".to_owned());
        }
        for p in &keep_untracked {
            std::fs::remove_file(self.root.join(p))?;
        }
        if !keep_untracked.is_empty() {
            commands.push("delete untracked files".to_owned());
        }
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: "discard".to_owned(),
            summary: if paths.len() == 1 { paths[0].clone() } else { format!("{} files", paths.len()) },
            branch: self.branch(),
            head_before: self.head(),
            head_after: self.head(),
            snapshot_ref: None,
            commands,
            undone: false,
            undone_at: 0,
            paths: keep_tracked,
            untracked: keep_untracked,
            args: vec![],
        })?;
        Ok(OpOutcome { id: Some(id), head: self.head(), ..Default::default() })
    }

    fn undo_discard(&self, e: &JournalEntry) -> Result<(), OpError> {
        let changed = || {
            OpError::new(
                "WORKTREE_CHANGED",
                "These files changed after they were discarded, so restoring them would overwrite newer work.",
                Some("Save or discard the newer changes first."),
            )
        };
        for p in &e.untracked {
            if self.root.join(p).exists() {
                return Err(changed());
            }
        }
        for chunk in e.paths.chunks(PATH_CHUNK) {
            let specs: Vec<String> = chunk.iter().map(|p| literal(p)).collect();
            let mut args = vec!["diff", "--quiet", "--"];
            args.extend(specs.iter().map(String::as_str));
            match self.git.run(&self.root, args) {
                Ok(_) => {}
                Err(GitError::Failed { code: Some(1), .. }) => return Err(changed()),
                Err(err) => return Err(err.into()),
            }
        }
        let dir = self.discard_dir(&e.id);
        for p in e.paths.iter().chain(&e.untracked) {
            let (bak, dst) = (dir.join(p), self.root.join(p));
            if bak.is_file() {
                std::fs::create_dir_all(dst.parent().unwrap_or(&self.root))?;
                std::fs::copy(&bak, &dst)?;
            } else {
                let _ = std::fs::remove_file(&dst); // the discarded change was a deletion
            }
        }
        Ok(())
    }
}

/// Paths from the UI must stay inside the working tree and away from `.git`.
fn check_rel(p: &str) -> Result<(), OpError> {
    let bad = p.is_empty()
        || p.contains('\0')
        || Path::new(p).is_absolute()
        || Path::new(p).components().any(|c| {
            matches!(c, std::path::Component::ParentDir | std::path::Component::RootDir)
                || c.as_os_str() == ".git"
        });
    if bad {
        Err(OpError::new("INVALID_PATH", "That path is not inside the repository.", None))
    } else {
        Ok(())
    }
}

/// Splits a unified diff of one file into its header and its hunks (bytes, so
/// non-UTF-8 content survives the round trip to `git apply`).
fn split_patch(diff: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut header = vec![];
    let mut hunks: Vec<Vec<u8>> = vec![];
    for line in diff.split_inclusive(|b| *b == b'\n') {
        if line.starts_with(b"@@ ") {
            hunks.push(line.to_vec());
        } else if let Some(h) = hunks.last_mut() {
            h.extend_from_slice(line);
        } else {
            header.extend_from_slice(line);
        }
    }
    (header, hunks)
}
