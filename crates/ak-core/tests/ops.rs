// SPDX-License-Identifier: Apache-2.0
//! Operation engine against throwaway repositories.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use ak_core::ops::lock::{self, Lock};
use ak_core::ops::{Engine, Operation};
use ak_gitcli::{Git, Mode};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn repo() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ops-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    git(&dir, &["config", "user.name", "Test"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    dir
}

fn engine(dir: &Path) -> Engine {
    Engine::open(Git::discover(Mode::ReadWrite).unwrap(), dir).unwrap()
}

fn write(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

fn stage(e: &Engine, paths: &[&str]) {
    e.run(&Operation::Stage { paths: paths.iter().map(|p| p.to_string()).collect() }).unwrap();
}

fn commit(e: &Engine, msg: &str, amend: bool) -> ak_core::ops::OpOutcome {
    e.run(&Operation::Commit { message: msg.into(), amend }).unwrap()
}

fn code(r: Result<ak_core::ops::OpOutcome, ak_core::ops::OpError>) -> &'static str {
    r.unwrap_err().code
}

#[test]
fn stage_commit_and_journal() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    assert_eq!(git(&d, &["diff", "--cached", "--name-only"]), "a.txt");
    let out = commit(&e, "First\n\nbody", false);
    assert_eq!(out.head.as_deref(), Some(git(&d, &["rev-parse", "HEAD"]).as_str()));
    assert_eq!(git(&d, &["log", "-1", "--format=%B"]), "First\n\nbody");
    let j = e.journal();
    assert_eq!(j.len(), 1);
    assert_eq!((j[0].kind.as_str(), j[0].summary.as_str(), j[0].head_before.is_none()), ("commit", "First", true));
}

#[test]
fn unstage_works_with_and_without_commits() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    e.run(&Operation::Unstage { paths: vec!["a.txt".into()] }).unwrap();
    assert_eq!(git(&d, &["diff", "--cached", "--name-only"]), "");
    stage(&e, &["a.txt"]);
    commit(&e, "c1", false);
    write(&d, "a.txt", "2");
    stage(&e, &["a.txt"]);
    e.run(&Operation::Unstage { paths: vec!["a.txt".into()] }).unwrap();
    assert_eq!(git(&d, &["diff", "--cached", "--name-only"]), "");
    assert_eq!(git(&d, &["status", "--porcelain"]), "M a.txt");
}

#[test]
fn nothing_to_commit_and_empty_message() {
    let d = repo();
    let e = engine(&d);
    assert_eq!(code(e.run(&Operation::Commit { message: "x".into(), amend: false })), "NOTHING_TO_COMMIT");
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    assert_eq!(code(e.run(&Operation::Commit { message: "  \n".into(), amend: false })), "EMPTY_MESSAGE");
    assert_eq!(code(e.run(&Operation::Commit { message: "x".into(), amend: true })), "NOTHING_TO_COMMIT", "no commit to amend yet");
}

#[test]
fn undo_keeps_changes_staged_and_cannot_repeat() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    commit(&e, "c1", false);
    write(&d, "b.txt", "2");
    stage(&e, &["b.txt"]);
    let c2 = commit(&e, "c2", false);
    let first = git(&d, &["rev-parse", "HEAD~1"]);
    e.undo(c2.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["rev-parse", "HEAD"]), first);
    assert_eq!(git(&d, &["diff", "--cached", "--name-only"]), "b.txt");
    assert_eq!(code(e.undo(c2.id.as_deref().unwrap()).map(|_| Default::default())), "ALREADY_UNDONE");
}

#[test]
fn undo_refuses_when_head_moved() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    let c1 = commit(&e, "c1", false);
    write(&d, "b.txt", "2");
    stage(&e, &["b.txt"]);
    commit(&e, "c2", false);
    assert_eq!(code(e.undo(c1.id.as_deref().unwrap()).map(|_| Default::default())), "HEAD_MOVED");
}

