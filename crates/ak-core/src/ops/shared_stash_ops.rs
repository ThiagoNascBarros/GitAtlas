// SPDX-License-Identifier: Apache-2.0
//! Shared Stash operations (docs/specs/shared-stash.md §5): fetch, probe, publish, update,
//! unshare/reshare, apply and branch. Network operations hold `Lock::Remote`, are journaled for
//! history only (not undoable) and take the askpass environment like fetch/pull/push.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::journal::JournalEntry;
use super::lock::{self, Lock};
use super::remote::net_error;
use super::shared_stash::{
    author_slug, clean_title, keep_ref, mirror_ref, mirror_refspec, normalize_email, probe_ref, remote_ref, SharedStashInfo, SharedStashMeta,
    KEEP_PREFIX,
};
use super::{new_id, now, Engine, OpError, OpOutcome};

type Env = [(String, String)];

/// Warn when the stash adds more than this many bytes of blobs the remote does not have yet.
pub const LARGE_STASH_BYTES: u64 = 10 * 1024 * 1024;

// ---------------------------------------------------------------- results

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeResult {
    Ok,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemoteStatus {
    pub name: String,
    /// The cached answer of the capability probe; `None` when not checked (or the URL changed).
    pub probe: Option<ProbeResult>,
}

/// Result of fetching shared stashes from one remote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemoteFetch {
    pub remote: String,
    pub error: Option<OpError>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SharedStashOutcome {
    /// The shared stash the operation was about.
    pub id: Option<String>,
    pub remote: Option<String>,
    /// Publish: bytes of new blobs, set only above [`LARGE_STASH_BYTES`].
    pub large_bytes: Option<u64>,
    /// Apply/branch: the staged state could not be restored, so everything is unstaged.
    pub index_dropped: bool,
    /// Fetch: one entry per remote.
    pub fetched: Vec<RemoteFetch>,
}

/// An unshared stash kept locally so it can be shared again (author only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeptSharedStash {
    pub id: String,
    pub remote: Option<String>,
    pub slug: String,
    pub wrapper: String,
    pub stash: String,
    pub title: String,
    pub base_branch: String,
    pub created: i64,
    pub updated: i64,
    pub unshared_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProbeEntry {
    probe: ProbeResult,
    url: String,
    checked: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeepEntry {
    remote: String,
    slug: String,
    unshared_at: i64,
}

// ---------------------------------------------------------------- errors

fn gone() -> OpError {
    OpError::new("SHARED_STASH_GONE", "That shared stash no longer exists on the remote.", Some("Refresh the list; its author may have unshared it."))
}

fn invalid_stash() -> OpError {
    OpError::new("INVALID_SHARED_STASH", "That shared stash is malformed, so Git Atlas will not use it.", Some("Ask its author to share it again."))
}

fn not_author() -> OpError {
    OpError::new("NOT_AUTHOR", "Only the author can change or remove a shared stash.", None)
}

fn stale() -> OpError {
    OpError::new(
        "STALE_SHARED_STASH",
        "The shared stash changed on the remote in the meantime; nothing was overwritten.",
        Some("Refresh the list and try again."),
    )
}

fn no_remote(msg: &str) -> OpError {
    OpError::new("NO_REMOTE", msg, Some("Add a remote, or choose one that exists."))
}

fn remote_rejected(remote: &str) -> OpError {
    OpError::new(
        "REMOTE_REJECTED",
        &format!("{remote} does not accept shared stashes."),
        Some("Some hosts (Gerrit, Bitbucket Server, Azure DevOps, some gateways) block custom refs. Use another remote, or retry the check after the host changed."),
    )
}

fn dirty_tree() -> OpError {
    OpError::new(
        "DIRTY_TREE",
        "Your working tree has changes in files this stash also touches.",
        Some("Commit or stash your changes first, then try again."),
    )
}

fn base_missing() -> OpError {
    OpError::new("BASE_MISSING", "The commit this stash was taken on is not available.", Some("Fetch the branch it was taken on, then try again."))
}

fn stash_not_found() -> OpError {
    OpError::new("STASH_NOT_FOUND", "That stash no longer exists.", Some("Refresh and try again."))
}

/// Turns the output of a failed `git push` into a stable code. `lease`: a `--force-with-lease`
/// push, where "stale" means the remote ref is not what we expected.
fn push_error(remote: &str, text: String, lease: bool) -> OpError {
    let low = text.to_lowercase();
    // Git reports a lost lease as "stale info" (client side) or, from the receiving end,
    // "failed to update ref" / "incorrect old value provided" (newer Git), with "but expected".
    if lease
        && (low.contains("stale info")
            || low.contains("failed to update ref")
            || low.contains("incorrect old value")
            || low.contains("but expected"))
    {
        return stale().with_detail(text);
    }
    if is_rejection(&text) {
        return remote_rejected(remote).with_detail(text);
    }
    net_error("push", text)
}

fn is_rejection(text: &str) -> bool {
    let low = text.to_lowercase();
    low.contains("[remote rejected]") || low.contains("hook declined")
}

// ---------------------------------------------------------------- small helpers

struct Identity {
    name: String,
    email: String,
}

/// Branch name from a stash message ("WIP on main: …" / "On main: …"); empty when detached.
fn branch_of_stash(message: &str) -> String {
    let rest = message.strip_prefix("WIP on ").or_else(|| message.strip_prefix("On ")).unwrap_or("");
    let name = rest.split(':').next().unwrap_or("").trim();
    if name == "(no branch)" || name.contains(' ') {
        String::new()
    } else {
        name.to_owned()
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), OpError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

const DIRTY_MARKERS: &[&str] = &[
    "would be overwritten",
    "please commit your changes or stash them",
    "already exists, no checkout",
    "untracked working tree files",
    "local changes",
];

fn looks_dirty(text: &str) -> bool {
    let low = text.to_lowercase();
    DIRTY_MARKERS.iter().any(|m| low.contains(m))
}

enum Applied {
    Clean,
    Conflict,
}

impl Engine {
    fn atlasgit_file(&self, name: &str) -> std::path::PathBuf {
        self.git_dir.join("atlasgit").join(name)
    }

    fn identity(&self) -> Result<Identity, OpError> {
        let email = self.config_value("user.email").ok_or_else(|| {
            OpError::new(
                "NO_IDENTITY",
                "Git does not know your email, so the stash cannot be marked as yours.",
                Some("Set it with git config user.email and try again."),
            )
        })?;
        Ok(Identity { name: self.config_value("user.name").unwrap_or_default(), email })
    }

    fn busy_remote(&self) -> Result<lock::LockGuard, OpError> {
        lock::try_acquire(&self.root, &[Lock::Remote]).map_err(|_| Self::busy())
    }

    fn require_remote(&self, remote: &str) -> Result<(), OpError> {
        if self.remotes()?.iter().any(|r| r == remote) {
            Ok(())
        } else {
            Err(no_remote("That remote does not exist."))
        }
    }

    /// Remote preselected for sharing: the upstream remote of the current branch, else `origin`.
    pub fn shared_stash_default_remote(&self) -> Option<String> {
        let remotes = self.remotes().ok()?;
        let upstream = self.branch().and_then(|b| self.config_value(&format!("branch.{b}.remote")));
        upstream.filter(|r| remotes.contains(r)).or_else(|| remotes.iter().find(|r| *r == "origin").cloned())
    }

    fn resolve_remote(&self, remote: Option<&str>) -> Result<String, OpError> {
        match remote {
            Some(r) => self.require_remote(r).map(|_| r.to_owned()),
            None => self.shared_stash_default_remote().ok_or_else(|| no_remote("There is no default remote to share to; choose one.")),
        }
    }

    fn empty_tree(&self) -> Result<String, OpError> {
        let out = self.git.run_with_input(&self.root, ["mktree"], b"")?;
        Ok(String::from_utf8_lossy(&out).trim().to_owned())
    }

    /// Commit-tree identity override for people without `user.name` (their email local part is used).
    fn author_env(id: &Identity) -> Vec<(String, String)> {
        if !id.name.is_empty() {
            return vec![];
        }
        let local = id.email.split('@').next().unwrap_or("user").to_owned();
        vec![("GIT_AUTHOR_NAME".into(), local.clone()), ("GIT_COMMITTER_NAME".into(), local)]
    }

    /// The wrapper: empty tree, the stash as parent, metadata trailers; signed when `commit.gpgsign` is on.
    fn make_wrapper(&self, id: &Identity, stash: &str, meta: &SharedStashMeta) -> Result<String, OpError> {
        let tree = self.empty_tree()?;
        let mut args = vec!["commit-tree".to_owned(), tree, "-p".into(), stash.to_owned()];
        if self
            .git
            .run(&self.root, ["config", "--bool", "--get", "commit.gpgsign"])
            .map(|o| String::from_utf8_lossy(&o).trim() == "true")
            .unwrap_or(false)
        {
            args.push("-S".into());
        }
        let out = self.git.run_with_input_env(&self.root, &args, meta.format_message().as_bytes(), &Self::author_env(id))?;
        Ok(String::from_utf8_lossy(&out).trim().to_owned())
    }

    fn local_stash(&self, sha: &str) -> Result<super::StashInfo, OpError> {
        self.stashes()?.into_iter().find(|s| s.sha == sha).ok_or_else(stash_not_found)
    }

    fn shared_entry(&self, remote: &str, id: &str) -> Result<SharedStashInfo, OpError> {
        self.shared_stash_entries(Some(remote))?.into_iter().find(|s| s.id == id).ok_or_else(gone)
    }

    /// Author-only check on a mirrored entry (spec §5.1). Invalid entries are never acted on.
    fn require_author(info: &SharedStashInfo) -> Result<(), OpError> {
        if !info.valid {
            return Err(invalid_stash());
        }
        if !info.mine {
            return Err(not_author());
        }
        Ok(())
    }

    fn push_shared(&self, remote: &str, src: &str, dst: &str, lease: Option<&str>, env: &Env) -> Result<(), OpError> {
        let mut args: Vec<String> = ["push", "--porcelain", "--no-follow-tags", "--no-recurse-submodules"].map(str::to_owned).to_vec();
        if let Some(expect) = lease {
            args.push(format!("--force-with-lease={dst}:{expect}"));
        }
        args.extend([remote.to_owned(), format!("{src}:{dst}")]);
        let (code, out, err) = self.git.run_status_env(&self.root, &args, env)?;
        if code == Some(0) {
            return Ok(());
        }
        Err(push_error(remote, format!("{out}\n{err}").trim().to_owned(), lease.is_some()))
    }

    /// Fresh fetch of one shared stash into its mirror ref; returns the mirror's sha.
    fn fetch_one(&self, remote: &str, slug: &str, id: &str, env: &Env) -> Result<String, OpError> {
        let mirror = mirror_ref(remote, slug, id);
        let spec = format!("+{}:{mirror}", remote_ref(slug, id));
        let (code, _, err) =
            self.git.run_status_env(&self.root, ["fetch", "--quiet", "--no-tags", "--no-recurse-submodules", remote, spec.as_str()], env)?;
        if code != Some(0) {
            return Err(if err.to_lowercase().contains("couldn't find remote ref") { gone() } else { net_error("fetch", err) });
        }
        let sha = self.git.run(&self.root, ["rev-parse", "--verify", "-q", mirror.as_str()])?;
        Ok(String::from_utf8_lossy(&sha).trim().to_owned())
    }

    /// After our own successful push: refresh the mirror from the remote, or write it directly.
    fn refresh_mirror(&self, remote: &str, slug: &str, id: &str, pushed: &str, env: &Env) -> Result<(), OpError> {
        if self.fetch_one(remote, slug, id, env).is_err() {
            self.git.run(&self.root, ["update-ref", mirror_ref(remote, slug, id).as_str(), pushed])?;
        }
        Ok(())
    }

    fn journal_shared(&self, kind: &str, summary: String, args: Vec<String>, commands: Vec<String>) -> Result<(), OpError> {
        self.journal.append(&JournalEntry {
            id: new_id(),
            time: now(),
            kind: kind.to_owned(),
            summary,
            branch: self.branch(),
            head_before: self.head(),
            head_after: self.head(),
            commands,
            args,
            ..Default::default()
        })?;
        Ok(())
    }

    fn outcome(&self, id: &str, remote: &str) -> SharedStashOutcome {
        SharedStashOutcome { id: Some(id.to_owned()), remote: Some(remote.to_owned()), ..Default::default() }
    }

    // ---------------------------------------------------------------- fetch

    /// Mirrors the shared stashes of one remote (or of all), pruning the ones that disappeared.
    /// With no `remote`, one failing remote does not stop the others; the result lists each.
    pub(super) fn shared_fetch(&self, remote: Option<&str>, env: &Env) -> Result<OpOutcome, OpError> {
        let _g = self.busy_remote()?;
        let fetch_one_remote = |r: &str| -> Result<(), OpError> {
            let spec = mirror_refspec(r);
            let (code, _, err) = self.git.run_status_env(
                &self.root,
                ["fetch", "--quiet", "--prune", "--no-tags", "--no-recurse-submodules", r, spec.as_str()],
                env,
            )?;
            if code == Some(0) {
                Ok(())
            } else {
                Err(net_error("fetch", err))
            }
        };
        let fetched = match remote {
            Some(r) => {
                self.require_remote(r)?;
                fetch_one_remote(r)?;
                vec![RemoteFetch { remote: r.to_owned(), error: None }]
            }
            None => self.remotes()?.into_iter().map(|r| RemoteFetch { error: fetch_one_remote(&r).err(), remote: r }).collect(),
        };
        Ok(OpOutcome { head: self.head(), shared_stash: Some(SharedStashOutcome { fetched, ..Default::default() }), ..Default::default() })
    }

    // ---------------------------------------------------------------- probe (spec §5.2)

    fn probe_cache(&self) -> BTreeMap<String, ProbeEntry> {
        read_json(&self.atlasgit_file("remotes.json"))
    }

    fn remote_url(&self, remote: &str) -> String {
        self.config_value(&format!("remote.{remote}.url")).unwrap_or_default()
    }

    /// Cached verdict per remote. An answer for a different URL than today's is ignored.
    pub fn shared_stash_remotes(&self) -> Vec<RemoteStatus> {
        let cache = self.probe_cache();
        self.remotes()
            .unwrap_or_default()
            .into_iter()
            .map(|name| {
                let probe = cache.get(&name).filter(|e| e.url == self.remote_url(&name)).map(|e| e.probe);
                RemoteStatus { name, probe }
            })
            .collect()
    }

    /// Forgets the cached verdict ("Retry check"); all remotes when `remote` is `None`.
    pub fn shared_stash_probe_reset(&self, remote: Option<&str>) -> Result<(), OpError> {
        let _g = self.busy_remote()?;
        let mut cache = self.probe_cache();
        match remote {
            Some(r) => cache.remove(r),
            None => {
                cache.clear();
                None
            }
        };
        write_json(&self.atlasgit_file("remotes.json"), &cache)
    }

    /// Pushes and deletes a throwaway ref to learn whether the host accepts custom refs.
    pub fn shared_stash_probe(&self, remote: &str, env: &Env) -> Result<ProbeResult, OpError> {
        let _g = self.busy_remote()?;
        self.require_remote(remote)?;
        self.run_probe(remote, env)
    }

    fn cached_probe(&self, remote: &str, env: &Env) -> Result<ProbeResult, OpError> {
        match self.shared_stash_remotes().into_iter().find(|r| r.name == remote).and_then(|r| r.probe) {
            Some(p) => Ok(p),
            None => self.run_probe(remote, env),
        }
    }

    fn run_probe(&self, remote: &str, env: &Env) -> Result<ProbeResult, OpError> {
        // An unrelated commit with a fixed identity: nothing of the user's history is uploaded.
        let tree = self.empty_tree()?;
        let probe_env = [
            ("GIT_AUTHOR_NAME", "AtlasGit"),
            ("GIT_AUTHOR_EMAIL", "probe@atlasgit.invalid"),
            ("GIT_COMMITTER_NAME", "AtlasGit"),
            ("GIT_COMMITTER_EMAIL", "probe@atlasgit.invalid"),
        ]
        .map(|(k, v)| (k.to_owned(), v.to_owned()));
        let commit = self.git.run_with_input_env(&self.root, ["commit-tree", tree.as_str(), "-m", "AtlasGit probe"], b"", &probe_env)?;
        let commit = String::from_utf8_lossy(&commit).trim().to_owned();
        let target = probe_ref(&new_id());
        let verdict = match self.push_shared(remote, &commit, &target, None, env) {
            Ok(()) => match self.push_shared(remote, "", &target, None, env) {
                Ok(()) => ProbeResult::Ok,
                // Created but cannot be deleted: Unshare would fail on this host too.
                Err(e) if e.code == "REMOTE_REJECTED" => ProbeResult::Rejected,
                Err(e) => return Err(e),
            },
            Err(e) if e.code == "REMOTE_REJECTED" => ProbeResult::Rejected,
            Err(e) => return Err(e),
        };
        let mut cache = self.probe_cache();
        cache.insert(remote.to_owned(), ProbeEntry { probe: verdict, url: self.remote_url(remote), checked: now() });
        write_json(&self.atlasgit_file("remotes.json"), &cache)?;
        Ok(verdict)
    }

    // ---------------------------------------------------------------- publish / update

    /// Bytes of blobs in the stash that the remote's known branches do not contain; best effort.
    fn new_blob_bytes(&self, stash: &str, remote: &str) -> u64 {
        let remotes = format!("--remotes={remote}");
        let Ok(list) = self.git.run(&self.root, ["rev-list", "--objects", stash, "--not", remotes.as_str()]) else { return 0 };
        let shas: String =
            String::from_utf8_lossy(&list).lines().filter_map(|l| l.split(' ').next().filter(|s| !s.is_empty())).map(|s| format!("{s}\n")).collect();
        if shas.is_empty() {
            return 0;
        }
        let Ok(sizes) = self.git.run_with_input(&self.root, ["cat-file", "--batch-check=%(objecttype) %(objectsize)"], shas.as_bytes()) else {
            return 0;
        };
        String::from_utf8_lossy(&sizes).lines().filter_map(|l| l.strip_prefix("blob ")?.parse::<u64>().ok()).sum()
    }

    pub(super) fn shared_publish(&self, stash_sha: &str, title: &str, remote: Option<&str>, env: &Env) -> Result<OpOutcome, OpError> {
        let _g = self.busy_remote()?;
        let who = self.identity()?;
        let remote = self.resolve_remote(remote)?;
        let stash = self.local_stash(stash_sha)?;
        if self.cached_probe(&remote, env)? == ProbeResult::Rejected {
            return Err(remote_rejected(&remote));
        }
        let bytes = self.new_blob_bytes(&stash.sha, &remote);
        let slug = author_slug(&who.name, &who.email);
        let id = new_id();
        let at = now();
        let title = clean_title(if title.trim().is_empty() { &stash.message } else { title });
        let meta = SharedStashMeta {
            title: title.clone(),
            author_email: who.email.clone(),
            base: stash.base.clone(),
            base_branch: branch_of_stash(&stash.message),
            created: at,
            updated: at,
        };
        let wrapper = self.make_wrapper(&who, &stash.sha, &meta)?;
        let dst = remote_ref(&slug, &id);
        self.push_shared(&remote, &wrapper, &dst, None, env)?;
        self.refresh_mirror(&remote, &slug, &id, &wrapper, env)?;
        self.journal_shared(
            "shared_stash_publish",
            format!("Share stash \"{title}\" to {remote}"),
            vec![id.clone(), remote.clone(), wrapper.clone()],
            vec![format!("git push {remote} {wrapper}:{dst}")],
        )?;
        let large_bytes = (bytes > LARGE_STASH_BYTES).then_some(bytes);
        Ok(OpOutcome {
            head: self.head(),
            shared_stash: Some(SharedStashOutcome { large_bytes, ..self.outcome(&id, &remote) }),
            ..Default::default()
        })
    }

    /// Replaces the content (a newer local stash) and/or the title of one of your shared stashes.
    pub(super) fn shared_update(
        &self,
        id: &str,
        remote: &str,
        stash_sha: Option<&str>,
        title: Option<&str>,
        env: &Env,
    ) -> Result<OpOutcome, OpError> {
        let _g = self.busy_remote()?;
        let who = self.identity()?;
        if stash_sha.is_none() && title.is_none() {
            return Err(OpError::new("NOTHING_TO_UPDATE", "There is nothing to update.", None));
        }
        let cached = self.shared_entry(remote, id)?;
        Self::require_author(&cached)?;
        // The lease is the value of the mirror right after a fresh fetch of this one ref.
        self.fetch_one(remote, &cached.slug, id, env)?;
        let fresh = self.shared_entry(remote, id)?;
        Self::require_author(&fresh)?;
        let (stash, base, base_branch) = match stash_sha {
            Some(s) => {
                let st = self.local_stash(s)?;
                (st.sha, st.base, branch_of_stash(&st.message))
            }
            None => (fresh.stash.clone(), fresh.base.clone(), fresh.base_branch.clone()),
        };
        let title = title.map_or_else(|| fresh.title.clone(), clean_title);
        let meta = SharedStashMeta {
            title: title.clone(),
            author_email: fresh.author_email.clone(),
            base,
            base_branch,
            created: fresh.created,
            updated: now(),
        };
        let wrapper = self.make_wrapper(&who, &stash, &meta)?;
        let dst = remote_ref(&fresh.slug, id);
        self.push_shared(remote, &wrapper, &dst, Some(&fresh.wrapper), env)?;
        self.refresh_mirror(remote, &fresh.slug, id, &wrapper, env)?;
        self.journal_shared(
            "shared_stash_update",
            format!("Update shared stash \"{title}\" on {remote}"),
            vec![id.to_owned(), remote.to_owned(), wrapper.clone()],
            vec![format!("git push --force-with-lease={dst}:{} {remote} {wrapper}:{dst}", fresh.wrapper)],
        )?;
        Ok(OpOutcome { head: self.head(), shared_stash: Some(self.outcome(id, remote)), ..Default::default() })
    }

    // ---------------------------------------------------------------- unshare / reshare / discard

    fn keep_book(&self) -> BTreeMap<String, KeepEntry> {
        read_json(&self.atlasgit_file("keep.json"))
    }

    fn forget_keep(&self, id: &str) -> Result<(), OpError> {
        let _ = self.git.run(&self.root, ["update-ref", "-d", keep_ref(id).as_str()]);
        let mut book = self.keep_book();
        if book.remove(id).is_some() {
            write_json(&self.atlasgit_file("keep.json"), &book)?;
        }
        Ok(())
    }

    fn keep_sha(&self, id: &str) -> Option<String> {
        // ids are ours (`new_id`); anything else never names a keep ref.
        let plain = !id.is_empty()
            && id.len() <= 200
            && !id.contains("..")
            && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        if !plain {
            return None;
        }
        let r = keep_ref(id);
        self.git.run(&self.root, ["rev-parse", "--verify", "-q", r.as_str()]).ok().map(|o| String::from_utf8_lossy(&o).trim().to_owned())
    }

    /// Removes your shared stash from the remote. A copy stays in `refs/atlasgit/keep/<id>`.
    pub(super) fn shared_unshare(&self, id: &str, remote: &str, env: &Env) -> Result<OpOutcome, OpError> {
        let _g = self.busy_remote()?;
        self.identity()?;
        let cached = self.shared_entry(remote, id)?;
        Self::require_author(&cached)?;
        self.fetch_one(remote, &cached.slug, id, env)?;
        let fresh = self.shared_entry(remote, id)?;
        Self::require_author(&fresh)?;

        let previous = (self.keep_sha(id), self.keep_book().get(id).cloned());
        self.git.run(&self.root, ["update-ref", keep_ref(id).as_str(), fresh.wrapper.as_str()])?;
        let mut book = self.keep_book();
        book.insert(id.to_owned(), KeepEntry { remote: remote.to_owned(), slug: fresh.slug.clone(), unshared_at: now() });
        write_json(&self.atlasgit_file("keep.json"), &book)?;

        let dst = remote_ref(&fresh.slug, id);
        if let Err(e) = self.push_shared(remote, "", &dst, Some(&fresh.wrapper), env) {
            // Put the keep state back the way it was; nothing happened on the remote.
            match previous {
                (Some(sha), entry) => {
                    let _ = self.git.run(&self.root, ["update-ref", keep_ref(id).as_str(), sha.as_str()]);
                    let mut book = self.keep_book();
                    match entry {
                        Some(en) => book.insert(id.to_owned(), en),
                        None => book.remove(id),
                    };
                    let _ = write_json(&self.atlasgit_file("keep.json"), &book);
                }
                (None, _) => self.forget_keep(id)?,
            }
            return Err(e);
        }
        let _ = self.git.run(&self.root, ["update-ref", "-d", fresh.ref_name.as_str()]);
        self.journal_shared(
            "shared_stash_unshare",
            format!("Unshare \"{}\" from {remote}", fresh.title),
            vec![id.to_owned(), remote.to_owned(), fresh.wrapper.clone()],
            vec![format!("git push --force-with-lease={dst}:{} {remote} :{dst}", fresh.wrapper)],
        )?;
        Ok(OpOutcome { head: self.head(), shared_stash: Some(self.outcome(id, remote)), ..Default::default() })
    }

    /// Your unshared stashes that can still be shared again.
    pub fn shared_stash_kept(&self) -> Result<Vec<KeptSharedStash>, OpError> {
        let out = self.git.run(&self.root, ["for-each-ref", "--format=%(objectname) %(refname)", KEEP_PREFIX])?;
        let refs: Vec<(String, String)> = String::from_utf8_lossy(&out)
            .lines()
            .filter_map(|l| {
                let (sha, name) = l.split_once(' ')?;
                Some((name.strip_prefix(KEEP_PREFIX)?.to_owned(), sha.to_owned()))
            })
            .collect();
        let me = self.local_email();
        let book = self.keep_book();
        let mut kept = vec![];
        for (id, sha) in refs {
            let Ok(v) = self.validate_shared_stash(&sha) else { continue };
            if me.as_deref() != Some(normalize_email(&v.meta.author_email).as_str()) {
                continue;
            }
            let entry = book.get(&id);
            kept.push(KeptSharedStash {
                slug: entry.map_or_else(|| author_slug(&v.author_name, &v.meta.author_email), |e| e.slug.clone()),
                remote: entry.map(|e| e.remote.clone()),
                unshared_at: entry.map_or(0, |e| e.unshared_at),
                id,
                wrapper: sha,
                stash: v.stash,
                title: v.meta.title,
                base_branch: v.meta.base_branch,
                created: v.meta.created,
                updated: v.meta.updated,
            });
        }
        kept.sort_by(|a, b| b.unshared_at.cmp(&a.unshared_at).then(a.id.cmp(&b.id)));
        Ok(kept)
    }

    /// Pushes a kept stash back to its ref (create-only: it must not exist on the remote now).
    pub(super) fn shared_reshare(&self, id: &str, remote: &str, env: &Env) -> Result<OpOutcome, OpError> {
        let _g = self.busy_remote()?;
        let who = self.identity()?;
        self.require_remote(remote)?;
        let sha = self.keep_sha(id).ok_or_else(stash_not_found)?;
        let v = self.validate_shared_stash(&sha).map_err(|_| invalid_stash())?;
        if normalize_email(&v.meta.author_email) != normalize_email(&who.email) {
            return Err(not_author());
        }
        let slug = self.keep_book().get(id).map_or_else(|| author_slug(&v.author_name, &v.meta.author_email), |e| e.slug.clone());
        let dst = remote_ref(&slug, id);
        self.push_shared(remote, &sha, &dst, Some(""), env)?;
        self.forget_keep(id)?;
        self.refresh_mirror(remote, &slug, id, &sha, env)?;
        self.journal_shared(
            "shared_stash_reshare",
            format!("Share \"{}\" again to {remote}", v.meta.title),
            vec![id.to_owned(), remote.to_owned(), sha.clone()],
            vec![format!("git push --force-with-lease={dst}: {remote} {sha}:{dst}")],
        )?;
        Ok(OpOutcome { head: self.head(), shared_stash: Some(self.outcome(id, remote)), ..Default::default() })
    }

    /// Forgets a kept copy for good (the wrapper becomes collectable by gc).
    pub(super) fn shared_discard_keep(&self, id: &str) -> Result<OpOutcome, OpError> {
        let sha = self.keep_sha(id).ok_or_else(stash_not_found)?;
        self.forget_keep(id)?;
        self.journal_shared(
            "shared_stash_discard_keep",
            "Discard an unshared stash".to_owned(),
            vec![id.to_owned(), sha],
            vec![format!("git update-ref -d {}", keep_ref(id))],
        )?;
        Ok(OpOutcome {
            head: self.head(),
            shared_stash: Some(SharedStashOutcome { id: Some(id.to_owned()), ..Default::default() }),
            ..Default::default()
        })
    }

    // ---------------------------------------------------------------- apply / branch

    fn has_commit(&self, sha: &str) -> bool {
        let spec = format!("{sha}^{{commit}}");
        self.git.run(&self.root, ["cat-file", "-e", spec.as_str()]).is_ok()
    }

    /// `git stash apply` needs the commit the stash was taken on. Tries its branch on the same remote once.
    fn ensure_base(&self, info: &SharedStashInfo, env: &Env) -> Result<(), OpError> {
        if self.has_commit(&info.base) {
            return Ok(());
        }
        if !info.base_branch.is_empty() {
            let spec = format!("refs/heads/{}", info.base_branch);
            let _ = self.git.run_status_env(
                &self.root,
                ["fetch", "--quiet", "--no-tags", "--no-recurse-submodules", info.remote.as_str(), spec.as_str()],
                env,
            );
        }
        if self.has_commit(&info.base) {
            Ok(())
        } else {
            Err(base_missing())
        }
    }

    fn nul_list(&self, args: &[&str]) -> Result<Vec<String>, OpError> {
        let out = self.git.run(&self.root, args)?;
        Ok(String::from_utf8_lossy(&out).split('\0').filter(|s| !s.is_empty()).map(str::to_owned).collect())
    }

    /// `DIRTY_TREE` before anything is touched: a file the stash changes (or creates) is already
    /// changed in the index or working tree. Git itself would apply part of a stash and restore the
    /// untracked files even when it then refuses the rest.
    fn check_no_overlap(&self, stash: &str) -> Result<(), OpError> {
        let base = format!("{stash}^1");
        let idx = format!("{stash}^2");
        let mut touched = self.nul_list(&["diff-tree", "-r", "--name-only", "-z", "--no-renames", base.as_str(), stash])?;
        touched.extend(self.nul_list(&["diff-tree", "-r", "--name-only", "-z", "--no-renames", base.as_str(), idx.as_str()])?);
        let third = format!("{stash}^3");
        let untracked = if self.git.run(&self.root, ["rev-parse", "--verify", "-q", third.as_str()]).is_ok() {
            self.nul_list(&["ls-tree", "-r", "--name-only", "-z", third.as_str()])?
        } else {
            vec![]
        };
        let status = self.nul_list(&["status", "--porcelain", "-z", "--untracked-files=all", "--no-renames"])?;
        let dirty: std::collections::HashSet<&str> = status.iter().filter_map(|e| e.get(3..)).collect();
        let clash = touched.iter().chain(&untracked).any(|p| dirty.contains(p.as_str()))
            || untracked.iter().any(|p| std::fs::symlink_metadata(self.root.join(p)).is_ok());
        if clash {
            Err(dirty_tree())
        } else {
            Ok(())
        }
    }

    fn run_apply(&self, stash: &str, index: bool) -> Result<(Option<i32>, String), OpError> {
        let mut args = vec!["stash", "apply"];
        if index {
            args.push("--index");
        }
        args.push(stash);
        let (code, out, err) = self.git.run_status(&self.root, args)?;
        Ok((code, format!("{out}\n{err}")))
    }

    /// Applies a stash commit like `git stash apply --index`; when the staged state cannot be
    /// restored it applies without `--index` and reports that. Caller holds `Lock::Index`.
    fn apply_stash_commit(&self, stash: &str) -> Result<(Applied, bool), OpError> {
        self.check_no_overlap(stash)?;
        let base = format!("{stash}^1");
        let idx = format!("{stash}^2");
        let staged_in_stash = matches!(self.git.run_status(&self.root, ["diff", "--quiet", base.as_str(), idx.as_str()])?.0, Some(1));
        let index_clean = matches!(self.git.run_status(&self.root, ["diff", "--cached", "--quiet"])?.0, Some(0));
        // With staged changes of its own, `git stash apply --index` fails and unstages them: skip it.
        let try_index = staged_in_stash && index_clean;
        let mut dropped = staged_in_stash && !index_clean;
        let (mut code, mut text) = self.run_apply(stash, try_index)?;
        let conflicted = |t: &str| t.contains("CONFLICT");
        if code != Some(0) && try_index && !conflicted(&text) && !looks_dirty(&text) {
            (code, text) = self.run_apply(stash, false)?;
            dropped = true;
        }
        if code == Some(0) {
            return Ok((Applied::Clean, dropped));
        }
        if conflicted(&text) {
            return Ok((Applied::Conflict, dropped || text.contains("Index was not unstashed")));
        }
        if looks_dirty(&text) {
            return Err(dirty_tree().with_detail(text));
        }
        Err(OpError::new("GIT_FAILED", "git stash apply failed.", Some("Commit or stash your current changes that overlap, then try again."))
            .with_detail(text))
    }

    pub(super) fn shared_apply(&self, id: &str, remote: &str, env: &Env) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let info = self.shared_entry(remote, id)?;
        if !info.valid {
            return Err(invalid_stash());
        }
        self.ensure_base(&info, env)?;
        let (state, index_dropped) = self.apply_stash_commit(&info.stash)?;
        self.journal_shared(
            "shared_stash_apply",
            format!("Apply shared stash \"{}\"", info.title),
            vec![id.to_owned(), remote.to_owned(), info.stash.clone()],
            vec![format!("git stash apply --index {}", info.stash)],
        )?;
        Ok(OpOutcome {
            head: self.head(),
            conflicted: matches!(state, Applied::Conflict),
            shared_stash: Some(SharedStashOutcome { index_dropped, ..self.outcome(id, remote) }),
            ..Default::default()
        })
    }

    fn restore_location(&self, branch: Option<&str>, head: &str) {
        let _ = match branch {
            Some(b) => self.git.run(&self.root, ["checkout", "-q", b, "--"]),
            None => self.git.run(&self.root, ["checkout", "-q", "--detach", head]),
        };
    }

    /// New branch at the stash's base, checked out, with the stash applied. Rolled back on failure.
    pub(super) fn shared_branch(&self, id: &str, remote: &str, name: &str, env: &Env) -> Result<OpOutcome, OpError> {
        self.valid_branch(name)?;
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.tip(name).is_some() {
            return Err(OpError::new("BRANCH_EXISTS", "A branch with that name already exists.", None));
        }
        let info = self.shared_entry(remote, id)?;
        if !info.valid {
            return Err(invalid_stash());
        }
        self.ensure_base(&info, env)?;
        let head = self.head().ok_or_else(|| OpError::new("INVALID_REF", "This repository has no commits yet.", None))?;
        let prev_branch = self.branch();
        self.check_no_overlap(&info.stash)?;

        self.git.run(&self.root, ["branch", name, info.base.as_str()])?;
        let undo_branch = |this: &Self| {
            this.restore_location(prev_branch.as_deref(), &head);
            let _ = this.git.run(&this.root, ["update-ref", "-d", format!("refs/heads/{name}").as_str()]);
        };
        match self.git.run_status(&self.root, ["checkout", "-q", name, "--"]) {
            Ok((Some(0), _, _)) => {}
            Ok((_, out, err)) => {
                undo_branch(self);
                let text = format!("{out}\n{err}");
                return Err(if looks_dirty(&text) {
                    dirty_tree().with_detail(text)
                } else {
                    OpError::new("GIT_FAILED", "git checkout failed.", None).with_detail(text)
                });
            }
            Err(e) => {
                undo_branch(self);
                return Err(e.into());
            }
        }
        let (state, index_dropped) = match self.apply_stash_commit(&info.stash) {
            Ok(r) => r,
            Err(e) => {
                undo_branch(self);
                return Err(e);
            }
        };
        let args = vec![name.to_owned(), info.base.clone(), prev_branch.clone().unwrap_or_else(|| head.clone()), prev_branch.is_some().to_string()];
        let mut out = self.record(
            "shared_stash_branch",
            format!("Create branch {name} from shared stash \"{}\"", info.title),
            args,
            Some(head),
            vec![format!("git branch {name} {}", info.base), format!("git checkout {name}"), format!("git stash apply --index {}", info.stash)],
        )?;
        out.conflicted = matches!(state, Applied::Conflict);
        out.shared_stash = Some(SharedStashOutcome { index_dropped, ..self.outcome(id, remote) });
        Ok(out)
    }

    /// Undo/redo of `shared_stash_branch` (see `branch.rs::apply_ref_entry`).
    pub(super) fn apply_shared_branch_entry(&self, e: &JournalEntry, forward: bool) -> Result<(), OpError> {
        let a = &e.args;
        let (name, base, prev, prev_is_branch) = (&a[0], &a[1], &a[2], a[3] == "true");
        let moved = || {
            OpError::new(
                "REF_MOVED",
                "The branch changed since that operation, so it cannot be undone or redone safely.",
                Some("Undo the newer operations first."),
            )
        };
        let at_prev = if prev_is_branch {
            self.branch().as_deref() == Some(prev.as_str())
        } else {
            self.branch().is_none() && self.head().as_deref() == Some(prev.as_str())
        };
        if forward {
            if self.tip(name).is_some() || !at_prev {
                return Err(moved());
            }
            self.git.run(&self.root, ["branch", name.as_str(), base.as_str()])?;
            self.git.run(&self.root, ["checkout", "-q", name.as_str(), "--"])?;
        } else {
            // Only while nothing was committed on the new branch. Changes from the stash stay as local changes.
            if self.branch().as_deref() != Some(name.as_str()) || self.head().as_deref() != Some(base.as_str()) {
                return Err(moved());
            }
            if prev_is_branch {
                self.git.run(&self.root, ["checkout", "-q", prev.as_str(), "--"])?;
            } else {
                self.git.run(&self.root, ["checkout", "-q", "--detach", prev.as_str()])?;
            }
            self.git.run(&self.root, ["update-ref", "-d", format!("refs/heads/{name}").as_str(), base.as_str()])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_comes_from_the_stash_message() {
        assert_eq!(branch_of_stash("WIP on main: 1a2b3c4 subject"), "main");
        assert_eq!(branch_of_stash("On feature/x: my note"), "feature/x");
        assert_eq!(branch_of_stash("WIP on (no branch): 1a2b3c4 subject"), "");
        assert_eq!(branch_of_stash("something else entirely"), "");
    }

    #[test]
    fn push_failures_map_to_stable_codes() {
        let hook = "!\tabc:refs/atlasgit/probe/x\t[remote rejected] (pre-receive hook declined)";
        assert_eq!(push_error("origin", hook.into(), false).code, "REMOTE_REJECTED");
        let race = "[remote rejected] (failed to update ref)\nremote: error: cannot lock ref 'r': is at a but expected b";
        assert_eq!(push_error("origin", race.into(), true).code, "STALE_SHARED_STASH");
        assert_eq!(push_error("origin", "[rejected] (stale info)".into(), true).code, "STALE_SHARED_STASH");
        let newer = "!\tabc:refs/atlasgit/shared-stash/a/b\t[remote rejected] (incorrect old value provided)\nremote: error: cannot lock ref 'r': is at a but expected b";
        assert_eq!(push_error("origin", newer.into(), true).code, "STALE_SHARED_STASH");
        assert_eq!(push_error("origin", hook.into(), true).code, "REMOTE_REJECTED");
        // Without a lease the same text is just a rejection by the host.
        assert_eq!(push_error("origin", race.into(), false).code, "REMOTE_REJECTED");
        assert_eq!(push_error("origin", "Permission denied (publickey).".into(), false).code, "AUTH_FAILED");
        assert_eq!(push_error("origin", "Could not resolve host: example.invalid".into(), false).code, "NETWORK_FAILED");
    }

    #[test]
    fn dirty_tree_markers() {
        assert!(looks_dirty("error: Your local changes to the following files would be overwritten by merge:\n\tf.txt"));
        assert!(looks_dirty("u.txt already exists, no checkout"));
        assert!(!looks_dirty("CONFLICT (content): Merge conflict in f.txt"));
    }
}
