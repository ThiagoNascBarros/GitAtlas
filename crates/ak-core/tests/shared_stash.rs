// SPDX-License-Identifier: Apache-2.0
//! Shared Stash end to end (docs/specs/shared-stash.md §9, PRD criterion 14) at engine level:
//! two bare remotes R1 and R2, and two clones A and B owned by different people.
//! Only Git features available in 2.20 are used (no `switch`, no `init -b`).

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use ak_core::ops::{Engine, OpError, OpOutcome, Operation, ProbeResult, SharedStashInfo};
use ak_gitcli::{Git, Mode};

fn sh(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?} in {dir:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn sh_ok(dir: &Path, args: &[&str]) -> bool {
    Command::new("git").arg("-C").arg(dir).args(args).output().unwrap().status.success()
}

fn write(dir: &Path, name: &str, text: &str) {
    if let Some(p) = Path::new(name).parent() {
        std::fs::create_dir_all(dir.join(p)).unwrap();
    }
    std::fs::write(dir.join(name), text).unwrap();
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

fn engine(dir: &Path) -> Engine {
    Engine::open(Git::discover(Mode::ReadWrite).unwrap(), dir).unwrap()
}

fn code<T>(r: Result<T, OpError>) -> &'static str {
    match r {
        Ok(_) => panic!("expected an error"),
        Err(e) => e.code,
    }
}

fn identity(dir: &Path, name: &str, email: &str) {
    sh(dir, &["config", "user.name", name]);
    sh(dir, &["config", "user.email", email]);
    // The machine's global config may sign every commit; tests opt in explicitly.
    sh(dir, &["config", "commit.gpgsign", "false"]);
}

struct World {
    r1: PathBuf,
    r2: PathBuf,
    a: PathBuf,
    b: PathBuf,
    ea: Engine,
    eb: Engine,
    base: String,
}

fn world() -> World {
    static N: AtomicU32 = AtomicU32::new(0);
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ss-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let bare = |name: &str| {
        let p = root.join(name);
        std::fs::create_dir_all(&p).unwrap();
        sh(&p, &["init", "-q", "--bare"]);
        sh(&p, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        p
    };
    let (r1, r2) = (bare("r1.git"), bare("r2.git"));
    let a = root.join("a");
    std::fs::create_dir_all(&a).unwrap();
    sh(&a, &["init", "-q"]);
    sh(&a, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    identity(&a, "Ana Silva", "ana@example.com");
    write(&a, "base.txt", "base\n");
    write(&a, "other.txt", "other\n");
    sh(&a, &["add", "."]);
    sh(&a, &["commit", "-qm", "base"]);
    sh(&a, &["remote", "add", "origin", r1.to_str().unwrap()]);
    sh(&a, &["remote", "add", "second", r2.to_str().unwrap()]);
    sh(&a, &["push", "-q", "origin", "main"]);
    sh(&a, &["push", "-q", "second", "main"]);
    let b = root.join("b");
    sh(&root, &["clone", "-q", r1.to_str().unwrap(), b.to_str().unwrap()]);
    identity(&b, "Bruno Costa", "bruno@example.com");
    sh(&b, &["remote", "add", "second", r2.to_str().unwrap()]);
    sh(&b, &["fetch", "-q", "second"]);
    let base = sh(&a, &["rev-parse", "HEAD"]);
    let (ea, eb) = (engine(&a), engine(&b));
    World { r1, r2, a, b, ea, eb, base }
}

/// Leaves tracked, staged and untracked changes in `dir` and stashes them (`-u`); returns the stash sha.
fn make_stash(dir: &Path, e: &Engine, tag: &str) -> String {
    write(dir, "base.txt", &format!("base\nchanged {tag}\n"));
    write(dir, &format!("staged-{tag}.txt"), "staged\n");
    sh(dir, &["add", &format!("staged-{tag}.txt")]);
    write(dir, &format!("new-{tag}.txt"), "untracked\n");
    e.run(&Operation::StashPush { message: Some(format!("wip {tag}")), include_untracked: true }).unwrap();
    e.stashes().unwrap()[0].sha.clone()
}

fn publish(e: &Engine, stash: &str, title: &str, remote: &str) -> Result<OpOutcome, OpError> {
    e.run(&Operation::SharedStashPublish { stash_sha: stash.into(), title: title.into(), remote: Some(remote.into()) })
}

fn shared_id(o: &OpOutcome) -> String {
    o.shared_stash.as_ref().and_then(|s| s.id.clone()).expect("shared stash id")
}

fn fetch_all(e: &Engine) -> OpOutcome {
    e.run(&Operation::SharedStashFetch { remote: None }).unwrap()
}

fn list(e: &Engine) -> Vec<SharedStashInfo> {
    e.shared_stashes().unwrap()
}

fn find(e: &Engine, id: &str) -> SharedStashInfo {
    list(e).into_iter().find(|s| s.id == id).unwrap_or_else(|| panic!("{id} not listed"))
}

fn remote_refs(bare: &Path) -> Vec<String> {
    sh(bare, &["for-each-ref", "--format=%(refname)", "refs/atlasgit/"]).lines().map(str::to_owned).collect()
}

fn empty_tree(dir: &Path) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["hash-object", "-t", "tree", "-w", "--stdin"])
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// A bare commit-tree wrapper made by hand, as a malicious or buggy client would.
fn wrap(dir: &Path, parent: &str, message: &str) -> String {
    let tree = empty_tree(dir);
    sh(dir, &["commit-tree", &tree, "-p", parent, "-m", message])
}

fn meta_message(title: &str, email: &str, base: &str) -> String {
    format!("{title}\n\nAtlasGit-Shared-Stash: 1\nAtlasGit-Author-Email: {email}\nAtlasGit-Base: {base}\nAtlasGit-Base-Branch: main\nAtlasGit-Created: 1700000000\nAtlasGit-Updated: 1700000000\n")
}

fn tree_state(dir: &Path) -> String {
    sh(dir, &["status", "--porcelain=v1", "--untracked-files=all"])
}

// ------------------------------------------------------------------ M1: validation and listing

#[test]
fn validation_accepts_a_real_wrapper_and_rejects_lookalikes() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "v");
    let ok = wrap(&w.a, &stash, &meta_message("fine", "ana@example.com", &w.base));
    let v = w.ea.validate_shared_stash(&ok).unwrap();
    assert_eq!((v.stash.as_str(), v.meta.title.as_str(), v.has_untracked), (stash.as_str(), "fine", true));

    // Not a stash: a plain commit as parent.
    let plain = wrap(&w.a, &w.base, &meta_message("plain", "ana@example.com", &w.base));
    assert!(w.ea.validate_shared_stash(&plain).unwrap_err().reason.contains("stash"));
    // Base trailer disagrees with the stash's first parent.
    let other = sh(&w.a, &["rev-parse", &format!("{stash}^2")]);
    let wrong = wrap(&w.a, &stash, &meta_message("wrong", "ana@example.com", &other));
    assert!(w.ea.validate_shared_stash(&wrong).is_err());
    // No trailers at all.
    let bare = wrap(&w.a, &stash, "just a message");
    assert!(w.ea.validate_shared_stash(&bare).is_err());
    // Not a commit.
    let blob = sh(&w.a, &["rev-parse", "HEAD:base.txt"]);
    assert!(w.ea.validate_shared_stash(&blob).is_err());
    assert!(w.ea.validate_shared_stash(&"0".repeat(40)).is_err());
}

