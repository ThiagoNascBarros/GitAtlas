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
            // `normal`: a new folder is one entry (`dir/`). Its files are listed when acted on
            // (see `untracked_under`), so the status stays cheap in big untracked trees (PERF-06).
            ["status", "--porcelain=v2", "-z", "--branch", "--untracked-files=normal"],
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

/// Text of one commit row, fetched on demand so the core does not keep it in memory.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RowText {
    pub sha: String,
    pub summary: String,
    pub author: String,
    pub email: String,
    pub time: i64,
}

impl CliReader {
    /// Summaries/authors for specific commits, in the order given.
    pub fn rows_text(&self, shas: &[&str]) -> Result<Vec<RowText>, ReadError> {
        if shas.is_empty() {
            return Ok(vec![]);
        }
        for s in shas {
            check_rev(s)?;
        }
        let input = shas.join("\n") + "\n";
        let out = self.git.run_with_input(
            &self.root,
            ["log", "-z", "--no-walk=unsorted", "--stdin", parse::COMMIT_FORMAT],
            input.as_bytes(),
        )?;
        Ok(parse::parse_commits(&out)?
            .into_iter()
            .map(|c| RowText { sha: c.sha, summary: c.summary, author: c.author, email: c.email, time: c.time })
            .collect())
    }

    /// Unified diff of a working-directory file: unstaged (against the index) or
    /// staged (index against HEAD). Untracked files are shown as all-added.
    pub fn worktree_diff(&self, path: &str, staged: bool, untracked: bool) -> Result<String, ReadError> {
        let spec = format!(":(literal){path}");
        let out = if untracked && path.ends_with('/') {
            // A new folder: every file in it, all shown as added.
            let mut all = Vec::new();
            for file in untracked_under(&self.git, &self.root, path)? {
                all.extend(self.no_index_diff(&file)?);
            }
            all
        } else if untracked {
            self.no_index_diff(path)?
        } else if staged {
            self.git.run(&self.root, ["diff", "--cached", "--no-color", "--no-ext-diff", "--", spec.as_str()])?
        } else {
            self.git.run(&self.root, ["diff", "--no-color", "--no-ext-diff", "--", spec.as_str()])?
        };
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    /// Diff of one untracked file against nothing. `--no-index` exits with 1 when the
    /// files differ, which is the normal case, so the exit status is not checked.
    fn no_index_diff(&self, file: &str) -> Result<Vec<u8>, ReadError> {
        let o = self
            .git
            .command(&self.root, ["diff", "--no-index", "--no-color", "--no-ext-diff", "--", "/dev/null", file])?
            .output()
            .map_err(GitError::from)?;
        Ok(o.stdout)
    }
}

/// Untracked, non-ignored files under a status directory entry (`dir/`), relative to the root.
/// Listed only when the user acts on that folder, not on every status refresh.
pub fn untracked_under(git: &Git, root: &Path, dir: &str) -> Result<Vec<String>, GitError> {
    let spec = format!(":(literal){dir}");
    let out = git.run(root, ["ls-files", "--others", "--exclude-standard", "-z", "--", spec.as_str()])?;
    Ok(out
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect())
}

