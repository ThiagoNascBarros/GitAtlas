// SPDX-License-Identifier: Apache-2.0
//! Spawning git with a fixed environment (SDD §5).

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

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
    #[error("git {subcommand} did not finish within {} s and was stopped", .after.as_secs())]
    TimedOut { subcommand: String, after: Duration },
}

/// Longest a single git process may run (PERF-11). A hook, a lock or a stuck network
/// connection must not keep the app busy forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    /// Commands that stay on this machine (status, log, commit, merge...).
    pub local: Duration,
    /// Commands that talk to a remote (fetch, pull, push, ls-remote, clone).
    pub network: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self { local: Duration::from_secs(300), network: Duration::from_secs(600) }
    }
}

const NETWORK_SUBCOMMANDS: &[&str] = &["fetch", "pull", "push", "ls-remote", "clone"];

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
    timeouts: Timeouts,
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
        Ok(Self { exe, mode, caps, timeouts: Timeouts::default() })
    }

    /// Same handle with other time limits for git processes.
    pub fn with_timeouts(mut self, timeouts: Timeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    pub fn caps(&self) -> &GitCaps {
        &self.caps
    }

    fn limit_for(&self, subcommand: &str) -> Duration {
        if NETWORK_SUBCOMMANDS.contains(&subcommand) { self.timeouts.network } else { self.timeouts.local }
    }

    /// Runs a prepared command to completion, within the limit for `subcommand`.
    fn output_of(&self, mut cmd: Command, subcommand: &str) -> Result<Output, GitError> {
        let child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        wait_limited(child, self.limit_for(subcommand), subcommand)
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
        let out = wait_limited(child, self.limit_for(&subcommand), &subcommand)?;
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
        let args: Vec<_> = args.into_iter().map(|a| a.as_ref().to_owned()).collect();
        let subcommand = first_subcommand(&args);
        let out = self.output_of(self.command(repo, &args)?, &subcommand)?;
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
        let args: Vec<_> = args.into_iter().map(|a| a.as_ref().to_owned()).collect();
        let subcommand = first_subcommand(&args);
        let mut cmd = self.command(repo, &args)?;
        cmd.envs(env.iter().map(|(k, v)| (k, v)));
        let out = self.output_of(cmd, &subcommand)?;
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
        let subcommand = first_subcommand(&args);
        let out = self.output_of(self.command(repo, &args)?, &subcommand)?;
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

fn first_subcommand(args: &[std::ffi::OsString]) -> String {
    args.first().map(|a| a.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Waits for a child whose stdout and stderr are piped, at most `limit`. On timeout the child is
/// killed. The pipes are drained on threads, so a full pipe never blocks git.
fn wait_limited(mut child: Child, limit: Duration, subcommand: &str) -> Result<Output, GitError> {
    fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut buf);
            }
            buf
        })
    }
    let (out, err) = (drain(child.stdout.take()), drain(child.stderr.take()));
    let started = Instant::now();
    let mut pause = Duration::from_micros(500);
    let mut status = None;
    loop {
        if status.is_none() {
            status = child.try_wait()?;
        }
        // Done when git has exited and both pipes reached EOF (a leftover helper can hold them open).
        if let Some(st) = status {
            if out.is_finished() && err.is_finished() {
                return Ok(Output { status: st, stdout: out.join().unwrap_or_default(), stderr: err.join().unwrap_or_default() });
            }
        }
        if started.elapsed() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(GitError::TimedOut { subcommand: subcommand.to_owned(), after: limit });
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(20));
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
    fn network_commands_get_the_network_limit() {
        let t = Timeouts { local: Duration::from_secs(1), network: Duration::from_secs(2) };
        let git = Git::discover(Mode::ReadOnly).expect("git installed").with_timeouts(t);
        assert_eq!(git.limit_for("fetch"), Duration::from_secs(2));
        assert_eq!(git.limit_for("status"), Duration::from_secs(1));
    }

    #[test]
    fn a_process_past_its_limit_is_stopped() {
        let t = Timeouts { local: Duration::ZERO, network: Duration::ZERO };
        let git = Git::discover(Mode::ReadOnly).expect("git installed").with_timeouts(t);
        let err = git.run(Path::new("."), ["rev-parse", "--git-dir"]).unwrap_err();
        assert!(matches!(err, GitError::TimedOut { ref subcommand, .. } if subcommand == "rev-parse"), "{err:?}");
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