#[test]
fn undo_first_commit_returns_to_unborn() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    let c = commit(&e, "root", false);
    e.undo(c.id.as_deref().unwrap()).unwrap();
    assert!(Command::new("git").arg("-C").arg(&d).args(["rev-parse", "--verify", "-q", "HEAD"]).status().unwrap().code() == Some(1));
    assert_eq!(git(&d, &["diff", "--cached", "--name-only"]), "a.txt");
}

#[test]
fn amend_takes_a_snapshot_and_can_be_undone() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    commit(&e, "c1", false);
    let original = git(&d, &["rev-parse", "HEAD"]);
    let out = commit(&e, "c1 fixed", true);
    assert_ne!(git(&d, &["rev-parse", "HEAD"]), original);
    assert_eq!(git(&d, &["log", "-1", "--format=%s"]), "c1 fixed");
    let entry = e.journal().into_iter().find(|x| x.kind == "commit_amend").unwrap();
    let snap = entry.snapshot_ref.clone().unwrap();
    assert!(snap.starts_with("refs/atlaskraken/snapshots/"));
    assert_eq!(git(&d, &["rev-parse", &snap]), original);
    e.undo(out.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["rev-parse", "HEAD"]), original);
}

#[test]
fn missing_identity_gets_a_stable_code() {
    let d = repo();
    git(&d, &["config", "user.name", ""]);
    git(&d, &["config", "user.email", ""]);
    let e = engine(&d);
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    let r = e.run(&Operation::Commit { message: "x".into(), amend: false });
    assert_eq!(code(r), "GIT_IDENTITY_MISSING");
}

#[test]
fn busy_repository_is_reported() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "1");
    let _held = lock::try_acquire(e.root(), &[Lock::Index]).unwrap();
    assert_eq!(code(e.run(&Operation::Stage { paths: vec!["a.txt".into()] })), "REPO_BUSY");
}

#[test]
fn paths_are_literal_not_globs() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a1b.txt", "1");
    write(&d, "a*b.txt", "2");
    stage(&e, &["a*b.txt"]);
    assert_eq!(git(&d, &["diff", "--cached", "--name-only"]), "a*b.txt");
}

#[test]
fn many_paths_are_chunked() {
    let d = repo();
    let e = engine(&d);
    let names: Vec<String> = (0..250).map(|i| format!("f{i}.txt")).collect();
    for n in &names {
        write(&d, n, "x");
    }
    e.run(&Operation::Stage { paths: names }).unwrap();
    assert_eq!(git(&d, &["diff", "--cached", "--name-only"]).lines().count(), 250);
}

#[test]
fn read_only_git_is_refused() {
    let ro = Git::discover(Mode::ReadOnly).unwrap();
    assert_eq!(Engine::open(ro, &repo()).err().unwrap().code, "READ_ONLY");
}

fn committed(d: &Path, e: &Engine, name: &str, text: &str) {
    write(d, name, text);
    stage(e, &[name]);
    commit(e, &format!("add {name}"), false);
}

#[test]
fn discard_restores_tracked_and_untracked_and_can_be_undone() {
    let d = repo();
    let e = engine(&d);
    committed(&d, &e, "a.txt", "one\n");
    committed(&d, &e, "gone.txt", "bye\n");
    write(&d, "a.txt", "changed\n");
    std::fs::remove_file(d.join("gone.txt")).unwrap();
    write(&d, "new.txt", "fresh\n");
    let out = e
        .run(&Operation::Discard { paths: vec!["a.txt".into(), "gone.txt".into(), "new.txt".into()] })
        .unwrap();
    assert_eq!(git(&d, &["status", "--porcelain"]), "");
    assert!(!d.join("new.txt").exists());
    e.undo(out.id.as_deref().unwrap()).unwrap();
    assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "changed\n");
    assert_eq!(std::fs::read_to_string(d.join("new.txt")).unwrap(), "fresh\n");
    assert!(!d.join("gone.txt").exists());
}

