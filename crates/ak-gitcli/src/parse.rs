// SPDX-License-Identifier: Apache-2.0
//! Parsers for git machine formats. Inputs come straight from git stdout.

use serde::Serialize;

/// Field separator used inside our `--format` strings (ASCII unit separator).
pub const FS: char = '\u{1f}';

/// `git log` arguments that produce what [`parse_commits`] expects.
pub const COMMIT_FORMAT: &str = "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%s";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct CommitRecord {
    pub sha: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub summary: String,
}

#[derive(Debug, thiserror::Error, PartialEq)]
#[error("malformed git output: {0}")]
pub struct ParseError(pub String);

/// Parses `git log -z <COMMIT_FORMAT>` output (records separated by NUL).
pub fn parse_commits(out: &[u8]) -> Result<Vec<CommitRecord>, ParseError> {
    let text = String::from_utf8_lossy(out);
    text.split('\0')
        .map(|r| r.trim_start_matches('\n'))
        .filter(|r| !r.is_empty())
        .map(|rec| {
            let mut f = rec.splitn(6, FS);
            let mut next = |name: &str| f.next().ok_or_else(|| ParseError(format!("missing {name} in {rec:?}")));
            let sha = next("sha")?.to_owned();
            let parents = next("parents")?.split_whitespace().map(str::to_owned).collect();
            let author = next("author")?.to_owned();
            let email = next("email")?.to_owned();
            let time = next("time")?.parse().map_err(|_| ParseError(format!("bad time in {rec:?}")))?;
            let summary = next("summary")?.to_owned();
            if sha.len() < 40 {
                return Err(ParseError(format!("bad sha {sha:?}")));
            }
            Ok(CommitRecord { sha, parents, author, email, time, summary })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RefKind {
    Head,
    Remote,
    Tag,
    Stash,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefRecord {
    pub name: String,
    pub short: String,
    pub kind: RefKind,
    /// Commit the ref points to (peeled for annotated tags).
    pub target: String,
    pub is_head: bool,
}

/// `git for-each-ref` arguments that produce what [`parse_refs`] expects.
pub const REF_FORMAT: &str = "--format=%(refname)%1f%(objectname)%1f%(*objectname)%1f%(HEAD)";

pub fn parse_refs(out: &[u8]) -> Result<Vec<RefRecord>, ParseError> {
    String::from_utf8_lossy(out)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let f: Vec<&str> = line.split(FS).collect();
            if f.len() != 4 {
                return Err(ParseError(format!("ref line {line:?}")));
            }
            let name = f[0].to_owned();
            let (kind, short) = if let Some(s) = name.strip_prefix("refs/heads/") {
                (RefKind::Head, s)
            } else if let Some(s) = name.strip_prefix("refs/remotes/") {
                (RefKind::Remote, s)
            } else if let Some(s) = name.strip_prefix("refs/tags/") {
                (RefKind::Tag, s)
            } else if name == "refs/stash" {
                (RefKind::Stash, "stash")
            } else {
                (RefKind::Other, name.as_str())
            };
            let target = if f[2].is_empty() { f[1] } else { f[2] }.to_owned();
            Ok(RefRecord { short: short.to_owned(), kind, target, is_head: f[3] == "*", name })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusEntry {
    pub path: String,
    pub orig_path: Option<String>,
    /// Index (staged) status letter, `.` when unchanged.
    pub index: char,
    /// Worktree status letter, `.` when unchanged.
    pub worktree: char,
    pub conflicted: bool,
    pub untracked: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct StatusReport {
    pub branch: Option<String>,
    pub oid: Option<String>,
    /// Tracking branch (e.g. `origin/main`) and commits ahead of / behind it.
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub entries: Vec<StatusEntry>,
}

/// Parses `git status --porcelain=v2 -z --branch`.
pub fn parse_status_v2(out: &[u8]) -> Result<StatusReport, ParseError> {
    let text = String::from_utf8_lossy(out);
    let mut parts = text.split('\0').filter(|p| !p.is_empty());
    let mut rep = StatusReport::default();
    while let Some(p) = parts.next() {
        let bad = || ParseError(format!("status entry {p:?}"));
        match p.as_bytes()[0] {
            b'#' => {
                if let Some(v) = p.strip_prefix("# branch.head ") {
                    rep.branch = (v != "(detached)").then(|| v.to_owned());
                } else if let Some(v) = p.strip_prefix("# branch.oid ") {
                    rep.oid = (v != "(initial)").then(|| v.to_owned());
                } else if let Some(v) = p.strip_prefix("# branch.upstream ") {
                    rep.upstream = Some(v.to_owned());
                } else if let Some(v) = p.strip_prefix("# branch.ab ") {
                    let mut ab = v.split(' ').map(|n| n.trim_start_matches(['+', '-']).parse().unwrap_or(0));
                    rep.ahead = ab.next().unwrap_or(0);
                    rep.behind = ab.next().unwrap_or(0);
                }
            }
            b'1' | b'2' | b'u' => {
                // 1: 8 fields + path; 2: 9 fields + path, then orig path; u: 10 fields + path.
                let fields = match p.as_bytes()[0] { b'1' => 8, b'2' => 9, _ => 10 };
                let mut f = p.splitn(fields + 1, ' ');
                f.next();
                let xy = f.next().ok_or_else(bad)?;
                let path = f.nth(fields - 2).ok_or_else(bad)?.to_owned();
                let mut c = xy.chars();
                let (index, worktree) = (c.next().ok_or_else(bad)?, c.next().ok_or_else(bad)?);
                let orig_path = if p.starts_with('2') { Some(parts.next().ok_or_else(bad)?.to_owned()) } else { None };
                rep.entries.push(StatusEntry { path, orig_path, index, worktree, conflicted: p.starts_with('u'), untracked: false });
            }
            b'?' => rep.entries.push(StatusEntry {
                path: p[2..].to_owned(),
                orig_path: None,
                index: '.',
                worktree: '?',
                conflicted: false,
                untracked: true,
            }),
            b'!' => {}
            _ => return Err(bad()),
        }
    }
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[test]
    fn commits_with_merge_and_root() {
        let raw = format!("{A}\u{1f}{B} {B}\u{1f}Ada\u{1f}a@x\u{1f}17\u{1f}Merge x\0{B}\u{1f}\u{1f}Bo\u{1f}b@x\u{1f}16\u{1f}Root: with\u{1f}sep\0");
        let c = parse_commits(raw.as_bytes()).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].parents.len(), 2);
        assert!(c[1].parents.is_empty());
        assert_eq!(c[1].summary, "Root: with\u{1f}sep");
        assert!(parse_commits(b"xyz\x1f\0").is_err());
    }

    #[test]
    fn refs_peel_annotated_tags() {
        let raw = format!(
            "refs/heads/main\u{1f}{A}\u{1f}\u{1f}*\nrefs/tags/v1\u{1f}{B}\u{1f}{A}\u{1f} \nrefs/remotes/origin/x\u{1f}{A}\u{1f}\u{1f} \nrefs/stash\u{1f}{B}\u{1f}\u{1f} \n"
        );
        let r = parse_refs(raw.as_bytes()).unwrap();
        assert!(r[0].is_head && r[0].kind == RefKind::Head && r[0].short == "main");
        assert_eq!(r[1].target, A);
        assert_eq!(r[2].short, "origin/x");
        assert_eq!(r[3].kind, RefKind::Stash);
    }

    #[test]
    fn status_v2_all_entry_kinds() {
        let raw = "# branch.oid abc\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
1 .M N... 100644 100644 100644 h1 h2 src/a b.txt\0\
2 R. N... 100644 100644 100644 h1 h2 R100 new.txt\0old.txt\0\
u UU N... 100644 100644 100644 100644 h1 h2 h3 c.txt\0\
? untracked file\0";
        let s = parse_status_v2(raw.as_bytes()).unwrap();
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!((s.upstream.as_deref(), s.ahead, s.behind), (Some("origin/main"), 2, 1));
        assert_eq!(s.entries[0].path, "src/a b.txt");
        assert_eq!((s.entries[0].index, s.entries[0].worktree), ('.', 'M'));
        assert_eq!(s.entries[1].orig_path.as_deref(), Some("old.txt"));
        assert_eq!(s.entries[1].path, "new.txt");
        assert!(s.entries[2].conflicted && s.entries[2].path == "c.txt");
        assert!(s.entries[3].untracked && s.entries[3].path == "untracked file");
    }
}
