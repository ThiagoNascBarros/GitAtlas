// SPDX-License-Identifier: Apache-2.0
//! Shared Stash model (docs/specs/shared-stash.md): author slug, wrapper-commit trailers,
//! ref names, validation of what arrives from a remote, and listing of the fetched mirror refs.
//! The operations that talk to the network live in `shared_stash_ops.rs`.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::{Engine, OpError};

/// Where a shared stash lives on the remote: `<REMOTE_PREFIX><slug>/<id>`.
pub const REMOTE_PREFIX: &str = "refs/atlasgit/shared-stash/";
/// Local mirrors, written only by an explicit fetch: `<MIRROR_ROOT><remote>/shared-stash/<slug>/<id>`.
pub const MIRROR_ROOT: &str = "refs/atlasgit/remotes/";
/// Local copies of stashes their author unshared, so Reshare can bring them back.
pub const KEEP_PREFIX: &str = "refs/atlasgit/keep/";
/// Throwaway refs used to find out whether a host accepts custom refs.
pub const PROBE_PREFIX: &str = "refs/atlasgit/probe/";

pub const MAX_TITLE: usize = 120;
pub const MAX_SLUG: usize = 40;
const MARK: &str = "AtlasGit-Shared-Stash";

// ---------------------------------------------------------------- slug

/// ASCII replacement for a lowercase accented letter; `None` when there is none (it then acts as a separator).
fn fold(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' | 'ǎ' => "a",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ď' | 'đ' | 'ð' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' | 'ǐ' => "i",
        'ĵ' => "j",
        'ķ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' | 'ŉ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' | 'ǒ' => "o",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'ţ' | 'ť' | 'ŧ' => "t",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' | 'ǔ' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        'ß' => "ss",
        'æ' => "ae",
        'œ' => "oe",
        'þ' => "th",
        _ => return None,
    })
}

/// Lowercase ASCII `[a-z0-9-]`, accents stripped, every other run collapsed into one `-`, no
/// leading/trailing `-`, at most [`MAX_SLUG`] characters. Empty when nothing usable is left.
fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    let push = |piece: &str, out: &mut String, gap: &mut bool| {
        for ch in piece.chars() {
            if ch.is_ascii_alphanumeric() {
                if *gap && !out.is_empty() {
                    out.push('-');
                }
                *gap = false;
                out.push(ch.to_ascii_lowercase());
            } else {
                *gap = true;
            }
        }
    };
    for c in input.chars() {
        // Combining marks (decomposed accents) vanish without splitting the word.
        if ('\u{300}'..='\u{36f}').contains(&c) {
            continue;
        }
        for lc in c.to_lowercase() {
            match fold(lc) {
                Some(f) => push(f, &mut out, &mut gap),
                None => push(lc.encode_utf8(&mut [0; 4]), &mut out, &mut gap),
            }
        }
    }
    out.truncate(MAX_SLUG);
    out.trim_end_matches('-').to_owned()
}

/// Readable, display-only name for the ref (spec §4): from `user.name`, else the email local part.
/// Always a valid ref component; never empty.
pub fn author_slug(name: &str, email: &str) -> String {
    let s = slugify(name);
    if !s.is_empty() {
        return s;
    }
    let s = slugify(email.split('@').next().unwrap_or(""));
    if s.is_empty() {
        "user".to_owned()
    } else {
        s
    }
}

// ---------------------------------------------------------------- wrapper message

/// What the wrapper commit message carries (spec §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedStashMeta {
    pub title: String,
    pub author_email: String,
    /// Commit the stash was taken on.
    pub base: String,
    /// Branch it was taken on; empty when detached.
    pub base_branch: String,
    pub created: i64,
    pub updated: i64,
}

/// One line, collapsed whitespace, at most [`MAX_TITLE`] characters; never empty.
pub fn clean_title(title: &str) -> String {
    let one_line = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = one_line.chars().take(MAX_TITLE).collect();
    let cut = cut.trim_end().to_owned();
    if cut.is_empty() {
        "Shared stash".to_owned()
    } else {
        cut
    }
}

