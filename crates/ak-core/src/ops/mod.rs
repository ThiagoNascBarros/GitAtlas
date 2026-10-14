// SPDX-License-Identifier: Apache-2.0
//! Operation engine (SDD §1.4): every working-directory change goes through
//! here so it is locked, journaled, undoable and reported with a stable error code.

pub mod journal;
pub mod lock;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use ak_gitcli::{Git, GitError, Mode};
use serde::Serialize;

use journal::{Journal, JournalEntry};
use lock::Lock;

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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OpOutcome {
    /// Journal id when the operation can be undone.
    pub id: Option<String>,
    pub head: Option<String>,
}

pub struct Engine {
    git: Git,
    root: PathBuf,
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
                "Your Git is too old to edit this repository; AtlasKraken opens it read-only.",
                Some("Install Git 2.20 or newer."),
            ));
        }
        let top = git.run(path, ["rev-parse", "--show-toplevel"])?;
        let root = PathBuf::from(String::from_utf8_lossy(&top).trim());
        let gd = git.run(&root, ["rev-parse", "--absolute-git-dir"])?;
        let git_dir = PathBuf::from(String::from_utf8_lossy(&gd).trim());
        Ok(Self { git, root, journal: Journal::new(&git_dir) })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn journal(&self) -> Vec<JournalEntry> {
        self.journal.list()
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
        match op {
            Operation::Stage { paths } => self.stage(paths, true),
            Operation::Unstage { paths } => self.stage(paths, false),
            Operation::Commit { message, amend } => self.commit(message, *amend),
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
        })?;
        Ok(OpOutcome { id: Some(id), head: head_after })
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
        if !matches!(entry.kind.as_str(), "commit" | "commit_amend") {
            return Err(OpError::new("UNDO_NOT_SUPPORTED", "This operation cannot be undone.", None));
        }
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
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
        self.journal.mark_undone(id)?;
        Ok(OpOutcome { id: None, head: self.head() })
    }
}
