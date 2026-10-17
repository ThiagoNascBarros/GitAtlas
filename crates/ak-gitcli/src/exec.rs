// SPDX-License-Identifier: Apache-2.0
//! Spawning git with a fixed environment (SDD §5).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::caps::{GitCaps, GitVersion};

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git executable not found or not runnable: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("could not parse git version from {0:?}")]
    Version(String),
    #[error("git {subcommand} is not allowed in read-only mode")]
    NotAllowed { subcommand: String },
    #[error("git {subcommand} failed with status {code:?}: {stderr}")]
    Failed { subcommand: String, code: Option<i32>, stderr: String },
}

/// Read-only mode only permits subcommands that never write to the repo
/// (roadmap Phase 1 constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    ReadOnly,
    ReadWrite,
}

const READ_ONLY_SUBCOMMANDS: &[&str] = &[
    "rev-parse", "rev-list", "for-each-ref", "cat-file", "show", "diff", "status", "log", "ls-tree", "ls-files", "version",
];

/// Environment variables never forwarded to git (credentials of other tools,
/// variables that change git's repository discovery or output).
const ENV_DENY: &[&str] = &[
    "GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_OBJECT_DIRECTORY", "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CEILING_DIRECTORIES", "GIT_NAMESPACE", "GIT_TRACE_PACKET", "GIT_EXTERNAL_DIFF", "GIT_PAGER", "PAGER",
];

#[derive(Debug, Clone)]
pub struct Git {
    exe: PathBuf,
    mode: Mode,
    caps: GitCaps,
}

impl Git {
    /// Locates git (`AK_GIT` env var, else `git` on PATH) and detects capabilities.
    pub fn discover(mode: Mode) -> Result<Self, GitError> {
        let exe = std::env::var_os("AK_GIT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("git"));
        Self::with_executable(exe, mode)
    }

    pub fn with_executable(exe: PathBuf, mode: Mode) -> Result<Self, GitError> {
        let out = Command::new(&exe).arg("version").env("LC_ALL", "C").output()?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let version = GitVersion::parse(&text).ok_or(GitError::Version(text))?;
        // Below the supported floor the app still opens, but read-only.
        let caps = GitCaps::from_version(version);
        let mode = if caps.supported() { mode } else { Mode::ReadOnly };
        Ok(Self { exe, mode, caps })
    }

    pub fn caps(&self) -> &GitCaps {
        &self.caps
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Builds a command for `git -C <repo> <args...>` after policy checks.
    pub fn command<I, S>(&self, repo: &Path, args: I) -> Result<Command, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<_> = args.into_iter().map(|a| a.as_ref().to_owned()).collect();
        let subcommand = args.first().map(|a| a.to_string_lossy().into_owned()).unwrap_or_default();
        if self.mode == Mode::ReadOnly && !READ_ONLY_SUBCOMMANDS.contains(&subcommand.as_str()) {
            return Err(GitError::NotAllowed { subcommand });
        }
        let mut cmd = Command::new(&self.exe);
        for var in ENV_DENY {
            cmd.env_remove(var);
        }
        cmd.env("LC_ALL", "C")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .arg("-C")
            .arg(repo)
            // Deliberately no `-c safe.directory=*` (SDD §5).
            .args(["-c", "core.quotepath=off", "-c", "color.ui=false", "--no-pager"])
            .args(&args)
            .stdin(Stdio::null());
        Ok(cmd)
    }

    /// Like [`run`](Self::run) but feeds `input` to git's stdin (e.g. `commit -F -`).
    pub fn run_with_input<I, S>(&self, repo: &Path, args: I, input: &[u8]) -> Result<Vec<u8>, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.run_with_input_env(repo, args, input, &[])
    }

    /// [`run_with_input`](Self::run_with_input) with extra environment variables
    /// (e.g. `GIT_AUTHOR_NAME` for `commit-tree` when the user has no `user.name`).
    pub fn run_with_input_env<I, S>(&self, repo: &Path, args: I, input: &[u8], env: &[(String, String)]) -> Result<Vec<u8>, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        use std::io::Write;
        let args: Vec<_> = args.into_iter().map(|a| a.as_ref().to_owned()).collect();
        let subcommand = args.first().map(|a| a.to_string_lossy().into_owned()).unwrap_or_default();
        let mut cmd = self.command(repo, &args)?;
        cmd.envs(env.iter().map(|(k, v)| (k, v)));
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        // Write on a thread so a large input cannot deadlock against git's output pipes.
        let mut stdin = child.stdin.take().expect("piped stdin");
        let data = input.to_vec();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(&data);
        });
        let out = child.wait_with_output()?;
        let _ = writer.join();
        if !out.status.success() {
            return Err(GitError::Failed {
                subcommand,
                code: out.status.code(),
                stderr: String::from_utf8_lossy(&out.stderr).trim().to_owned(),
            });
        }
        Ok(out.stdout)
    }

    /// Runs git and returns `(exit code, stdout)` without treating a non-zero exit as an error
    /// (for commands like `merge` and `merge-tree` where 1 means "conflicts").
    pub fn run_status<I, S>(&self, repo: &Path, args: I) -> Result<(Option<i32>, String, String), GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let out = self.command(repo, args)?.output()?;
        Ok((
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).trim().to_owned(),
        ))
    }

    /// [`run_status`](Self::run_status) with extra environment variables (askpass for network commands).
    pub fn run_status_env<I, S>(&self, repo: &Path, args: I, env: &[(String, String)]) -> Result<(Option<i32>, String, String), GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let out = self.command(repo, args)?.envs(env.iter().map(|(k, v)| (k, v))).output()?;
        Ok((
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).trim().to_owned(),
        ))
    }

    /// Runs git and returns stdout bytes; non-zero exit becomes `GitError::Failed`.
    pub fn run<I, S>(&self, repo: &Path, args: I) -> Result<Vec<u8>, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<_> = args.into_iter().map(|a| a.as_ref().to_owned()).collect();
        let subcommand = args.first().map(|a| a.to_string_lossy().into_owned()).unwrap_or_default();
        let out = self.command(repo, &args)?.output()?;
        if !out.status.success() {
            return Err(GitError::Failed {
                subcommand,
                code: out.status.code(),
                stderr: String::from_utf8_lossy(&out.stderr).trim().to_owned(),
            });
        }
        Ok(out.stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_blocks_write_subcommands() {
        let git = Git::discover(Mode::ReadOnly).expect("git installed");
        for sub in ["commit", "push", "merge", "reset", "stash", "maintenance", "gc"] {
            assert!(matches!(git.command(Path::new("."), [sub]), Err(GitError::NotAllowed { .. })), "{sub}");
        }
        assert!(git.command(Path::new("."), ["rev-list"]).is_ok());
    }

    #[test]
    fn never_passes_safe_directory_wildcard() {
        let git = Git::discover(Mode::ReadWrite).expect("git installed");
        let cmd = git.command(Path::new("."), ["status"]).unwrap();
        let joined: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert!(!joined.iter().any(|a| a.contains("safe.directory")));
    }

    #[test]
    fn run_reports_subcommand_on_failure() {
        let git = Git::discover(Mode::ReadOnly).expect("git installed");
        let err = git.run(Path::new("/"), ["rev-parse", "--verify", "no-such-ref-xyz"]).unwrap_err();
        match err {
            GitError::Failed { subcommand, .. } => assert_eq!(subcommand, "rev-parse"),
            other => panic!("unexpected {other:?}"),
        }
    }
}