#[test]
fn undo_discard_refuses_to_overwrite_newer_work() {
    let d = repo();
    let e = engine(&d);
    committed(&d, &e, "a.txt", "one\n");
    write(&d, "a.txt", "changed\n");
    let out = e.run(&Operation::Discard { paths: vec!["a.txt".into()] }).unwrap();
    write(&d, "a.txt", "newer\n");
    assert_eq!(code(e.undo(out.id.as_deref().unwrap()).map(|_| Default::default())), "WORKTREE_CHANGED");
    assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "newer\n");
}

#[test]
fn discard_rejects_paths_outside_the_repo() {
    let d = repo();
    let e = engine(&d);
    for p in ["../x", "/etc/passwd", ".git/config", "a/../../x"] {
        assert_eq!(code(e.run(&Operation::Discard { paths: vec![p.into()] })), "INVALID_PATH", "{p}");
    }
}

#[test]
fn stage_and_unstage_single_hunks() {
    let d = repo();
    let e = engine(&d);
    let base: String = (1..=40).map(|i| format!("line {i}\n")).collect();
    committed(&d, &e, "f.txt", &base);
    let edited = base.replace("line 2\n", "LINE 2\n").replace("line 38\n", "LINE 38\n");
    write(&d, "f.txt", &edited);
    e.run(&Operation::StageHunk { path: "f.txt".into(), hunk: 1, unstage: false }).unwrap();
    let staged = git(&d, &["diff", "--cached"]);
    assert!(staged.contains("+LINE 38") && !staged.contains("+LINE 2\n"), "{staged}");
    let unstaged = git(&d, &["diff"]);
    assert!(unstaged.contains("+LINE 2") && !unstaged.contains("LINE 38"), "{unstaged}");
    e.run(&Operation::StageHunk { path: "f.txt".into(), hunk: 0, unstage: true }).unwrap();
    assert_eq!(git(&d, &["diff", "--cached"]), "");
    assert_eq!(code(e.run(&Operation::StageHunk { path: "f.txt".into(), hunk: 5, unstage: false })), "INVALID_HUNK");
}

#[test]
fn reader_worktree_diff_and_rows_text() {
    use ak_core::read::{CliReader, RepoReader};
    let d = repo();
    let e = engine(&d);
    committed(&d, &e, "a.txt", "one\n");
    write(&d, "a.txt", "two\n");
    write(&d, "u.txt", "new\n");
    let r = CliReader::open(Git::discover(Mode::ReadOnly).unwrap(), &d).unwrap();
    assert!(r.worktree_diff("a.txt", false, false).unwrap().contains("+two"));
    assert_eq!(r.worktree_diff("a.txt", true, false).unwrap(), "");
    assert!(r.worktree_diff("u.txt", false, true).unwrap().contains("+new"));
    let head = r.head().unwrap().unwrap();
    let rows = r.rows_text(&[head.as_str()]).unwrap();
    assert_eq!((rows[0].summary.as_str(), rows[0].author.as_str()), ("add a.txt", "Test"));
    assert!(r.rows_text(&["--all"]).is_err());
}

#[test]
fn redo_brings_back_commit_amend_and_first_commit() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    let root = commit(&e, "root", false);
    let sha = git(&d, &["rev-parse", "HEAD"]);
    e.undo(root.id.as_deref().unwrap()).unwrap();
    e.redo_last().unwrap();
    assert_eq!(git(&d, &["rev-parse", "HEAD"]), sha);
    // amend
    let am = commit(&e, "root v2", true);
    let amended = git(&d, &["rev-parse", "HEAD"]);
    e.undo(am.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["rev-parse", "HEAD"]), sha);
    e.redo_last().unwrap();
    assert_eq!(git(&d, &["rev-parse", "HEAD"]), amended);
    assert_eq!(e.redo_last().unwrap_err().code, "NOTHING_TO_REDO");
}

