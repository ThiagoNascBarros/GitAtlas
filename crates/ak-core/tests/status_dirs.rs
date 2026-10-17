// SPDX-License-Identifier: Apache-2.0
//! A new (untracked) folder is one status entry. Its files are listed only when the user acts on
//! it: diff shows every file, discard removes them and can be undone (PERF-06).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use ak_core::ops::{Engine, Operation};
use ak_core::read::{CliReader, RepoReader};
use ak_gitcli::{Git, Mode};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// Repository with one commit (`a.txt`) and an untracked folder `newdir/` holding two files.
fn repo_with_new_folder() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("status-dirs-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("newdir/sub")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    git(&dir, &["config", "user.name", "Test"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    std::fs::write(dir.join("a.txt"), "one\n").unwrap();
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-q", "-m", "init"]);
    std::fs::write(dir.join("newdir/x.txt"), "x\n").unwrap();
    std::fs::write(dir.join("newdir/sub/y.txt"), "y\n").unwrap();
    dir
}

fn reader(dir: &Path) -> CliReader {
    CliReader::open(Git::discover(Mode::ReadOnly).unwrap(), dir).unwrap()
}

#[test]
fn status_lists_a_new_folder_as_one_entry() {
    let d = repo_with_new_folder();
    let entries: Vec<(String, bool)> = reader(&d).status().unwrap().entries.into_iter().map(|e| (e.path, e.untracked)).collect();
    assert_eq!(entries, vec![("newdir/".to_owned(), true)]);
}

#[test]
fn diff_of_a_new_folder_shows_every_file_as_added() {
    let d = repo_with_new_folder();
    let diff = reader(&d).worktree_diff("newdir/", false, true).unwrap();
    assert!(diff.contains("+++ b/newdir/x.txt"), "{diff}");
    assert!(diff.contains("+++ b/newdir/sub/y.txt"), "{diff}");
}

#[test]
fn discard_of_a_new_folder_removes_its_files_and_undo_restores_them() {
    let d = repo_with_new_folder();
    let e = Engine::open(Git::discover(Mode::ReadWrite).unwrap(), &d).unwrap();
    let out = e.run(&Operation::Discard { paths: vec!["newdir/".into()] }).unwrap();
    assert!(!d.join("newdir/x.txt").exists());
    assert!(!d.join("newdir/sub/y.txt").exists());
    assert_eq!(git(&d, &["status", "--porcelain"]), "");
    e.undo(out.id.as_deref().unwrap()).unwrap();
    assert_eq!(std::fs::read_to_string(d.join("newdir/sub/y.txt")).unwrap(), "y\n");
    assert_eq!(std::fs::read_to_string(d.join("newdir/x.txt")).unwrap(), "x\n");
}