#[test]
fn stash_without_untracked_part_is_valid_and_flagged() {
    let w = world();
    write(&w.a, "base.txt", "tracked only\n");
    w.ea.run(&Operation::StashPush { message: None, include_untracked: false }).unwrap();
    let stash = w.ea.stashes().unwrap()[0].sha.clone();
    let id = shared_id(&publish(&w.ea, &stash, "t", "origin").unwrap());
    assert!(!find(&w.ea, &id).has_untracked);
}

// ------------------------------------------------------------------ criterion 1

#[test]
fn a_publishes_a_stash_with_untracked_files() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "1");
    let out = publish(&w.ea, &stash, "Try my WIP", "origin").unwrap();
    let id = shared_id(&out);
    assert_eq!(out.id, None, "network operations are not undoable");
    assert_eq!(remote_refs(&w.r1), vec![format!("refs/atlasgit/shared-stash/ana-silva/{id}")]);
    assert!(remote_refs(&w.r2).is_empty());
    // The wrapper: empty tree, parent = the stash, trailers.
    let wrapper = sh(&w.r1, &["rev-parse", &format!("refs/atlasgit/shared-stash/ana-silva/{id}")]);
    assert_eq!(sh(&w.r1, &["rev-parse", &format!("{wrapper}^{{tree}}")]), empty_tree(&w.r1));
    assert_eq!(sh(&w.r1, &["rev-parse", &format!("{wrapper}^")]), stash);
    let msg = sh(&w.r1, &["log", "-1", "--format=%B", &wrapper]);
    assert!(msg.starts_with("Try my WIP\n\nAtlasGit-Shared-Stash: 1\nAtlasGit-Author-Email: ana@example.com\n"), "{msg}");
    assert!(msg.contains(&format!("AtlasGit-Base: {}", w.base)) && msg.contains("AtlasGit-Base-Branch: main"), "{msg}");
    // The local stash is untouched, and A sees its own mirror straight away.
    assert_eq!(w.ea.stashes().unwrap().len(), 1);
    let mine = find(&w.ea, &id);
    assert!(mine.valid && mine.mine && !mine.verified);
    assert_eq!(
        (mine.title.as_str(), mine.slug.as_str(), mine.remote.as_str(), mine.author_name.as_str()),
        ("Try my WIP", "ana-silva", "origin", "Ana Silva")
    );
    assert!(w.ea.journal().iter().any(|j| j.kind == "shared_stash_publish" && j.summary.contains("Try my WIP") && !j.undone));
}

#[test]
fn publish_refuses_bad_input_with_stable_codes() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "e");
    assert_eq!(code(publish(&w.ea, &"0".repeat(40), "t", "origin")), "STASH_NOT_FOUND");
    assert_eq!(code(publish(&w.ea, &w.base, "t", "origin")), "STASH_NOT_FOUND", "a commit that is not a stash");
    assert_eq!(code(publish(&w.ea, &stash, "t", "nope")), "NO_REMOTE");
    // No remote given, no upstream and no origin: the caller must choose.
    sh(&w.a, &["remote", "rename", "origin", "elsewhere"]);
    assert_eq!(code(w.ea.run(&Operation::SharedStashPublish { stash_sha: stash.clone(), title: "t".into(), remote: None })), "NO_REMOTE");
    sh(&w.a, &["remote", "rename", "elsewhere", "origin"]);
    sh(&w.a, &["config", "user.email", ""]);
    assert_eq!(code(publish(&w.ea, &stash, "t", "origin")), "NO_IDENTITY");
    assert!(remote_refs(&w.r1).is_empty());
}

#[test]
fn default_remote_is_the_upstream_remote_else_origin() {
    let w = world();
    assert_eq!(w.ea.shared_stash_default_remote().as_deref(), Some("origin"));
    sh(&w.a, &["branch", "--set-upstream-to=second/main"]);
    assert_eq!(w.ea.shared_stash_default_remote().as_deref(), Some("second"));
    let stash = make_stash(&w.a, &w.ea, "d");
    let out = w.ea.run(&Operation::SharedStashPublish { stash_sha: stash, title: "t".into(), remote: None }).unwrap();
    assert_eq!(out.shared_stash.unwrap().remote.as_deref(), Some("second"));
    assert!(remote_refs(&w.r1).is_empty() && remote_refs(&w.r2).len() == 1);
}

#[test]
fn empty_title_falls_back_to_the_stash_message() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "t");
    let id = shared_id(&publish(&w.ea, &stash, "  ", "origin").unwrap());
    assert!(find(&w.ea, &id).title.contains("wip t"));
}

// ------------------------------------------------------------------ criterion 2 (engine side)