#[test]
fn redo_is_cleared_by_a_new_operation_and_follows_undo_order() {
    let d = repo();
    let e = engine(&d);
    committed(&d, &e, "a.txt", "1");
    let c2 = {
        write(&d, "b.txt", "2");
        stage(&e, &["b.txt"]);
        commit(&e, "c2", false)
    };
    e.undo(c2.id.as_deref().unwrap()).unwrap();
    // A new commit makes the undone one unreachable for Redo.
    commit(&e, "c2 again", false);
    assert_eq!(e.redo_last().unwrap_err().code, "NOTHING_TO_REDO");
}

#[test]
fn redo_discard_discards_again() {
    let d = repo();
    let e = engine(&d);
    committed(&d, &e, "a.txt", "one\n");
    write(&d, "a.txt", "changed\n");
    write(&d, "u.txt", "x\n");
    let out = e.run(&Operation::Discard { paths: vec!["a.txt".into(), "u.txt".into()] }).unwrap();
    e.undo(out.id.as_deref().unwrap()).unwrap();
    assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "changed\n");
    e.redo_last().unwrap();
    assert_eq!(git(&d, &["status", "--porcelain"]), "");
    assert!(!d.join("u.txt").exists());
}

#[test]
fn hunk_staging_keeps_crlf_bytes() {
    let d = repo();
    let e = engine(&d);
    let base: String = (1..=30).map(|i| format!("line{i}\r\n")).collect();
    write(&d, "w.txt", &base);
    git(&d, &["config", "core.autocrlf", "false"]);
    stage(&e, &["w.txt"]);
    commit(&e, "base", false);
    write(&d, "w.txt", &base.replace("line2\r\n", "LINE2\r\n").replace("line29\r\n", "LINE29\r\n"));
    e.run(&Operation::StageHunk { path: "w.txt".into(), hunk: 0, unstage: false }).unwrap();
    let cached = Command::new("git").arg("-C").arg(&d).args(["diff", "--cached"]).output().unwrap().stdout;
    let cached = String::from_utf8_lossy(&cached);
    assert!(cached.contains("+LINE2\r\n") && !cached.contains("LINE29"));
}

#[test]
fn engine_opens_in_a_linked_worktree_and_stages() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    commit(&e, "base", false);
    let wt = d.with_file_name(format!("{}-wt", d.file_name().unwrap().to_string_lossy()));
    let _ = std::fs::remove_dir_all(&wt);
    git(&d, &["worktree", "add", "-q", "-b", "side", wt.to_str().unwrap()]);
    let we = engine(&wt);
    write(&wt, "b.txt", "2");
    stage(&we, &["b.txt"]);
    assert_eq!(git(&wt, &["diff", "--cached", "--name-only"]), "b.txt");
    assert_eq!(we.journal().len(), 0);
}

#[test]
fn symlink_discard_is_refused_and_conflicted_file_can_be_resolved() {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "base\n");
    stage(&e, &["a.txt"]);
    commit(&e, "base", false);
    git(&d, &["checkout", "-q", "-b", "other"]);
    write(&d, "a.txt", "other\n");
    git(&d, &["commit", "-qam", "other"]);
    git(&d, &["checkout", "-q", "main"]);
    write(&d, "a.txt", "main\n");
    git(&d, &["commit", "-qam", "main"]);
    let _ = Command::new("git").arg("-C").arg(&d).args(["merge", "other"]).output().unwrap();
    assert!(git(&d, &["status", "--porcelain"]).starts_with("UU"));
    write(&d, "a.txt", "resolved\n");
    stage(&e, &["a.txt"]);
    assert_eq!(git(&d, &["status", "--porcelain"]), "M  a.txt");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("a.txt", d.join("link")).unwrap();
        assert_eq!(code(e.run(&Operation::Discard { paths: vec!["link".into()] })), "UNSUPPORTED_PATH");
    }
}