fn one_line(v: &str) -> String {
    v.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_hex_sha(s: &str) -> bool {
    matches!(s.len(), 40 | 64) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

impl SharedStashMeta {
    pub fn format_message(&self) -> String {
        format!(
            "{}\n\n{MARK}: 1\nAtlasGit-Author-Email: {}\nAtlasGit-Base: {}\nAtlasGit-Base-Branch: {}\nAtlasGit-Created: {}\nAtlasGit-Updated: {}\n",
            clean_title(&self.title),
            one_line(&self.author_email),
            self.base,
            one_line(&self.base_branch),
            self.created,
            self.updated
        )
    }

    /// `None` unless the message is a Shared Stash wrapper with every required trailer.
    pub fn parse_message(message: &str) -> Option<Self> {
        let msg = message.trim_end_matches(['\n', '\r']);
        let (head, block) = msg.rsplit_once("\n\n")?;
        let title = head.lines().next()?.trim();
        if title.is_empty() {
            return None;
        }
        let mut t: HashMap<&str, &str> = HashMap::new();
        for line in block.lines() {
            if let Some((k, v)) = line.split_once(':') {
                t.entry(k.trim()).or_insert_with(|| v.trim());
            }
        }
        if t.get(MARK) != Some(&"1") {
            return None;
        }
        let email = t.get("AtlasGit-Author-Email").filter(|e| !e.is_empty())?;
        let base = t.get("AtlasGit-Base").filter(|b| is_hex_sha(b))?;
        Some(Self {
            title: clean_title(title),
            author_email: (*email).to_owned(),
            base: (*base).to_owned(),
            base_branch: t.get("AtlasGit-Base-Branch").copied().unwrap_or("").to_owned(),
            created: t.get("AtlasGit-Created")?.parse().ok()?,
            updated: t.get("AtlasGit-Updated")?.parse().ok()?,
        })
    }
}

/// Emails are compared trimmed and lowercased.
pub fn normalize_email(e: &str) -> String {
    e.trim().to_lowercase()
}

// ---------------------------------------------------------------- ref names

pub fn remote_ref(slug: &str, id: &str) -> String {
    format!("{REMOTE_PREFIX}{slug}/{id}")
}

pub fn mirror_prefix(remote: &str) -> String {
    format!("{MIRROR_ROOT}{remote}/shared-stash/")
}

pub fn mirror_ref(remote: &str, slug: &str, id: &str) -> String {
    format!("{}{slug}/{id}", mirror_prefix(remote))
}

/// Refspec that mirrors every shared stash of `remote` (used with `fetch --prune`).
pub fn mirror_refspec(remote: &str) -> String {
    format!("+{REMOTE_PREFIX}*:{}*", mirror_prefix(remote))
}

pub fn keep_ref(id: &str) -> String {
    format!("{KEEP_PREFIX}{id}")
}

pub fn probe_ref(id: &str) -> String {
    format!("{PROBE_PREFIX}{id}")
}

fn component(s: &str) -> bool {
    !s.is_empty() && !s.contains('/')
}

/// `refs/atlasgit/shared-stash/<slug>/<id>` -> `(slug, id)`.
pub fn parse_remote_ref(r: &str) -> Option<(&str, &str)> {
    let (slug, id) = r.strip_prefix(REMOTE_PREFIX)?.split_once('/')?;
    (component(slug) && component(id)).then_some((slug, id))
}

/// `refs/atlasgit/remotes/<remote>/shared-stash/<slug>/<id>` -> `(remote, slug, id)`.
/// Remote names may contain `/`, so the ref is read from the right.
pub fn parse_mirror_ref(r: &str) -> Option<(&str, &str, &str)> {
    let rest = r.strip_prefix(MIRROR_ROOT)?;
    let (rest, id) = rest.rsplit_once('/')?;
    let (rest, slug) = rest.rsplit_once('/')?;
    let remote = rest.strip_suffix("/shared-stash")?;
    (component(id) && component(slug) && !remote.is_empty()).then_some((remote, slug, id))
}

// ---------------------------------------------------------------- reading from the repository

/// A shared stash as the tab shows it. Invalid rows keep what could be read so they can be greyed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SharedStashInfo {
    pub remote: String,
    /// Display-only part of the ref, fixed when it was published.
    pub slug: String,
    pub id: String,
    /// The local mirror ref this was read from.
    pub ref_name: String,
    /// The wrapper commit.
    pub wrapper: String,
    /// The stash commit (the wrapper's parent); empty when invalid.
    pub stash: String,
    pub title: String,
    pub author_name: String,
    pub author_email: String,
    pub base: String,
    pub base_branch: String,
    pub created: i64,
    pub updated: i64,
    /// The stash carries untracked files (a third parent).
    pub has_untracked: bool,
    /// The trailer email is the local `user.email` (normalized).
    pub mine: bool,
    /// The wrapper carries a valid signature whose signer matches the trailer email.
    pub verified: bool,
    pub valid: bool,
    pub invalid_reason: Option<String>,
}

/// A wrapper that passed validation (spec §4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidShared {
    pub meta: SharedStashMeta,
    pub stash: String,
    pub author_name: String,
    pub has_untracked: bool,
    /// The wrapper commit has a signature header (not yet checked).
    pub signed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidShared {
    pub reason: String,
}

fn invalid(reason: &str) -> InvalidShared {
    InvalidShared { reason: reason.to_owned() }
}

#[derive(Debug)]
struct RawCommit {
    parents: Vec<String>,
    author_name: String,
    message: String,
    signed: bool,
}

enum Obj {
    Missing,
    Other,
    Commit(RawCommit),
}

impl RawCommit {
    fn parse(raw: &[u8]) -> Self {
        let text = String::from_utf8_lossy(raw);
        let (head, message) = text.split_once("\n\n").unwrap_or((&text, ""));
        let (mut parents, mut author_name, mut signed) = (vec![], String::new(), false);
        for line in head.lines() {
            if let Some(p) = line.strip_prefix("parent ") {
                parents.push(p.trim().to_owned());
            } else if let Some(a) = line.strip_prefix("author ") {
                author_name = a.split(" <").next().unwrap_or("").trim().to_owned();
            } else if line.starts_with("gpgsig") {
                signed = true;
            }
        }
        Self { parents, author_name, message: message.to_owned(), signed }
    }
}

/// Parses `git cat-file --batch` output for `wanted` (same order as the input).
fn parse_batch(out: &[u8], wanted: &[String]) -> HashMap<String, Obj> {
    let mut map = HashMap::new();
    let mut pos = 0;
    for sha in wanted {
        let Some(nl) = out[pos..].iter().position(|b| *b == b'\n') else { break };
        let header = String::from_utf8_lossy(&out[pos..pos + nl]).into_owned();
        pos += nl + 1;
        let mut f = header.split(' ');
        let (_, kind, size) = (f.next(), f.next().unwrap_or(""), f.next().and_then(|n| n.parse::<usize>().ok()));
        let Some(size) = size.filter(|_| kind != "missing") else {
            map.insert(sha.clone(), Obj::Missing);
            continue;
        };
        let end = (pos + size).min(out.len());
        let body = &out[pos..end];
        pos = (end + 1).min(out.len());
        map.insert(sha.clone(), if kind == "commit" { Obj::Commit(RawCommit::parse(body)) } else { Obj::Other });
    }
    map
}

impl Engine {
    pub fn git_dir(&self) -> &std::path::Path {
        &self.git_dir
    }

    /// Local `user.email`, normalized; `None` when unset or empty.
    pub(super) fn local_email(&self) -> Option<String> {
        self.config_value("user.email").map(|e| normalize_email(&e))
    }

    pub(super) fn config_value(&self, key: &str) -> Option<String> {
        let out = self.git.run(&self.root, ["config", "--get", key]).ok()?;
        let v = String::from_utf8_lossy(&out).trim().to_owned();
        (!v.is_empty()).then_some(v)
    }

    fn read_objects(&self, shas: &[String]) -> Result<HashMap<String, Obj>, OpError> {
        let mut seen = HashSet::new();
        let wanted: Vec<String> = shas.iter().filter(|s| seen.insert(s.as_str())).cloned().collect();
        if wanted.is_empty() {
            return Ok(HashMap::new());
        }
        let input = wanted.join("\n") + "\n";
        let out = self.git.run_with_input(&self.root, ["cat-file", "--batch"], input.as_bytes())?;
        Ok(parse_batch(&out, &wanted))
    }

    /// Checks wrappers (trailers) and what they point at (stash shape) in three batched reads.
    fn validate_many(&self, wrappers: &[String]) -> Result<HashMap<String, Result<ValidShared, InvalidShared>>, OpError> {
        let objs = self.read_objects(wrappers)?;
        let mut results: HashMap<String, Result<ValidShared, InvalidShared>> = HashMap::new();
        let mut pending: Vec<(String, SharedStashMeta, String, String, bool)> = vec![];
        for sha in wrappers {
            let Some(Obj::Commit(c)) = objs.get(sha) else {
                results.insert(sha.clone(), Err(invalid("the ref does not point at a commit")));
                continue;
            };
            let Some(meta) = SharedStashMeta::parse_message(&c.message) else {
                results.insert(sha.clone(), Err(invalid("not a Shared Stash wrapper")));
                continue;
            };
            let [stash] = c.parents.as_slice() else {
                results.insert(sha.clone(), Err(invalid("the wrapper must have exactly one parent, the stash")));
                continue;
            };
            pending.push((sha.clone(), meta, stash.clone(), c.author_name.clone(), c.signed));
        }
        let stash_objs = self.read_objects(&pending.iter().map(|p| p.2.clone()).collect::<Vec<_>>())?;
        let mut extra: Vec<String> = vec![];
        for p in &pending {
            if let Some(Obj::Commit(sc)) = stash_objs.get(&p.2) {
                extra.extend(sc.parents.iter().skip(1).cloned());
            }
        }
        let extra_objs = self.read_objects(&extra)?;
        for (sha, meta, stash, author_name, signed) in pending {
            let verdict = match stash_objs.get(&stash) {
                Some(Obj::Commit(sc)) => {
                    let shaped = matches!(sc.parents.len(), 2 | 3);
                    if !shaped {
                        Err(invalid("the parent is not a stash commit"))
                    } else if sc.parents[0] != meta.base {
                        Err(invalid("the stash was not taken on the base the wrapper names"))
                    } else if !sc.parents[1..].iter().all(|x| matches!(extra_objs.get(x), Some(Obj::Commit(_)))) {
                        Err(invalid("the stash data is incomplete"))
                    } else {
                        Ok(ValidShared { has_untracked: sc.parents.len() == 3, meta, stash, author_name, signed })
                    }
                }
                Some(Obj::Missing) | None => Err(invalid("the stash commit is missing")),
                Some(Obj::Other) => Err(invalid("the parent is not a stash commit")),
            };
            results.insert(sha, verdict);
        }
        Ok(results)
    }

    /// Spec §4.1: the wrapper parses and its parent is stash-shaped (`^1 == Base`, `^2` present).
    pub fn validate_shared_stash(&self, wrapper: &str) -> Result<ValidShared, InvalidShared> {
        let key = wrapper.to_owned();
        match self.validate_many(std::slice::from_ref(&key)) {
            Ok(mut m) => m.remove(&key).unwrap_or_else(|| Err(invalid("not found"))),
            Err(e) => Err(InvalidShared { reason: e.message }),
        }
    }

    /// True when `git verify-commit` accepts the signature and the signer is the trailer's author.
    /// False whenever it cannot be checked (no gpg/ssh tooling, unknown key, read-only Git).
    fn signature_matches(&self, wrapper: &str, email: &str) -> bool {
        let good = matches!(self.git.run_status(&self.root, ["verify-commit", wrapper]), Ok((Some(0), _, _)));
        if !good {
            return false;
        }
        let Ok(signer) = self.git.run(&self.root, ["log", "-1", "--format=%GS", wrapper]) else { return false };
        let signer = normalize_email(&String::from_utf8_lossy(&signer));
        let email = normalize_email(email);
        signer == email || signer.contains(&format!("<{email}>"))
    }

    fn mirror_refs(&self, remote: Option<&str>) -> Result<Vec<(String, String)>, OpError> {
        let prefix = remote.map_or_else(|| MIRROR_ROOT.to_owned(), mirror_prefix);
        let out = self.git.run(&self.root, ["for-each-ref", "--format=%(objectname) %(refname)", prefix.as_str()])?;
        Ok(String::from_utf8_lossy(&out).lines().filter_map(|l| l.split_once(' ').map(|(sha, name)| (name.to_owned(), sha.to_owned()))).collect())
    }

    /// Every shared stash mirrored from the repository's remotes (or just from `remote`).
    pub(super) fn shared_stash_entries(&self, remote: Option<&str>) -> Result<Vec<SharedStashInfo>, OpError> {
        let existing: HashSet<String> = self.remotes()?.into_iter().collect();
        let refs: Vec<(String, String)> = self.mirror_refs(remote)?;
        let parsed: Vec<(String, String, String, String, String)> = refs
            .into_iter()
            .filter_map(|(name, sha)| {
                let (r, slug, id) = parse_mirror_ref(&name)?;
                existing.contains(r).then(|| (r.to_owned(), slug.to_owned(), id.to_owned(), name.clone(), sha))
            })
            .collect();
        let verdicts = self.validate_many(&parsed.iter().map(|p| p.4.clone()).collect::<Vec<_>>())?;
        let me = self.local_email();
        let mut out: Vec<SharedStashInfo> = parsed
            .into_iter()
            .map(|(remote, slug, id, ref_name, wrapper)| {
                let base = SharedStashInfo {
                    remote,
                    slug,
                    id: id.clone(),
                    ref_name,
                    wrapper: wrapper.clone(),
                    stash: String::new(),
                    title: id,
                    author_name: String::new(),
                    author_email: String::new(),
                    base: String::new(),
                    base_branch: String::new(),
                    created: 0,
                    updated: 0,
                    has_untracked: false,
                    mine: false,
                    verified: false,
                    valid: false,
                    invalid_reason: None,
                };
                match verdicts.get(&wrapper) {
                    Some(Ok(v)) => SharedStashInfo {
                        stash: v.stash.clone(),
                        title: v.meta.title.clone(),
                        author_name: v.author_name.clone(),
                        author_email: v.meta.author_email.clone(),
                        base: v.meta.base.clone(),
                        base_branch: v.meta.base_branch.clone(),
                        created: v.meta.created,
                        updated: v.meta.updated,
                        has_untracked: v.has_untracked,
                        mine: me.as_deref() == Some(normalize_email(&v.meta.author_email).as_str()),
                        verified: v.signed && self.signature_matches(&wrapper, &v.meta.author_email),
                        valid: true,
                        ..base
                    },
                    other => SharedStashInfo {
                        invalid_reason: Some(other.and_then(|r| r.as_ref().err()).map_or_else(|| "unreadable".to_owned(), |e| e.reason.clone())),
                        ..base
                    },
                }
            })
            .collect();
        out.sort_by(|a, b| a.remote.cmp(&b.remote).then(b.updated.cmp(&a.updated)).then(a.id.cmp(&b.id)));
        Ok(out)
    }

    /// Shared stashes read from the local mirrors of all remotes. Needs no network.
    pub fn shared_stashes(&self) -> Result<Vec<SharedStashInfo>, OpError> {
        self.shared_stash_entries(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_basics() {
        assert_eq!(author_slug("Thiago Barros", "t@x.io"), "thiago-barros");
        assert_eq!(author_slug("  José  da Silva-Ñandú ", ""), "jose-da-silva-nandu");
        assert_eq!(author_slug("Müller & Søn", ""), "muller-son");
        assert_eq!(author_slug("Straße", ""), "strasse");
        // decomposed accent (e + U+0301) must not split the word
        assert_eq!(author_slug("Jose\u{301} Lopez", ""), "jose-lopez");
        assert_eq!(author_slug("a..b/c~d^e:f?g*h[i\\j@k{l}", ""), "a-b-c-d-e-f-g-h-i-j-k-l");
    }

    #[test]
    fn slug_falls_back_to_email_then_user() {
        assert_eq!(author_slug("", "Jane.Doe+work@example.com"), "jane-doe-work");
        assert_eq!(author_slug("   ", "x@y"), "x");
        assert_eq!(author_slug("日本語", "山田@example.com"), "user");
        assert_eq!(author_slug("", ""), "user");
    }

    #[test]
    fn slug_is_capped_and_never_ends_with_a_dash() {
        let s = author_slug(&format!("{} {}", "a".repeat(39), "bcd"), "");
        assert_eq!(s, "a".repeat(39), "cut at 40 leaves a trailing dash that is trimmed");
        assert_eq!(author_slug(&"x".repeat(100), "").len(), MAX_SLUG);
    }

    /// Tiny deterministic generator so the property-style tests need no extra dependency.
    pub(crate) struct Rng(pub u64);
    impl Rng {
        pub(crate) fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        pub(crate) fn text(&mut self, alphabet: &[char], max: usize) -> String {
            let n = (self.next() % (max as u64 + 1)) as usize;
            (0..n).map(|_| alphabet[(self.next() % alphabet.len() as u64) as usize]).collect()
        }
    }

    const NASTY: &[char] = &[
        'a', 'Z', '7', ' ', '\t', '\n', '.', '/', '\\', '~', '^', ':', '?', '*', '[', '@', '{', '}', '-', '_', 'é', 'Ç', 'ß', 'ø', 'ž', '日',
        '\u{301}', '😀', '\u{0}', '\u{7f}', '"', '\'', '#', '$',
    ];

    #[test]
    fn slug_is_always_a_valid_ref_component() {
        let git = ak_gitcli::Git::discover(ak_gitcli::Mode::ReadOnly).unwrap();
        let mut rng = Rng(0x9E3779B97F4A7C15);
        for _ in 0..200 {
            let (name, email) = (rng.text(NASTY, 60), rng.text(NASTY, 20));
            let s = author_slug(&name, &email);
            assert!(!s.is_empty() && s.len() <= MAX_SLUG, "{name:?} -> {s:?}");
            assert!(s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'), "{s:?}");
            assert!(!s.starts_with('-') && !s.ends_with('-') && !s.contains("--"), "{s:?}");
            let r = remote_ref(&s, "1a2b-3");
            let ok = std::process::Command::new(git_exe()).args(["check-ref-format", &r]).status().unwrap().success();
            assert!(ok, "git rejects {r:?}");
        }
        drop(git);
    }

    fn git_exe() -> std::ffi::OsString {
        std::env::var_os("AK_GIT").unwrap_or_else(|| "git".into())
    }

    fn meta() -> SharedStashMeta {
        SharedStashMeta {
            title: "Fix the thing".into(),
            author_email: "a@example.com".into(),
            base: "a".repeat(40),
            base_branch: "feature/x".into(),
            created: 1_700_000_000,
            updated: 1_700_000_500,
        }
    }

    #[test]
    fn message_format_matches_the_spec() {
        assert_eq!(
            meta().format_message(),
            format!(
                "Fix the thing\n\nAtlasGit-Shared-Stash: 1\nAtlasGit-Author-Email: a@example.com\nAtlasGit-Base: {}\nAtlasGit-Base-Branch: feature/x\nAtlasGit-Created: 1700000000\nAtlasGit-Updated: 1700000500\n",
                "a".repeat(40)
            )
        );
    }

    #[test]
    fn message_round_trips_including_empty_base_branch() {
        let mut m = meta();
        assert_eq!(SharedStashMeta::parse_message(&m.format_message()), Some(m.clone()));
        m.base_branch.clear();
        assert_eq!(SharedStashMeta::parse_message(&m.format_message()), Some(m));
    }

    #[test]
    fn title_is_one_line_capped_and_never_empty() {
        assert_eq!(clean_title("  two\n lines\ttabbed  "), "two lines tabbed");
        assert_eq!(clean_title(&"é".repeat(300)).chars().count(), MAX_TITLE);
        assert_eq!(clean_title(" \n\t"), "Shared stash");
        let mut m = meta();
        m.title = "line one\nAtlasGit-Author-Email: evil@example.com".into();
        let back = SharedStashMeta::parse_message(&m.format_message()).unwrap();
        assert_eq!(back.author_email, "a@example.com", "a title cannot inject trailers");
        assert!(!back.title.contains('\n'));
    }

    #[test]
    fn message_round_trip_property() {
        let mut rng = Rng(42);
        for _ in 0..300 {
            let m = SharedStashMeta {
                title: clean_title(&rng.text(NASTY, 200)),
                author_email: format!("{}@x.io", slugify(&rng.text(NASTY, 12)).replace('-', "") + "u"),
                base: (0..40).map(|_| char::from_digit((rng.next() % 16) as u32, 16).unwrap()).collect(),
                base_branch: one_line(&rng.text(&['a', 'b', '/', '-', '_'], 20)),
                created: (rng.next() % 4_000_000_000) as i64,
                updated: (rng.next() % 4_000_000_000) as i64,
            };
            assert_eq!(SharedStashMeta::parse_message(&m.format_message()).as_ref(), Some(&m), "{m:?}");
        }
    }

    #[test]
    fn malformed_messages_are_rejected() {
        let good = meta().format_message();
        assert!(SharedStashMeta::parse_message(&good).is_some());
        for bad in [
            "".to_owned(),
            "just a title".to_owned(),
            "title\n\nAtlasGit-Shared-Stash: 2\n".to_owned(),
            good.replace("AtlasGit-Shared-Stash: 1\n", ""),
            good.replace("AtlasGit-Author-Email: a@example.com\n", ""),
            good.replace("AtlasGit-Author-Email: a@example.com", "AtlasGit-Author-Email: "),
            good.replace(&"a".repeat(40), "nothex"),
            good.replace("AtlasGit-Created: 1700000000", "AtlasGit-Created: soon"),
            good.replace("AtlasGit-Updated: 1700000500\n", ""),
            good.replacen("Fix the thing", "", 1),
            // trailers must be the last paragraph
            format!("{good}\nSigned-off-by: someone\n"),
        ] {
            assert_eq!(SharedStashMeta::parse_message(&bad), None, "{bad:?}");
        }
        // Base-Branch is optional on input.
        assert!(SharedStashMeta::parse_message(&good.replace("AtlasGit-Base-Branch: feature/x\n", "")).is_some());
    }

    #[test]
    fn ref_names_round_trip() {
        assert_eq!(remote_ref("ana", "1f-2"), "refs/atlasgit/shared-stash/ana/1f-2");
        assert_eq!(parse_remote_ref("refs/atlasgit/shared-stash/ana/1f-2"), Some(("ana", "1f-2")));
        assert_eq!(mirror_ref("origin", "ana", "1f-2"), "refs/atlasgit/remotes/origin/shared-stash/ana/1f-2");
        assert_eq!(parse_mirror_ref("refs/atlasgit/remotes/origin/shared-stash/ana/1f-2"), Some(("origin", "ana", "1f-2")));
        assert_eq!(keep_ref("1f-2"), "refs/atlasgit/keep/1f-2");
        assert_eq!(probe_ref("1f-2"), "refs/atlasgit/probe/1f-2");
        assert_eq!(mirror_refspec("up"), "+refs/atlasgit/shared-stash/*:refs/atlasgit/remotes/up/shared-stash/*");
    }

    #[test]
    fn remote_names_with_slashes_and_lookalikes_parse_from_the_right() {
        let r = mirror_ref("team/shared-stash/fork", "ana", "9");
        assert_eq!(parse_mirror_ref(&r), Some(("team/shared-stash/fork", "ana", "9")));
    }

    #[test]
    fn foreign_or_incomplete_refs_do_not_parse() {
        for r in [
            "refs/atlasgit/shared-stash/ana",
            "refs/atlasgit/shared-stash/ana/",
            "refs/atlasgit/shared-stash//id",
            "refs/atlasgit/shared-stash/ana/id/extra",
            "refs/heads/ana/id",
        ] {
            assert_eq!(parse_remote_ref(r), None, "{r}");
        }
        for r in [
            "refs/atlasgit/remotes/origin/shared-stash/ana",
            "refs/atlasgit/remotes/origin/other/ana/id",
            "refs/atlasgit/remotes//shared-stash/ana/id",
            "refs/remotes/origin/shared-stash/ana/id",
        ] {
            assert_eq!(parse_mirror_ref(r), None, "{r}");
        }
    }
}