#[test]
fn plain_fetch_ignores_the_namespace_and_shared_fetch_downloads_it() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "2");
    let id = shared_id(&publish(&w.ea, &stash, "From Ana", "origin").unwrap());
    w.eb.run(&Operation::Fetch).unwrap();
    assert_eq!(sh(&w.b, &["for-each-ref", "refs/atlasgit/"]), "", "a plain fetch must not download shared stashes");
    assert!(list(&w.eb).is_empty());

    let out = fetch_all(&w.eb);
    let results = out.shared_stash.unwrap().fetched;
    assert_eq!(results.iter().map(|r| (r.remote.as_str(), r.error.is_none())).collect::<Vec<_>>(), vec![("origin", true), ("second", true)]);
    let s = find(&w.eb, &id);
    assert!(!s.mine, "B sees it under Team");
    assert_eq!(
        (s.title.as_str(), s.author_name.as_str(), s.base_branch.as_str(), s.author_email.as_str()),
        ("From Ana", "Ana Silva", "main", "ana@example.com")
    );
    assert!(s.valid && s.invalid_reason.is_none() && s.base == w.base && !s.verified);
}

// ------------------------------------------------------------------ criterion 3

#[test]
fn two_remotes_are_grouped_and_an_unreachable_one_only_fails_itself() {
    let w = world();
    let s1 = make_stash(&w.a, &w.ea, "r1");
    let id1 = shared_id(&publish(&w.ea, &s1, "to r1", "origin").unwrap());
    let s2 = make_stash(&w.a, &w.ea, "r2");
    let id2 = shared_id(&publish(&w.ea, &s2, "to r2", "second").unwrap());
    fetch_all(&w.eb);
    let seen: Vec<(String, String)> = list(&w.eb).into_iter().map(|s| (s.remote, s.id)).collect();
    assert_eq!(seen, vec![("origin".to_owned(), id1.clone()), ("second".to_owned(), id2.clone())]);

    std::fs::rename(&w.r2, w.r2.with_extension("gone")).unwrap();
    let results = fetch_all(&w.eb).shared_stash.unwrap().fetched;
    let by: std::collections::HashMap<_, _> = results.iter().map(|r| (r.remote.as_str(), r.error.as_ref().map(|e| e.code))).collect();
    assert_eq!(by["origin"], None);
    assert!(matches!(by["second"], Some("NO_REMOTE" | "NETWORK_FAILED")), "{by:?}");
    // R1 is still listed, and R2's last known list is not thrown away by the failure.
    assert!(list(&w.eb).iter().any(|s| s.id == id1));
    assert!(list(&w.eb).iter().any(|s| s.id == id2));
    // Asking for the broken remote alone is a hard error with the same code.
    assert!(matches!(code(w.eb.run(&Operation::SharedStashFetch { remote: Some("second".into()) })), "NO_REMOTE" | "NETWORK_FAILED"));
    assert_eq!(code(w.eb.run(&Operation::SharedStashFetch { remote: Some("nope".into()) })), "NO_REMOTE");
}

// ------------------------------------------------------------------ criterion 4

#[test]
fn b_applies_a_stash_and_gets_tracked_staged_and_untracked_changes() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "4");
    let id = shared_id(&publish(&w.ea, &stash, "apply me", "origin").unwrap());
    fetch_all(&w.eb);
    let out = w.eb.run(&Operation::SharedStashApply { id: id.clone(), remote: "origin".into() }).unwrap();
    let ss = out.shared_stash.unwrap();
    assert!(!out.conflicted && !ss.index_dropped);
    assert_eq!(read(&w.b, "base.txt"), "base\nchanged 4\n");
    assert_eq!(read(&w.b, "new-4.txt"), "untracked\n");
    assert_eq!(read(&w.b, "staged-4.txt"), "staged\n");
    assert_eq!(sh(&w.b, &["diff", "--cached", "--name-only"]), "staged-4.txt", "the staged state came along");
    // Same result A had before stashing.
    sh(&w.a, &["stash", "apply", "--index", "-q"]);
    assert_eq!(tree_state(&w.a), tree_state(&w.b));
    assert_eq!(w.eb.stashes().unwrap().len(), 0, "B's own stash list is untouched");
    assert!(w.eb.journal().iter().any(|j| j.kind == "shared_stash_apply"));
}

#[test]
fn dirty_overlapping_file_gives_dirty_tree_and_changes_nothing() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "4d");
    let id = shared_id(&publish(&w.ea, &stash, "apply me", "origin").unwrap());
    fetch_all(&w.eb);
    write(&w.b, "base.txt", "my own edit\n");
    let before = tree_state(&w.b);
    let err = w.eb.run(&Operation::SharedStashApply { id: id.clone(), remote: "origin".into() }).unwrap_err();
    assert_eq!(err.code, "DIRTY_TREE");
    assert!(err.suggestion.unwrap().to_lowercase().contains("commit or stash"));
    assert_eq!(tree_state(&w.b), before, "not even the untracked files were restored");
    assert_eq!(read(&w.b, "base.txt"), "my own edit\n");
    assert!(!w.b.join("new-4d.txt").exists());
    // An untracked file in the way counts too.
    sh(&w.b, &["checkout", "-q", "--", "base.txt"]);
    write(&w.b, "new-4d.txt", "mine\n");
    assert_eq!(code(w.eb.run(&Operation::SharedStashApply { id, remote: "origin".into() })), "DIRTY_TREE");
    assert_eq!(read(&w.b, "new-4d.txt"), "mine\n");
}

#[test]
fn unrelated_local_changes_do_not_block_apply() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "4u");
    let id = shared_id(&publish(&w.ea, &stash, "t", "origin").unwrap());
    fetch_all(&w.eb);
    write(&w.b, "other.txt", "b edits something else\n");
    w.eb.run(&Operation::SharedStashApply { id, remote: "origin".into() }).unwrap();
    assert_eq!(read(&w.b, "other.txt"), "b edits something else\n");
    assert_eq!(read(&w.b, "new-4u.txt"), "untracked\n");
}