#[test]
fn prune_journal_drops_old_entries_and_their_snapshots() {
    let d = repo();
    let e = engine(&d);
    for i in 0..3 {
        write(&d, "a.txt", &i.to_string());
        stage(&e, &["a.txt"]);
        commit(&e, &format!("c{i}"), false);
    }
    commit(&e, "c2 amended", true);
    assert_eq!(e.journal().len(), 4);
    assert_eq!(e.prune_journal(1).unwrap(), 3);
    let j = e.journal();
    assert_eq!((j.len(), j[0].summary.as_str()), (1, "c2 amended"));
    assert_eq!(e.prune_journal(1).unwrap(), 0);
    assert_eq!(git(&d, &["for-each-ref", "refs/atlaskraken/snapshots"]).lines().count(), 1);
}

fn base_repo() -> (PathBuf, Engine) {
    let d = repo();
    let e = engine(&d);
    write(&d, "a.txt", "1");
    stage(&e, &["a.txt"]);
    commit(&e, "base", false);
    (d, e)
}

fn branches(d: &Path) -> String {
    git(d, &["for-each-ref", "--format=%(refname:short)", "refs/heads"]).replace('\n', ",")
}

#[test]
fn branch_create_rename_delete_with_undo_and_redo() {
    let (d, e) = base_repo();
    let create = e.run(&Operation::BranchCreate { name: "feat".into(), at: None }).unwrap();
    assert_eq!(branches(&d), "feat,main");
    e.run(&Operation::BranchRename { old: "feat".into(), new: "feat2".into() }).unwrap();
    assert_eq!(branches(&d), "feat2,main");
    let del = e.run(&Operation::BranchDelete { name: "feat2".into(), force: false }).unwrap();
    assert_eq!(branches(&d), "main");
    e.undo(del.id.as_deref().unwrap()).unwrap();
    assert_eq!(branches(&d), "feat2,main");
    e.redo_last().unwrap();
    assert_eq!(branches(&d), "main");
    e.undo(del.id.as_deref().unwrap()).unwrap();
    let ren = e.journal().iter().rev().find(|x| x.kind == "branch_rename").unwrap().id.clone();
    e.undo(&ren).unwrap();
    assert_eq!(branches(&d), "feat,main");
    e.undo(create.id.as_deref().unwrap()).unwrap();
    assert_eq!(branches(&d), "main");
}

#[test]
fn branch_errors_have_stable_codes() {
    let (d, e) = base_repo();
    assert_eq!(code(e.run(&Operation::BranchCreate { name: "bad name".into(), at: None })), "INVALID_BRANCH_NAME");
    assert_eq!(code(e.run(&Operation::BranchCreate { name: "main".into(), at: None })), "BRANCH_EXISTS");
    assert_eq!(code(e.run(&Operation::BranchDelete { name: "main".into(), force: true })), "BRANCH_IS_CURRENT");
    git(&d, &["checkout", "-q", "-b", "side"]);
    write(&d, "b.txt", "2");
    git(&d, &["add", "b.txt"]);
    git(&d, &["commit", "-qm", "side work"]);
    git(&d, &["checkout", "-q", "main"]);
    assert_eq!(code(e.run(&Operation::BranchDelete { name: "side".into(), force: false })), "BRANCH_NOT_MERGED");
    e.run(&Operation::BranchDelete { name: "side".into(), force: true }).unwrap();
    assert_eq!(branches(&d), "main");
}

