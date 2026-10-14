// SPDX-License-Identifier: Apache-2.0
//! In-process commit/ref reads with gitoxide (feature `gix-reader`).
//! Produces the same output as `CliReader` (verified by tests); status and
//! anything not implemented here falls back to the CLI reader.

use std::collections::{BinaryHeap, HashMap};
use std::path::Path;

use ak_gitcli::parse::{CommitRecord, RefKind, RefRecord, StatusReport};
use ak_gitcli::Git;
use gix::bstr::ByteSlice;

use crate::read::{CliReader, ReadError, RepoReader};

pub struct GixReader {
    repo: gix::ThreadSafeRepository,
    cli: CliReader,
}

fn err(e: impl std::fmt::Display) -> ReadError {
    ReadError::Parse(ak_gitcli::parse::ParseError(format!("gix: {e}")))
}

/// `%s` in git: the first paragraph of the message, lines joined by spaces.
fn subject(message: &[u8]) -> String {
    let text = String::from_utf8_lossy(message);
    let para = text.trim_start_matches('\n').split("\n\n").next().unwrap_or("");
    para.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ")
}

impl GixReader {
    pub fn open(git: Git, path: &Path) -> Result<Self, ReadError> {
        let cli = CliReader::open(git, path)?;
        let repo = gix::open(cli.root()).map_err(err)?.into_sync();
        Ok(Self { repo, cli })
    }

    pub fn cli(&self) -> &CliReader {
        &self.cli
    }

    /// Commit ids of all branch/remote/tag tips (peeled) and HEAD, excluding refs/stash.
    fn tips(&self, repo: &gix::Repository) -> Result<Vec<gix::ObjectId>, ReadError> {
        let mut tips = Vec::new();
        let platform = repo.references().map_err(err)?;
        for r in platform.all().map_err(err)? {
            let mut r = r.map_err(err)?;
            if r.name().as_bstr() == "refs/stash" {
                continue;
            }
            if let Ok(id) = r.peel_to_id() {
                if let Ok(obj) = repo.find_object(id) {
                    if obj.kind == gix::object::Kind::Commit {
                        tips.push(id.detach());
                    }
                }
            }
        }
        if let Ok(id) = repo.head_id() {
            tips.push(id.detach());
        }
        tips.sort();
        tips.dedup();
        Ok(tips)
    }
}

struct Node {
    rec: CommitRecord,
    commit_time: i64,
    parents: Vec<gix::ObjectId>,
}

impl RepoReader for GixReader {
    fn commits(&self) -> Result<Vec<CommitRecord>, ReadError> {
        let repo = self.repo.to_thread_local();
        // 1. Collect every reachable commit.
        let mut nodes: HashMap<gix::ObjectId, Node> = HashMap::new();
        let mut stack = self.tips(&repo)?;
        while let Some(id) = stack.pop() {
            if nodes.contains_key(&id) {
                continue;
            }
            let commit = repo.find_commit(id).map_err(err)?;
            let c = commit.decode().map_err(err)?;
            let parents: Vec<gix::ObjectId> = c.parents().collect();
            let author = c.author().map_err(err)?;
            let committer = c.committer().map_err(err)?;
            let rec = CommitRecord {
                sha: id.to_string(),
                parents: parents.iter().map(|p| p.to_string()).collect(),
                author: author.name.to_str_lossy().into_owned(),
                email: author.email.to_str_lossy().into_owned(),
                time: author.time().map_err(err)?.seconds,
                summary: subject(c.message),
            };
            let commit_time = committer.time().map_err(err)?.seconds;
            stack.extend(parents.iter().copied());
            nodes.insert(id, Node { rec, commit_time, parents });
        }
        // 2. Topological order with newest committer date first (git --date-order):
        //    a commit is emitted only after all its children.
        let mut indegree: HashMap<gix::ObjectId, usize> = nodes.keys().map(|k| (*k, 0)).collect();
        for n in nodes.values() {
            for p in &n.parents {
                *indegree.get_mut(p).unwrap() += 1;
            }
        }
        // Max-heap on (commit time, then insertion order reversed to keep it stable).
        let mut seq = 0u64;
        let mut heap: BinaryHeap<(i64, std::cmp::Reverse<u64>, gix::ObjectId)> = BinaryHeap::new();
        let mut roots: Vec<_> = indegree.iter().filter(|(_, d)| **d == 0).map(|(k, _)| *k).collect();
        roots.sort_by_key(|k| std::cmp::Reverse(nodes[k].commit_time));
        for k in roots {
            heap.push((nodes[&k].commit_time, std::cmp::Reverse(seq), k));
            seq += 1;
        }
        let mut out = Vec::with_capacity(nodes.len());
        while let Some((_, _, id)) = heap.pop() {
            let n = &nodes[&id];
            for p in &n.parents {
                let d = indegree.get_mut(p).unwrap();
                *d -= 1;
                if *d == 0 {
                    heap.push((nodes[p].commit_time, std::cmp::Reverse(seq), *p));
                    seq += 1;
                }
            }
            out.push(n.rec.clone());
        }
        Ok(out)
    }

    fn refs(&self) -> Result<Vec<RefRecord>, ReadError> {
        let repo = self.repo.to_thread_local();
        let head_ref = repo.head_name().ok().flatten().map(|n| n.as_bstr().to_string());
        let mut out = Vec::new();
        for r in repo.references().map_err(err)?.all().map_err(err)? {
            let mut r = r.map_err(err)?;
            let name = r.name().as_bstr().to_string();
            // for-each-ref lists symbolic refs too (e.g. refs/remotes/origin/HEAD).
            let target = match r.peel_to_id() {
                Ok(id) => id.detach().to_string(),
                Err(_) => continue,
            };
            let (kind, short) = if let Some(s) = name.strip_prefix("refs/heads/") {
                (RefKind::Head, s.to_owned())
            } else if let Some(s) = name.strip_prefix("refs/remotes/") {
                (RefKind::Remote, s.to_owned())
            } else if let Some(s) = name.strip_prefix("refs/tags/") {
                (RefKind::Tag, s.to_owned())
            } else if name == "refs/stash" {
                (RefKind::Stash, "stash".to_owned())
            } else {
                (RefKind::Other, name.clone())
            };
            out.push(RefRecord { is_head: head_ref.as_deref() == Some(name.as_str()), short, kind, target, name });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn status(&self) -> Result<StatusReport, ReadError> {
        self.cli.status()
    }

    fn head(&self) -> Result<Option<String>, ReadError> {
        self.cli.head()
    }
}