#[test]
fn apply_without_index_when_the_index_is_busy_and_says_so() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "ix");
    let id = shared_id(&publish(&w.ea, &stash, "t", "origin").unwrap());
    fetch_all(&w.eb);
    // B has an unrelated staged file. `git stash apply --index` would fail here and unstage it,
    // so the engine applies without --index and reports that the staged state was not restored.
    write(&w.b, "z.txt", "z\n");
    sh(&w.b, &["add", "z.txt"]);
    let out = w.eb.run(&Operation::SharedStashApply { id, remote: "origin".into() }).unwrap();
    assert!(out.shared_stash.unwrap().index_dropped);
    assert_eq!(read(&w.b, "base.txt"), "base\nchanged ix\n");
    assert_eq!(read(&w.b, "new-ix.txt"), "untracked\n");
    assert!(sh(&w.b, &["diff", "--cached", "--name-only"]).lines().any(|l| l == "z.txt"), "B's own staged file is still staged");
}

#[test]
fn apply_conflict_leaves_the_conflicted_state_for_the_existing_banner() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "cf");
    let id = shared_id(&publish(&w.ea, &stash, "t", "origin").unwrap());
    fetch_all(&w.eb);
    // B committed a different change to the same line, so the stash cannot apply cleanly.
    write(&w.b, "base.txt", "base\nB's own line\n");
    sh(&w.b, &["commit", "-qam", "b edit"]);
    let out = w.eb.run(&Operation::SharedStashApply { id, remote: "origin".into() }).unwrap();
    assert!(out.conflicted);
    assert!(sh(&w.b, &["status", "--porcelain"]).contains("UU base.txt"));
}

// ------------------------------------------------------------------ criterion 5

#[test]
fn b_creates_a_branch_at_the_base_with_the_changes_and_can_undo_and_redo() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "5");
    let id = shared_id(&publish(&w.ea, &stash, "branch me", "origin").unwrap());
    fetch_all(&w.eb);
    // B moved on, so the base differs from B's HEAD.
    write(&w.b, "b-only.txt", "b\n");
    sh(&w.b, &["add", "."]);
    sh(&w.b, &["commit", "-qm", "b moves on"]);
    let b_head = sh(&w.b, &["rev-parse", "HEAD"]);

    let out = w.eb.run(&Operation::SharedStashBranch { id: id.clone(), remote: "origin".into(), name: "try-ana".into() }).unwrap();
    assert_eq!(sh(&w.b, &["symbolic-ref", "--short", "HEAD"]), "try-ana");
    assert_eq!(sh(&w.b, &["rev-parse", "try-ana"]), w.base);
    assert_eq!(read(&w.b, "base.txt"), "base\nchanged 5\n");
    assert_eq!(read(&w.b, "new-5.txt"), "untracked\n");
    assert!(!w.b.join("b-only.txt").exists(), "branch is at the base, not at B's HEAD");
    assert_eq!(sh(&w.b, &["rev-parse", "main"]), b_head, "nothing else touched");
    assert!(w.eb.stashes().unwrap().is_empty());
    let jid = out.id.expect("undoable like BranchCreate");

    w.eb.undo(&jid).unwrap();
    assert_eq!(sh(&w.b, &["symbolic-ref", "--short", "HEAD"]), "main");
    assert!(!sh_ok(&w.b, &["rev-parse", "--verify", "-q", "refs/heads/try-ana"]));
    assert_eq!(read(&w.b, "base.txt"), "base\nchanged 5\n", "the applied changes stay as ordinary local changes");
    w.eb.redo_last().unwrap();
    assert_eq!(sh(&w.b, &["symbolic-ref", "--short", "HEAD"]), "try-ana");
    assert_eq!(sh(&w.b, &["rev-parse", "try-ana"]), w.base);
}

#[test]
fn branch_rolls_back_when_the_changes_cannot_be_applied() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "5r");
    let id = shared_id(&publish(&w.ea, &stash, "t", "origin").unwrap());
    fetch_all(&w.eb);
    write(&w.b, "base.txt", "my own edit\n");
    let err = w.eb.run(&Operation::SharedStashBranch { id: id.clone(), remote: "origin".into(), name: "x".into() }).unwrap_err();
    assert_eq!(err.code, "DIRTY_TREE");
    assert!(!sh_ok(&w.b, &["rev-parse", "--verify", "-q", "refs/heads/x"]), "branch removed again");
    assert_eq!(sh(&w.b, &["symbolic-ref", "--short", "HEAD"]), "main");
    assert_eq!(read(&w.b, "base.txt"), "my own edit\n");
    assert!(w.eb.journal().iter().all(|j| j.kind != "shared_stash_branch"));
    // Name problems are reported before anything happens.
    assert_eq!(
        code(w.eb.run(&Operation::SharedStashBranch { id: id.clone(), remote: "origin".into(), name: "bad name".into() })),
        "INVALID_BRANCH_NAME"
    );
    assert_eq!(code(w.eb.run(&Operation::SharedStashBranch { id, remote: "origin".into(), name: "main".into() })), "BRANCH_EXISTS");
}

#[test]
fn branch_reports_a_base_that_cannot_be_found() {
    let w = world();
    // A stash on a commit that exists nowhere else: the topic branch is never pushed.
    sh(&w.a, &["checkout", "-q", "-b", "topic"]);
    write(&w.a, "topic.txt", "t\n");
    sh(&w.a, &["add", "."]);
    sh(&w.a, &["commit", "-qm", "topic work"]);
    let topic = sh(&w.a, &["rev-parse", "HEAD"]);
    let stash = make_stash(&w.a, &w.ea, "bm");
    let id = shared_id(&publish(&w.ea, &stash, "t", "origin").unwrap());
    // B receives loose objects, then loses the base commit.
    sh(&w.b, &["config", "fetch.unpackLimit", "100000"]);
    sh(&w.b, &["config", "transfer.unpackLimit", "100000"]);
    fetch_all(&w.eb);
    let loose = w.b.join(".git/objects").join(&topic[..2]).join(&topic[2..]);
    assert!(loose.exists(), "the base arrived as a loose object");
    std::fs::remove_file(&loose).unwrap();
    assert!(!sh_ok(&w.b, &["cat-file", "-e", &topic]));

    for op in [
        Operation::SharedStashBranch { id: id.clone(), remote: "origin".into(), name: "nb".into() },
        Operation::SharedStashApply { id: id.clone(), remote: "origin".into() },
    ] {
        let e = w.eb.run(&op).unwrap_err();
        assert_eq!(e.code, "BASE_MISSING", "{op:?}");
    }
    assert!(!sh_ok(&w.b, &["rev-parse", "--verify", "-q", "refs/heads/nb"]));
}