#[test]
fn checkout_branch_and_detached_with_undo() {
    let (d, e) = base_repo();
    e.run(&Operation::BranchCreate { name: "other".into(), at: None }).unwrap();
    let co = e.run(&Operation::Checkout { target: "other".into(), is_branch: true }).unwrap();
    assert_eq!(git(&d, &["symbolic-ref", "--short", "HEAD"]), "other");
    e.undo(co.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["symbolic-ref", "--short", "HEAD"]), "main");
    e.redo_last().unwrap();
    assert_eq!(git(&d, &["symbolic-ref", "--short", "HEAD"]), "other");
    e.undo(co.id.as_deref().unwrap()).unwrap();
    let sha = git(&d, &["rev-parse", "HEAD"]);
    let det = e.run(&Operation::Checkout { target: sha.clone(), is_branch: false }).unwrap();
    assert!(Command::new("git").arg("-C").arg(&d).args(["symbolic-ref", "-q", "HEAD"]).output().unwrap().status.code() == Some(1));
    e.undo(det.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["symbolic-ref", "--short", "HEAD"]), "main");
    // Dirty file that conflicts with the target blocks the switch without losing work.
    git(&d, &["checkout", "-q", "other"]);
    write(&d, "a.txt", "other\n");
    git(&d, &["commit", "-qam", "other change"]);
    git(&d, &["checkout", "-q", "main"]);
    write(&d, "a.txt", "dirty\n");
    assert_eq!(code(e.run(&Operation::Checkout { target: "other".into(), is_branch: true })), "GIT_FAILED");
    assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "dirty\n");
}

fn conflict_repo() -> (PathBuf, Engine) {
    let (d, e) = base_repo();
    git(&d, &["checkout", "-q", "-b", "feature/conflict"]);
    write(&d, "a.txt", "feature\n");
    git(&d, &["commit", "-qam", "feature"]);
    git(&d, &["checkout", "-q", "main"]);
    write(&d, "a.txt", "main\n");
    git(&d, &["commit", "-qam", "main"]);
    (d, e)
}

#[test]
fn merge_preview_predicts_conflicts_then_ours_resolution_makes_two_parents() {
    let (d, e) = conflict_repo();
    let p = e.merge_preview("feature/conflict").unwrap();
    assert!(!p.fast_forward && !p.up_to_date);
    assert_eq!(p.conflicts, Some(vec!["a.txt".to_string()]));
    // Old-Git path (used below 2.38) must agree.
    assert_eq!(e.merge_conflicts_legacy("feature/conflict").unwrap(), vec!["a.txt".to_string()]);
    let before = git(&d, &["status", "--porcelain"]);
    let out = e.run(&Operation::Merge { source: "feature/conflict".into() }).unwrap();
    assert!(out.conflicted && e.merge_in_progress());
    // Abort restores the pre-merge status (PRD criterion 6).
    e.run(&Operation::MergeAbort).unwrap();
    assert_eq!(git(&d, &["status", "--porcelain"]), before);
    // Resolve with "ours" and commit (PRD criterion 5).
    e.run(&Operation::Merge { source: "feature/conflict".into() }).unwrap();
    e.run(&Operation::ResolveConflict { path: "a.txt".into(), theirs: false }).unwrap();
    assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "main\n");
    git(&d, &["commit", "-q", "--no-edit"]);
    assert_eq!(git(&d, &["log", "-1", "--format=%P"]).split(' ').count(), 2);
}

#[test]
fn clean_merge_and_fast_forward_undo_redo() {
    let (d, e) = base_repo();
    git(&d, &["checkout", "-q", "-b", "side"]);
    write(&d, "b.txt", "2");
    git(&d, &["add", "b.txt"]);
    git(&d, &["commit", "-qm", "side"]);
    git(&d, &["checkout", "-q", "main"]);
    let base = git(&d, &["rev-parse", "HEAD"]);
    assert!(e.merge_preview("side").unwrap().fast_forward);
    let m = e.run(&Operation::Merge { source: "side".into() }).unwrap();
    assert_ne!(git(&d, &["rev-parse", "HEAD"]), base);
    e.undo(m.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["rev-parse", "HEAD"]), base);
    e.redo_last().unwrap();
    assert!(d.join("b.txt").exists());
    assert_eq!(code(e.run(&Operation::Merge { source: "side".into() })), "ALREADY_UP_TO_DATE");
}

