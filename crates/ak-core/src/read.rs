// SPDX-License-Identifier: Apache-2.0
//! Repository reads (SDD §3 `core::read`). Phase 1 ships the CLI reader;
//! a `gix` reader will implement the same trait.

use std::path::{Path, PathBuf};

use ak_gitcli::parse::{self, CommitRecord, RefRecord, StatusReport};
use ak_gitcli::{Git, GitError};

pub trait RepoReader {
    /// All commits reachable from branches, remotes, tags and HEAD (not stashes),
    /// newest first in date order with parents always after children.
    fn commits(&self) -> Result<Vec<CommitRecord>, ReadError>;
    fn refs(&self) -> Result<Vec<RefRecord>, ReadError>;
    fn status(&self) -> Result<StatusReport, ReadError>;
    fn head(&self) -> Result<Option<String>, ReadError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error(transparent)]
    Git(#[from] GitError),
    #[error(transparent)]
    Parse(#[from] parse::ParseError),
    #[error("not a git repository: {0}")]
    NotARepo(PathBuf),
}

pub struct CliReader {
    git: Git,
    root: PathBuf,
}

impl CliReader {
    /// Opens the repository containing `path` (resolves the top-level dir).
    pub fn open(git: Git, path: &Path) -> Result<Self, ReadError> {
        let out = git
            .run(path, ["rev-parse", "--show-toplevel"])
            .map_err(|_| ReadError::NotARepo(path.to_owned()))?;
        let root = PathBuf::from(String::from_utf8_lossy(&out).trim());
        Ok(Self { git, root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn git(&self) -> &Git {
        &self.git
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FileChange {
    /// Status letter: A, M, D, R, C, T.
    pub status: char,
    pub path: String,
    pub orig_path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitDetails {
    pub sha: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub message: String,
    pub files: Vec<FileChange>,
}

impl CliReader {
    /// Metadata and changed files (against the first parent for merges).
    pub fn commit_details(&self, sha: &str) -> Result<CommitDetails, ReadError> {
        check_rev(sha)?;
        let out = self.git.run(
            &self.root,
            ["show", "-z", "-m", "--first-parent", "--name-status", "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%B%x00", sha],
        )?;
        let text = String::from_utf8_lossy(&out);
        let (head, rest) = text.split_once('\0').ok_or_else(|| parse::ParseError("show output".into()))?;
        let f: Vec<&str> = head.splitn(6, parse::FS).collect();
        if f.len() != 6 {
            return Err(parse::ParseError(format!("show header {head:?}")).into());
        }
        let mut files = Vec::new();
        let mut it = rest.trim_start_matches('\n').split('\0').filter(|s| !s.is_empty());
        while let Some(st) = it.next() {
            let st = st.trim_start_matches('\n');
            let status = st.chars().next().ok_or_else(|| parse::ParseError("empty status".into()))?;
            let first = it.next().ok_or_else(|| parse::ParseError(format!("path after {st}")))?.to_owned();
            let (path, orig_path) = if matches!(status, 'R' | 'C') {
                (it.next().ok_or_else(|| parse::ParseError("rename target".into()))?.to_owned(), Some(first))
            } else {
                (first, None)
            };
            files.push(FileChange { status, path, orig_path });
        }
        Ok(CommitDetails {
            sha: f[0].to_owned(),
            parents: f[1].split_whitespace().map(str::to_owned).collect(),
            author: f[2].to_owned(),
            email: f[3].to_owned(),
            time: f[4].parse().map_err(|_| parse::ParseError("time".into()))?,
            message: f[5].trim_end().to_owned(),
            files,
        })
    }

    /// Unified diff of one file in a commit (first parent for merges).
    pub fn file_diff(&self, sha: &str, path: &str) -> Result<String, ReadError> {
        check_rev(sha)?;
        let out = self.git.run(
            &self.root,
            ["show", "--format=", "-m", "--first-parent", "--no-ext-diff", "--no-color", sha, "--", path],
        )?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }
}

/// Only full or abbreviated hex object ids are accepted from the UI, so a
/// value can never be interpreted as an option or a revision expression.
fn check_rev(rev: &str) -> Result<(), ReadError> {
    if (4..=64).contains(&rev.len()) && rev.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(parse::ParseError(format!("invalid object id {rev:?}")).into())
    }
}

impl RepoReader for CliReader {
    fn commits(&self) -> Result<Vec<CommitRecord>, ReadError> {
        self.commits_limited(None)
    }

    fn refs(&self) -> Result<Vec<RefRecord>, ReadError> {
        Ok(parse::parse_refs(&self.git.run(&self.root, ["for-each-ref", parse::REF_FORMAT])?)?)
    }

    fn status(&self) -> Result<StatusReport, ReadError> {
        Ok(parse::parse_status_v2(&self.git.run(
            &self.root,
            ["status", "--porcelain=v2", "-z", "--branch", "--untracked-files=all"],
        )?)?)
    }

    fn head(&self) -> Result<Option<String>, ReadError> {
        match self.git.run(&self.root, ["rev-parse", "--verify", "--quiet", "HEAD"]) {
            Ok(out) => Ok(Some(String::from_utf8_lossy(&out).trim().to_owned())),
            Err(GitError::Failed { .. }) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

impl CliReader {
    /// Newest `limit` commits (date order). `None` reads the whole history.
    /// Used for the first paint: the rest of the history can load in the background.
    pub fn commits_limited(&self, limit: Option<usize>) -> Result<Vec<CommitRecord>, ReadError> {
        // HEAD may be detached or unborn; --ignore-missing keeps unborn repos working.
        let max = limit.map(|n| format!("--max-count={n}"));
        let mut args = vec!["log", "-z", "--date-order", parse::COMMIT_FORMAT, "--exclude=refs/stash", "--all", "HEAD", "--ignore-missing"];
        if let Some(m) = max.as_deref() {
            args.push(m);
        }
        let out = self.git.run(&self.root, args);
        match out {
            Ok(bytes) => Ok(parse::parse_commits(&bytes)?),
            // Empty repository: no commits yet.
            Err(GitError::Failed { stderr, .. }) if stderr.contains("does not have any commits") => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

}