// ------------------------------------------------------------------ criterion 6

#[test]
fn only_the_author_may_change_or_remove_and_forged_authors_are_only_claimed() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "6");
    let id = shared_id(&publish(&w.ea, &stash, "ana's", "origin").unwrap());
    fetch_all(&w.eb);
    let before = remote_refs(&w.r1);
    let sha = sh(&w.r1, &["rev-parse", &before[0]]);
    for op in [
        Operation::SharedStashUpdate { id: id.clone(), remote: "origin".into(), stash_sha: None, title: Some("mine now".into()) },
        Operation::SharedStashUnshare { id: id.clone(), remote: "origin".into() },
    ] {
        assert_eq!(code(w.eb.run(&op)), "NOT_AUTHOR", "{op:?}");
    }
    assert_eq!(sh(&w.r1, &["rev-parse", &before[0]]), sha, "nothing was pushed");

    // B forges Ana's email in a wrapper and pushes it with plain Git.
    let bstash = make_stash(&w.b, &w.eb, "6b");
    let forged = wrap(&w.b, &bstash, &meta_message("totally from Ana", "ana@example.com", &w.base));
    sh(&w.b, &["push", "-q", "origin", &format!("{forged}:refs/atlasgit/shared-stash/ana-silva/forged-1")]);
    fetch_all(&w.ea);
    let f = find(&w.ea, "forged-1");
    assert!(f.valid && f.mine, "it claims to be Ana's");
    assert!(!f.verified, "but nothing proves it");
    assert!(!find(&w.ea, &id).verified, "unsigned wrappers are never verified either");
}