#[test]
fn stash_push_pop_leaves_tree_identical_and_drop_is_undoable() {
    let (d, e) = base_repo();
    write(&d, "a.txt", "changed\n");
    let before = git(&d, &["diff"]);
    assert_eq!(code(e.run(&Operation::StashPush { message: None, include_untracked: false }).and_then(|_| e.run(&Operation::StashPush { message: None, include_untracked: false }))), "NOTHING_TO_STASH");
    assert_eq!(git(&d, &["diff"]), "");
    assert_eq!(e.stashes().unwrap().len(), 1);
    assert_eq!(e.stashes().unwrap()[0].base, git(&d, &["rev-parse", "HEAD"]));
    e.run(&Operation::StashPop { index: 0 }).unwrap();
    assert_eq!(git(&d, &["diff"]), before);
    assert!(e.stashes().unwrap().is_empty());
    // Undo a push puts the changes back; undo a drop restores the entry.
    let push = e.run(&Operation::StashPush { message: Some("wip".into()), include_untracked: false }).unwrap();
    e.undo(push.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["diff"]), before);
    e.run(&Operation::StashPush { message: Some("wip".into()), include_untracked: false }).unwrap();
    let drop = e.run(&Operation::StashDrop { index: 0 }).unwrap();
    assert!(e.stashes().unwrap().is_empty());
    e.undo(drop.id.as_deref().unwrap()).unwrap();
    assert_eq!(e.stashes().unwrap().len(), 1);
    assert_eq!(code(e.run(&Operation::StashApply { index: 7 })), "INVALID_STASH");
}

#[test]
fn stash_apply_brings_changes_back_and_keeps_the_stash() {
    let (d, e) = base_repo();
    write(&d, "a.txt", "changed\n");
    let before = git(&d, &["diff"]);
    e.run(&Operation::StashPush { message: Some("keep me".into()), include_untracked: false }).unwrap();
    assert_eq!(git(&d, &["diff"]), "");
    e.run(&Operation::StashApply { index: 0 }).unwrap();
    assert_eq!(git(&d, &["diff"]), before);
    let list = e.stashes().unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].message.contains("keep me"));
}

#[test]
fn resolve_conflict_with_theirs_takes_the_merged_branch_version() {
    let (d, e) = conflict_repo();
    let out = e.run(&Operation::Merge { source: "feature/conflict".into() }).unwrap();
    assert!(out.conflicted);
    e.run(&Operation::ResolveConflict { path: "a.txt".into(), theirs: true }).unwrap();
    assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "feature\n");
    assert_eq!(git(&d, &["diff", "--name-only", "--diff-filter=U"]), "");
    git(&d, &["commit", "-q", "--no-edit"]);
    assert_eq!(git(&d, &["log", "-1", "--format=%P"]).split(' ').count(), 2);
}

#[test]
fn conflict_versions_and_resolve_with_edited_content() {
    let (d, e) = conflict_repo();
    e.run(&Operation::Merge { source: "feature/conflict".into() }).unwrap();
    let v = e.conflict_versions("a.txt").unwrap();
    assert_eq!((v.ours.as_deref(), v.theirs.as_deref()), (Some("main\n"), Some("feature\n")));
    assert!(v.merged.contains("<<<<<<<") && v.base.is_some());
    e.run(&Operation::ResolveWith { path: "a.txt".into(), content: "both\n".into() }).unwrap();
    assert_eq!(git(&d, &["diff", "--name-only", "--diff-filter=U"]), "");
    assert_eq!(git(&d, &["show", ":a.txt"]), "both");
    assert_eq!(code(e.run(&Operation::ResolveWith { path: "../x".into(), content: String::new() })), "INVALID_PATH");
}

fn side_commit(d: &Path, file: &str, text: &str) -> String {
    git(d, &["checkout", "-q", "-b", "side"]);
    write(d, file, text);
    git(d, &["add", file]);
    git(d, &["commit", "-qm", "side change"]);
    let sha = git(d, &["rev-parse", "HEAD"]);
    git(d, &["checkout", "-q", "main"]);
    sha
}