#[test]
fn signed_wrapper_is_verified_only_for_the_matching_signer() {
    if !Command::new("gpg").arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
        eprintln!("gpg not installed: skipping");
        return;
    }
    let home = std::env::temp_dir().join(format!("akgpg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::fs::set_permissions(&home, std::fs::Permissions::from_mode(0o700)).unwrap();
    let wrapper_script = home.join("gpg-wrapper.sh");
    std::fs::write(&wrapper_script, format!("#!/bin/sh\nexport GNUPGHOME={}\nexec gpg \"$@\"\n", home.display())).unwrap();
    std::fs::set_permissions(&wrapper_script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let key = |uid: &str| {
        Command::new("gpg")
            .env("GNUPGHOME", &home)
            .args(["--batch", "--passphrase", "", "--quick-gen-key", uid, "default", "default", "never"])
            .output()
            .unwrap()
            .status
            .success()
    };
    if !(key("Ana Silva <ana@example.com>") && key("Mallory <mallory@example.com>")) {
        eprintln!("cannot create a throwaway gpg key here: skipping");
        let _ = std::fs::remove_dir_all(&home);
        return;
    }
    let w = world();
    for d in [&w.a, &w.b] {
        sh(d, &["config", "gpg.format", "openpgp"]);
        sh(d, &["config", "gpg.program", wrapper_script.to_str().unwrap()]);
    }
    sh(&w.a, &["config", "user.signingkey", "ana@example.com"]);
    sh(&w.a, &["config", "commit.gpgsign", "true"]);
    let stash = make_stash(&w.a, &w.ea, "sg");
    let id = shared_id(&publish(&w.ea, &stash, "signed", "origin").unwrap());
    assert!(
        sh(&w.r1, &["cat-file", "-p", &format!("refs/atlasgit/shared-stash/ana-silva/{id}")]).contains("gpgsig"),
        "commit.gpgsign signs the wrapper"
    );
    fetch_all(&w.eb);
    assert!(find(&w.eb, &id).verified);

    // Mallory signs a wrapper that claims Ana's email: the signature is valid but not Ana's.
    let bstash = make_stash(&w.b, &w.eb, "sm");
    let tree = empty_tree(&w.b);
    sh(&w.b, &["config", "user.signingkey", "mallory@example.com"]);
    let forged = sh(&w.b, &["commit-tree", &tree, "-p", &bstash, "-S", "-m", &meta_message("forged", "ana@example.com", &w.base)]);
    sh(&w.b, &["push", "-q", "origin", &format!("{forged}:refs/atlasgit/shared-stash/ana-silva/forged-2")]);
    fetch_all(&w.ea);
    let f = find(&w.ea, "forged-2");
    assert!(f.valid && !f.verified, "valid signature, wrong person");
    let _ = std::fs::remove_dir_all(&home);
}

// ------------------------------------------------------------------ criterion 7

#[test]
fn author_updates_content_and_renames_and_b_sees_it_after_fetch() {
    let w = world();
    let s1 = make_stash(&w.a, &w.ea, "7a");
    let id = shared_id(&publish(&w.ea, &s1, "first title", "origin").unwrap());
    fetch_all(&w.eb);
    let old = find(&w.eb, &id);

    // Rename only: same stash, new title, Created kept, Updated moves.
    w.ea.run(&Operation::SharedStashUpdate { id: id.clone(), remote: "origin".into(), stash_sha: None, title: Some("better title".into()) }).unwrap();
    assert_eq!(find(&w.ea, &id).title, "better title");
    assert_eq!(find(&w.eb, &id).title, "first title", "B sees nothing until it fetches");
    fetch_all(&w.eb);
    let renamed = find(&w.eb, &id);
    assert_eq!((renamed.title.as_str(), renamed.stash.as_str(), renamed.created), ("better title", old.stash.as_str(), old.created));
    assert!(renamed.updated >= old.updated && renamed.wrapper != old.wrapper);

    // New content: another local stash.
    let s2 = make_stash(&w.a, &w.ea, "7b");
    w.ea.run(&Operation::SharedStashUpdate { id: id.clone(), remote: "origin".into(), stash_sha: Some(s2.clone()), title: None }).unwrap();
    fetch_all(&w.eb);
    let updated = find(&w.eb, &id);
    assert_eq!((updated.stash.as_str(), updated.title.as_str()), (s2.as_str(), "better title"));
    w.eb.run(&Operation::SharedStashApply { id: id.clone(), remote: "origin".into() }).unwrap();
    assert_eq!(read(&w.b, "new-7b.txt"), "untracked\n");
    assert!(!w.b.join("new-7a.txt").exists());
    assert_eq!(remote_refs(&w.r1).len(), 1, "same ref, rewritten");
    assert!(w.ea.journal().iter().filter(|j| j.kind == "shared_stash_update").count() == 2);
    assert_eq!(code(w.ea.run(&Operation::SharedStashUpdate { id, remote: "origin".into(), stash_sha: None, title: None })), "NOTHING_TO_UPDATE");
}

#[test]
fn update_is_refused_when_the_remote_ref_moved_in_the_meantime() {
    let w = world();
    let s1 = make_stash(&w.a, &w.ea, "7s");
    let id = shared_id(&publish(&w.ea, &s1, "title", "origin").unwrap());
    let rref = format!("refs/atlasgit/shared-stash/ana-silva/{id}");
    let original = sh(&w.r1, &["rev-parse", &rref]);

    // Another clone moves the ref after A's fresh fetch but before A's push: a pre-push hook plays that clone.
    let other = wrap(&w.b, &make_stash(&w.b, &w.eb, "7x"), &meta_message("someone else", "ana@example.com", &w.base));
    let flag = w.a.join("hook-ran");
    let hook = w.a.join(".git/hooks/pre-push");
    std::fs::write(
        &hook,
        format!(
            "#!/bin/sh\nunset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE\n[ -e '{flag}' ] && exit 0\ntouch '{flag}'\ngit -C '{b}' push -q --force '{r1}' '{other}:{rref}'\n",
            flag = flag.display(),
            b = w.b.display(),
            r1 = w.r1.display(),
        ),
    )
    .unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

    let err =
        w.ea.run(&Operation::SharedStashUpdate { id: id.clone(), remote: "origin".into(), stash_sha: None, title: Some("mine".into()) }).unwrap_err();
    assert_eq!(err.code, "STALE_SHARED_STASH", "{err:?}");
    assert!(flag.exists(), "the hook ran, so the ref really moved");
    assert_eq!(sh(&w.r1, &["rev-parse", &rref]), other, "A did not overwrite the newer value");
    assert_ne!(other, original);
    assert!(w.ea.journal().iter().all(|j| j.kind != "shared_stash_update"));
}

// ------------------------------------------------------------------ criterion 8

#[test]
fn unshare_prunes_for_others_keeps_a_copy_and_reshare_restores_it() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "8");
    let id = shared_id(&publish(&w.ea, &stash, "to be unshared", "origin").unwrap());
    fetch_all(&w.eb);
    assert_eq!(list(&w.eb).len(), 1);
    let wrapper = find(&w.ea, &id).wrapper;

    w.ea.run(&Operation::SharedStashUnshare { id: id.clone(), remote: "origin".into() }).unwrap();
    assert!(remote_refs(&w.r1).is_empty());
    assert!(list(&w.ea).is_empty(), "gone from A's list at once");
    assert_eq!(sh(&w.a, &["rev-parse", &format!("refs/atlasgit/keep/{id}")]), wrapper, "protected from gc");
    let kept = w.ea.shared_stash_kept().unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!((kept[0].id.as_str(), kept[0].title.as_str(), kept[0].remote.as_deref()), (id.as_str(), "to be unshared", Some("origin")));
    assert_eq!(list(&w.eb).len(), 1, "B still has its old copy until it fetches");
    fetch_all(&w.eb);
    assert!(list(&w.eb).is_empty(), "pruned");
    assert_eq!(sh(&w.b, &["for-each-ref", "refs/atlasgit/remotes/"]), "");

    w.ea.run(&Operation::SharedStashReshare { id: id.clone(), remote: "origin".into() }).unwrap();
    assert_eq!(remote_refs(&w.r1), vec![format!("refs/atlasgit/shared-stash/ana-silva/{id}")]);
    assert_eq!(sh(&w.r1, &["rev-parse", &format!("refs/atlasgit/shared-stash/ana-silva/{id}")]), wrapper);
    assert!(w.ea.shared_stash_kept().unwrap().is_empty());
    assert!(!sh_ok(&w.a, &["rev-parse", "--verify", "-q", &format!("refs/atlasgit/keep/{id}")]));
    assert_eq!(find(&w.ea, &id).wrapper, wrapper);
    fetch_all(&w.eb);
    assert_eq!(find(&w.eb, &id).title, "to be unshared");
    let kinds: Vec<String> = w.ea.journal().into_iter().map(|j| j.kind).collect();
    assert!(kinds.contains(&"shared_stash_unshare".to_owned()) && kinds.contains(&"shared_stash_reshare".to_owned()));
}

#[test]
fn reshare_refuses_to_overwrite_and_discard_keep_forgets_the_copy() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "8b");
    let id = shared_id(&publish(&w.ea, &stash, "t", "origin").unwrap());
    w.ea.run(&Operation::SharedStashUnshare { id: id.clone(), remote: "origin".into() }).unwrap();
    // Someone put another value on the same ref meanwhile: reshare is create-only.
    let rref = format!("refs/atlasgit/shared-stash/ana-silva/{id}");
    let squatter = wrap(&w.a, &stash, &meta_message("squatter", "ana@example.com", &w.base));
    sh(&w.a, &["push", "-q", "origin", &format!("{squatter}:{rref}")]);
    assert_eq!(code(w.ea.run(&Operation::SharedStashReshare { id: id.clone(), remote: "origin".into() })), "STALE_SHARED_STASH");
    assert_eq!(sh(&w.r1, &["rev-parse", &rref]), squatter);
    assert_eq!(w.ea.shared_stash_kept().unwrap().len(), 1, "the kept copy survives a failed reshare");

    w.ea.run(&Operation::SharedStashDiscardKeep { id: id.clone() }).unwrap();
    assert!(w.ea.shared_stash_kept().unwrap().is_empty());
    assert!(!sh_ok(&w.a, &["rev-parse", "--verify", "-q", &format!("refs/atlasgit/keep/{id}")]));
    assert_eq!(code(w.ea.run(&Operation::SharedStashDiscardKeep { id: id.clone() })), "STASH_NOT_FOUND");
    assert_eq!(code(w.ea.run(&Operation::SharedStashReshare { id, remote: "origin".into() })), "STASH_NOT_FOUND");
    for odd in ["../../heads/main", "", "a b"] {
        assert_eq!(code(w.ea.run(&Operation::SharedStashReshare { id: odd.into(), remote: "origin".into() })), "STASH_NOT_FOUND", "{odd:?}");
        assert_eq!(code(w.ea.run(&Operation::SharedStashDiscardKeep { id: odd.into() })), "STASH_NOT_FOUND", "{odd:?}");
    }
}

#[test]
fn unshare_of_a_stash_that_is_already_gone_says_so() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "8g");
    let id = shared_id(&publish(&w.ea, &stash, "t", "origin").unwrap());
    sh(&w.r1, &["update-ref", "-d", &format!("refs/atlasgit/shared-stash/ana-silva/{id}")]);
    assert_eq!(code(w.ea.run(&Operation::SharedStashUnshare { id, remote: "origin".into() })), "SHARED_STASH_GONE");
    assert!(w.ea.shared_stash_kept().unwrap().is_empty());
    assert_eq!(code(w.ea.run(&Operation::SharedStashApply { id: "nope".into(), remote: "origin".into() })), "SHARED_STASH_GONE");
}

// ------------------------------------------------------------------ criterion 9

#[test]
fn a_ref_that_is_not_a_stash_is_listed_invalid_and_never_acted_on() {
    let w = world();
    sh(&w.a, &["push", "-q", "origin", "HEAD:refs/atlasgit/shared-stash/mallory/not-a-stash"]);
    // Also: a wrapper whose parent is a plain commit.
    let plain = wrap(&w.a, &w.base, &meta_message("looks fine", "ana@example.com", &w.base));
    sh(&w.a, &["push", "-q", "origin", &format!("{plain}:refs/atlasgit/shared-stash/mallory/plain-parent")]);
    fetch_all(&w.eb);
    let all = list(&w.eb);
    assert_eq!(all.len(), 2);
    for s in &all {
        assert!(!s.valid && s.invalid_reason.is_some() && !s.mine && !s.verified, "{s:?}");
        for op in [
            Operation::SharedStashApply { id: s.id.clone(), remote: "origin".into() },
            Operation::SharedStashBranch { id: s.id.clone(), remote: "origin".into(), name: "nope".into() },
        ] {
            assert_eq!(code(w.eb.run(&op)), "INVALID_SHARED_STASH", "{op:?}");
        }
    }
    assert!(!sh_ok(&w.b, &["rev-parse", "--verify", "-q", "refs/heads/nope"]));
    // Invalid rows cannot be changed by anyone, not even by a matching email.
    assert_eq!(
        code(w.ea.run(&Operation::SharedStashUnshare { id: "plain-parent".into(), remote: "origin".into() })),
        "SHARED_STASH_GONE",
        "A never fetched it"
    );
    fetch_all(&w.ea);
    assert_eq!(code(w.ea.run(&Operation::SharedStashUnshare { id: "plain-parent".into(), remote: "origin".into() })), "INVALID_SHARED_STASH");
}

// ------------------------------------------------------------------ criterion 10