#[test]
fn cherry_pick_and_revert_with_undo_redo() {
    let (d, e) = base_repo();
    let sha = side_commit(&d, "b.txt", "side\n");
    let before = git(&d, &["rev-parse", "HEAD"]);
    let cp = e.run(&Operation::CherryPick { sha: sha.clone() }).unwrap();
    assert_eq!(std::fs::read_to_string(d.join("b.txt")).unwrap(), "side\n");
    assert_eq!(code(e.run(&Operation::CherryPick { sha: sha.clone() })), "NOTHING_TO_COMMIT");
    assert!(e.in_progress().is_none());
    e.undo(cp.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["rev-parse", "HEAD"]), before);
    e.redo_last().unwrap();
    let rv = e.run(&Operation::Revert { sha: "HEAD".into() }).unwrap();
    assert!(!d.join("b.txt").exists());
    assert!(git(&d, &["log", "-1", "--format=%s"]).starts_with("Revert"));
    e.undo(rv.id.as_deref().unwrap()).unwrap();
    assert!(d.join("b.txt").exists());
}

#[test]
fn cherry_pick_conflict_can_be_aborted() {
    let (d, e) = base_repo();
    let sha = side_commit(&d, "a.txt", "side\n");
    write(&d, "a.txt", "main\n");
    git(&d, &["commit", "-qam", "main change"]);
    let before = git(&d, &["status", "--porcelain"]);
    assert!(e.run(&Operation::CherryPick { sha }).unwrap().conflicted);
    assert_eq!(e.in_progress(), Some("cherry-pick"));
    assert_eq!(code(e.run(&Operation::Revert { sha: "HEAD".into() })), "MERGE_IN_PROGRESS");
    e.run(&Operation::MergeAbort).unwrap();
    assert!(e.in_progress().is_none());
    assert_eq!(git(&d, &["status", "--porcelain"]), before);
}

#[test]
fn hard_reset_undo_restores_commit_and_uncommitted_work() {
    let (d, e) = base_repo();
    write(&d, "a.txt", "2");
    stage(&e, &["a.txt"]);
    commit(&e, "second", false);
    let head = git(&d, &["rev-parse", "HEAD"]);
    write(&d, "a.txt", "uncommitted");
    let r = e.run(&Operation::Reset { sha: "HEAD~1".into(), mode: "hard".into() }).unwrap();
    assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "1");
    e.undo(r.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["rev-parse", "HEAD"]), head);
    assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "uncommitted");
    // Redo refuses while there is uncommitted work it would destroy.
    assert_eq!(code(e.redo_last()), "WORKTREE_CHANGED");
    let soft = e.run(&Operation::Reset { sha: "HEAD~1".into(), mode: "soft".into() }).unwrap();
    assert_eq!(git(&d, &["diff", "--cached", "--name-only"]), "a.txt");
    e.undo(soft.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["rev-parse", "HEAD"]), head);
    assert_eq!(code(e.run(&Operation::Reset { sha: "HEAD".into(), mode: "bogus".into() })), "INVALID_REF");
}

#[test]
fn tags_create_delete_with_undo() {
    let (d, e) = base_repo();
    e.run(&Operation::TagCreate { name: "v1".into(), at: "HEAD".into(), message: None }).unwrap();
    e.run(&Operation::TagCreate { name: "v2".into(), at: "HEAD".into(), message: Some("Release 2".into()) }).unwrap();
    assert_eq!(git(&d, &["cat-file", "-t", "v2"]), "tag");
    assert_eq!(code(e.run(&Operation::TagCreate { name: "v1".into(), at: "HEAD".into(), message: None })), "TAG_EXISTS");
    let obj = git(&d, &["rev-parse", "refs/tags/v2"]);
    let del = e.run(&Operation::TagDelete { name: "v2".into() }).unwrap();
    assert_eq!(git(&d, &["tag"]), "v1");
    e.undo(del.id.as_deref().unwrap()).unwrap();
    assert_eq!(git(&d, &["rev-parse", "refs/tags/v2"]), obj);
    e.redo_last().unwrap();
    assert_eq!(git(&d, &["tag"]), "v1");
}