fn reject_custom_refs(bare: &Path) {
    let hook = bare.join("hooks/pre-receive");
    std::fs::write(&hook, "#!/bin/sh\nwhile read old new ref; do\n  case \"$ref\" in refs/atlasgit/*) echo \"custom refs are not allowed here\" >&2; exit 1;; esac\ndone\nexit 0\n").unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn a_host_that_rejects_custom_refs_is_detected_once_and_cached() {
    let w = world();
    reject_custom_refs(&w.r1);
    let stash = make_stash(&w.a, &w.ea, "10");
    assert_eq!(w.ea.shared_stash_probe("origin", &[]).unwrap(), ProbeResult::Rejected);
    let cache = w.a.join(".git/atlasgit/remotes.json");
    assert!(std::fs::read_to_string(&cache).unwrap().contains("rejected"));
    assert_eq!(w.ea.shared_stash_remotes().into_iter().find(|r| r.name == "origin").unwrap().probe, Some(ProbeResult::Rejected));
    let e = publish(&w.ea, &stash, "t", "origin").unwrap_err();
    assert_eq!(e.code, "REMOTE_REJECTED");
    assert!(e.suggestion.is_some());
    assert!(remote_refs(&w.r1).is_empty());
    // Cached: lifting the restriction changes nothing until the user asks for a new check.
    std::fs::remove_file(w.r1.join("hooks/pre-receive")).unwrap();
    assert_eq!(code(publish(&w.ea, &stash, "t", "origin")), "REMOTE_REJECTED");
    w.ea.shared_stash_probe_reset(Some("origin")).unwrap();
    assert_eq!(w.ea.shared_stash_remotes().into_iter().find(|r| r.name == "origin").unwrap().probe, None);
    publish(&w.ea, &stash, "t", "origin").unwrap();
    assert_eq!(remote_refs(&w.r1).len(), 1);
    assert!(std::fs::read_to_string(&cache).unwrap().contains("\"ok\""));
}

#[test]
fn an_accepting_host_passes_the_probe_and_keeps_no_trace() {
    let w = world();
    assert_eq!(w.ea.shared_stash_probe("origin", &[]).unwrap(), ProbeResult::Ok);
    assert!(remote_refs(&w.r1).is_empty(), "the throwaway ref is deleted again");
    // Cached: a later outage does not matter for the decision.
    let stash = make_stash(&w.a, &w.ea, "10ok");
    publish(&w.ea, &stash, "t", "origin").unwrap();
    // If the remote URL changes, the cached answer no longer applies.
    reject_custom_refs(&w.r2);
    sh(&w.a, &["remote", "set-url", "origin", w.r2.to_str().unwrap()]);
    assert_eq!(w.ea.shared_stash_remotes().into_iter().find(|r| r.name == "origin").unwrap().probe, None);
    assert_eq!(code(publish(&w.ea, &stash, "t", "origin")), "REMOTE_REJECTED");
    // A broken remote is an error, not a verdict, and is not cached.
    std::fs::remove_dir_all(&w.r2).unwrap();
    let e = w.ea.shared_stash_probe("origin", &[]).unwrap_err();
    assert!(matches!(e.code, "NO_REMOTE" | "NETWORK_FAILED"), "{e:?}");
}

// ------------------------------------------------------------------ criterion 11

#[test]
fn same_user_name_gives_distinct_refs_and_ownership_follows_the_email() {
    let w = world();
    sh(&w.b, &["config", "user.name", "Ana Silva"]); // same display name, different person
    let sa = make_stash(&w.a, &w.ea, "11a");
    let sb = make_stash(&w.b, &w.eb, "11b");
    let ia = shared_id(&publish(&w.ea, &sa, "Ana's", "origin").unwrap());
    let ib = shared_id(&publish(&w.eb, &sb, "Other Ana's", "origin").unwrap());
    assert_ne!(ia, ib);
    let refs = remote_refs(&w.r1);
    assert_eq!(refs.len(), 2);
    assert!(refs.iter().all(|r| r.starts_with("refs/atlasgit/shared-stash/ana-silva/")), "{refs:?}");
    fetch_all(&w.ea);
    fetch_all(&w.eb);
    for (e, mine, theirs) in [(&w.ea, &ia, &ib), (&w.eb, &ib, &ia)] {
        assert!(find(e, mine).mine && !find(e, theirs).mine);
    }

    // A renames themselves: the old stash is still theirs; the new one gets the new slug.
    sh(&w.a, &["config", "user.name", "Ana Costa Silva"]);
    assert!(find(&w.ea, &ia).mine);
    let sa2 = make_stash(&w.a, &w.ea, "11c");
    let ia2 = shared_id(&publish(&w.ea, &sa2, "after rename", "origin").unwrap());
    assert!(remote_refs(&w.r1).contains(&format!("refs/atlasgit/shared-stash/ana-costa-silva/{ia2}")));
    // They can still manage the old one.
    w.ea.run(&Operation::SharedStashUpdate { id: ia.clone(), remote: "origin".into(), stash_sha: None, title: Some("renamed".into()) }).unwrap();
    assert!(remote_refs(&w.r1).contains(&format!("refs/atlasgit/shared-stash/ana-silva/{ia}")), "slug is fixed at publish time");
    assert_eq!(find(&w.ea, &ia).title, "renamed");
}

#[test]
fn a_missing_user_name_uses_the_email_for_slug_and_wrapper_author() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "nn");
    sh(&w.a, &["config", "user.name", ""]);
    let id = shared_id(&publish(&w.ea, &stash, "t", "origin").unwrap());
    assert_eq!(remote_refs(&w.r1), vec![format!("refs/atlasgit/shared-stash/ana/{id}")]);
    assert_eq!(find(&w.ea, &id).author_name, "ana");
}

// ------------------------------------------------------------------ criterion 12

#[test]
fn big_stashes_warn_but_still_publish() {
    let w = world();
    std::fs::write(w.a.join("big.bin"), (0..11 * 1024 * 1024u32).map(|i| (i.wrapping_mul(2654435761) >> 24) as u8).collect::<Vec<u8>>()).unwrap();
    w.ea.run(&Operation::StashPush { message: Some("big".into()), include_untracked: true }).unwrap();
    let big = w.ea.stashes().unwrap()[0].sha.clone();
    let out = publish(&w.ea, &big, "big one", "origin").unwrap();
    let warn = out.shared_stash.as_ref().unwrap().large_bytes.expect("warning present");
    assert!(warn > 10 * 1024 * 1024, "{warn}");
    assert_eq!(remote_refs(&w.r1).len(), 1, "still published");

    let small = make_stash(&w.a, &w.ea, "small");
    assert_eq!(publish(&w.ea, &small, "small", "origin").unwrap().shared_stash.unwrap().large_bytes, None);
}

// ------------------------------------------------------------------ T15 journal and undo wiring

#[test]
fn network_entries_are_not_undoable_and_do_not_clear_redo() {
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "15");
    write(&w.a, "new.txt", "n\n");
    w.ea.run(&Operation::Stage { paths: vec!["new.txt".into()] }).unwrap();
    let c = w.ea.run(&Operation::Commit { message: "to undo".into(), amend: false }).unwrap();
    w.ea.undo(c.id.as_deref().unwrap()).unwrap();
    assert!(w.ea.redo_candidate().is_some());

    let id = shared_id(&publish(&w.ea, &stash, "t", "origin").unwrap());
    w.ea.run(&Operation::SharedStashUpdate { id: id.clone(), remote: "origin".into(), stash_sha: None, title: Some("x".into()) }).unwrap();
    let entries: Vec<_> = w.ea.journal().into_iter().filter(|j| j.kind.starts_with("shared_stash_")).collect();
    assert_eq!(entries.iter().map(|e| e.kind.as_str()).collect::<Vec<_>>(), vec!["shared_stash_publish", "shared_stash_update"]);
    for e in &entries {
        assert_eq!(code(w.ea.undo(&e.id)), "UNDO_NOT_SUPPORTED");
    }
    let cand = w.ea.redo_candidate().expect("publishing did not clear the redo stack");
    assert_eq!(cand.id, c.id.unwrap());
    w.ea.redo_last().unwrap();
    assert!(w.ea.redo_candidate().is_none());
}

#[test]
fn remote_operations_conflict_with_each_other_but_not_with_local_ones() {
    use ak_core::ops::lock::{self, Lock};
    let w = world();
    let stash = make_stash(&w.a, &w.ea, "lk");
    let _held = lock::try_acquire(w.ea.root(), &[Lock::Remote]).unwrap();
    assert_eq!(code(publish(&w.ea, &stash, "t", "origin")), "REPO_BUSY");
    assert_eq!(code(w.ea.run(&Operation::SharedStashFetch { remote: None })), "REPO_BUSY");
    write(&w.a, "n.txt", "x\n");
    w.ea.run(&Operation::Stage { paths: vec!["n.txt".into()] }).unwrap();
}
