// SPDX-License-Identifier: Apache-2.0
//! Merge (Phase 3): conflict preview, merge, abort and per-file conflict resolution.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::journal::JournalEntry;
use super::lock::{self, Lock};
use super::{literal, new_id, now, Engine, OpError, OpOutcome, SNAPSHOT_PREFIX};

/// The three sides of a conflicted file plus the working-tree file (with conflict markers).
/// A side is `None` when the file does not exist there (added/deleted on one side).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ConflictVersions {
    pub base: Option<String>,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    /// The working-tree file as it is now (with conflict markers until it is resolved).
    pub merged: String,
    /// The same conflicts rendered in diff3 style, so each block also carries its base lines.
    /// `None` when one of the three sides is missing (add/add or modify/delete conflicts).
    pub merged_diff3: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MergePreview {
    pub up_to_date: bool,
    pub fast_forward: bool,
    /// Files that will conflict; `None` when this Git cannot predict it (before 2.38).
    pub conflicts: Option<Vec<String>>,
}

impl Engine {
    /// The unfinished operation that stopped on conflicts, if any: `merge`, `cherry-pick`, `revert` or `rebase`.
    pub fn in_progress(&self) -> Option<&'static str> {
        let has = |f: &str| self.git_dir.join(f).exists();
        if has("rebase-merge") || has("rebase-apply") {
            Some("rebase")
        } else if has("CHERRY_PICK_HEAD") {
            Some("cherry-pick")
        } else if has("REVERT_HEAD") {
            Some("revert")
        } else if has("MERGE_HEAD") {
            Some("merge")
        } else {
            None
        }
    }

    pub fn merge_in_progress(&self) -> bool {
        self.in_progress().is_some()
    }

    pub(super) fn op_in_progress() -> OpError {
        OpError::new("MERGE_IN_PROGRESS", "A merge, cherry-pick, revert or rebase is already in progress.", Some("Finish it or abort it first."))
    }

    fn is_ancestor(&self, a: &str, b: &str) -> bool {
        self.git.run_status(&self.root, ["merge-base", "--is-ancestor", a, b]).map(|(c, ..)| c == Some(0)).unwrap_or(false)
    }

    /// What merging `source` into HEAD would do, without touching the repository.
    pub fn merge_preview(&self, source: &str) -> Result<MergePreview, OpError> {
        let rev = format!("{source}^{{commit}}");
        if self.git.run(&self.root, ["rev-parse", "-q", "--verify", rev.as_str()]).is_err() {
            return Err(OpError::new("INVALID_REF", "That branch or commit does not exist.", None));
        }
        if self.is_ancestor(source, "HEAD") {
            return Ok(MergePreview { up_to_date: true, ..Default::default() });
        }
        if self.is_ancestor("HEAD", source) {
            return Ok(MergePreview { fast_forward: true, conflicts: Some(vec![]), ..Default::default() });
        }
        if !self.git.caps().merge_tree_write_tree {
            return Ok(MergePreview { conflicts: Some(self.merge_conflicts_legacy(source)?), ..Default::default() });
        }
        let (code, out, err) = self.git.run_status(&self.root, ["merge-tree", "--write-tree", "--name-only", "HEAD", source])?;
        let conflicts = match code {
            Some(0) => vec![],
            // Line 1 is the result tree; file names follow until the first blank line.
            Some(1) => out.lines().skip(1).take_while(|l| !l.is_empty()).map(str::to_owned).collect(),
            _ => return Err(OpError::new("GIT_FAILED", "git merge-tree failed.", None).with_detail(err)),
        };
        Ok(MergePreview { conflicts: Some(conflicts), ..Default::default() })
    }

    pub(super) fn merge(&self, source: &str) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        if self.merge_in_progress() {
            return Err(Self::op_in_progress());
        }
        let (Some(before), Some(branch)) = (self.head(), self.branch()) else {
            return Err(OpError::new("INVALID_REF", "Merging needs a branch with at least one commit checked out.", None));
        };
        let preview = self.merge_preview(source)?;
        if preview.up_to_date {
            return Err(OpError::new("ALREADY_UP_TO_DATE", "That branch is already merged.", None));
        }
        let (code, out, err) = self.git.run_status(&self.root, ["merge", "--no-edit", source])?;
        if code != Some(0) {
            if self.merge_in_progress() {
                return Ok(OpOutcome { head: self.head(), conflicted: true, ..Default::default() });
            }
            return Err(OpError::new("GIT_FAILED", "git merge failed.", None).with_detail(if err.is_empty() { out } else { err }));
        }
        let after = self.head();
        let id = new_id();
        if let Some(a) = &after {
            // Keep the result reachable so Redo can bring it back after an Undo.
            self.git.run(&self.root, ["update-ref", format!("{SNAPSHOT_PREFIX}{id}-redo").as_str(), a.as_str()])?;
        }
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: "merge".into(),
            summary: format!("Merge {source} into {branch}"),
            branch: Some(branch),
            head_before: Some(before),
            head_after: after.clone(),
            commands: vec![format!("git merge --no-edit {source}")],
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head: after, ..Default::default() })
    }

    /// Undo/redo of a merge: move the branch with `reset --merge`, which refuses to
    /// clobber local changes. Only allowed while the branch is where the merge left it.
    pub(super) fn apply_merge_entry(&self, e: &JournalEntry, forward: bool) -> Result<(), OpError> {
        let (here, go) = if forward { (&e.head_before, &e.head_after) } else { (&e.head_after, &e.head_before) };
        if self.head() != *here || self.branch() != e.branch {
            return Err(OpError::new("REF_MOVED", "The branch changed since that merge, so it cannot be undone or redone safely.", Some("Undo the newer operations first.")));
        }
        let go = go.as_deref().unwrap_or_default();
        self.git.run(&self.root, ["reset", "-q", "--merge", go])?;
        Ok(())
    }

    pub(super) fn merge_abort(&self) -> Result<OpOutcome, OpError> {
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let Some(what) = self.in_progress() else {
            return Err(OpError::new("NO_MERGE_IN_PROGRESS", "There is no merge to abort.", None));
        };
        self.git.run(&self.root, [what, "--abort"])?;
        Ok(OpOutcome { head: self.head(), ..Default::default() })
    }

    pub(super) fn resolve_conflict(&self, path: &str, theirs: bool) -> Result<OpOutcome, OpError> {
        super::check_rel(path)?;
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let spec = literal(path);
        let side = if theirs { "--theirs" } else { "--ours" };
        self.git.run(&self.root, ["checkout", side, "--", spec.as_str()])?;
        self.git.run(&self.root, ["add", "--", spec.as_str()])?;
        Ok(OpOutcome::default())
    }

    /// Index stages 1/2/3 (base/ours/theirs) of a conflicted file, for the 3-pane editor.
    pub fn conflict_versions(&self, path: &str) -> Result<ConflictVersions, OpError> {
        super::check_rel(path)?;
        let stage = |n: u8| {
            let spec = format!(":{n}:{path}");
            self.git.run(&self.root, ["show", spec.as_str()]).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
        };
        let (base, ours, theirs) = (stage(1), stage(2), stage(3));
        let merged = std::fs::read(self.root.join(path)).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
        let merged_diff3 = match (&ours, &base, &theirs) {
            (Some(o), Some(b), Some(t)) => self.diff3_render(o, b, t),
            _ => None,
        };
        Ok(ConflictVersions { base, ours, theirs, merged, merged_diff3 })
    }

    /// Re-runs the three-way merge of the given sides with `git merge-file --diff3`, in a
    /// temporary directory inside the git dir. `None` if Git cannot do it.
    fn diff3_render(&self, ours: &str, base: &str, theirs: &str) -> Option<String> {
        let dir = self.git_dir.join("atlaskraken").join("tmp").join(new_id());
        let files: Option<Vec<String>> = [("ours", ours), ("base", base), ("theirs", theirs)]
            .into_iter()
            .map(|(name, text)| {
                let p = dir.join(name);
                write_file(&p, text.as_bytes()).ok().map(|_| p.to_string_lossy().into_owned())
            })
            .collect();
        let rendered = files.and_then(|f| {
            let args = ["merge-file", "-p", "--diff3", "-L", "ours", "-L", "base", "-L", "theirs"]
                .into_iter()
                .map(str::to_owned)
                .chain(f)
                .collect::<Vec<String>>();
            // merge-file exits with the number of conflicts (0-127); anything else is an error.
            match self.git.run_status(&self.root, args) {
                Ok((Some(code), out, _)) if (0..128).contains(&code) => Some(out),
                _ => None,
            }
        });
        let _ = std::fs::remove_dir_all(&dir);
        rendered
    }

    /// Writes the user's resolution and stages it. Journaled as `resolve`: Undo puts the
    /// conflict back (file and index stages), Redo applies the resolution again.
    pub(super) fn resolve_with(&self, path: &str, content: &str) -> Result<OpOutcome, OpError> {
        super::check_rel(path)?;
        let _g = lock::try_acquire(&self.root, &[Lock::Index]).map_err(|_| Self::busy())?;
        let spec = literal(path);
        let file = self.root.join(path);
        let before = match std::fs::read(&file) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let stages = self.index_records(path)?;
        let id = new_id();
        let dir = self.resolve_dir(&id);
        // Keep both sides of the change before touching anything.
        if let Some(bytes) = &before {
            write_file(&dir.join("before").join(path), bytes)?;
        }
        write_file(&dir.join("after").join(path), content.as_bytes())?;
        let head = self.head();
        write_atomic(&file, content.as_bytes())?;
        self.git.run(&self.root, ["add", "--", spec.as_str()])?;
        self.journal.append(&JournalEntry {
            id: id.clone(),
            time: now(),
            kind: "resolve".into(),
            summary: path.to_owned(),
            branch: self.branch(),
            head_before: head.clone(),
            head_after: head.clone(),
            commands: vec!["write resolved file".into(), format!("git add -- {path}")],
            paths: vec![path.to_owned()],
            args: stages,
            ..Default::default()
        })?;
        Ok(OpOutcome { id: Some(id), head, ..Default::default() })
    }

    fn resolve_dir(&self, id: &str) -> PathBuf {
        self.git_dir.join("atlaskraken").join("resolved").join(id)
    }

    /// `ls-files -s` records (`mode sha stage<TAB>path`) of one path: the index state Undo restores.
    fn index_records(&self, path: &str) -> Result<Vec<String>, OpError> {
        let spec = literal(path);
        let out = self.git.run(&self.root, ["ls-files", "-s", "-z", "--", spec.as_str()])?;
        Ok(String::from_utf8_lossy(&out).split('\0').filter(|s| !s.is_empty()).map(str::to_owned).collect())
    }

    /// Guards Undo/Redo of a resolution: the same merge must still be open and HEAD must not
    /// have moved, and the file must still hold `expect` ("before" or "after" the resolution).
    fn check_resolve(&self, e: &JournalEntry, path: &str, expect: &str) -> Result<(), OpError> {
        let conflicted = e.args.iter().any(|r| r.split_whitespace().nth(2).is_some_and(|s| s != "0"));
        if self.head() != e.head_after || (conflicted && self.in_progress().is_none()) {
            return Err(OpError::new(
                "NOT_IN_MERGE",
                "The merge was finished or aborted, so this resolution cannot be undone or redone safely.",
                Some("Undo the newer operations first."),
            ));
        }
        let want = std::fs::read(self.resolve_dir(&e.id).join(expect).join(path)).ok();
        let have = std::fs::read(self.root.join(path)).ok();
        if want != have {
            return Err(OpError::new(
                "WORKTREE_CHANGED",
                "This file changed after the resolution, so restoring it would overwrite newer work.",
                Some("Save or discard the newer changes first."),
            ));
        }
        Ok(())
    }

    /// Undo of a resolution: the conflicted file and its index stages come back.
    pub(super) fn undo_resolve(&self, e: &JournalEntry) -> Result<(), OpError> {
        let path = e.paths.first().ok_or_else(|| OpError::new("UNDO_NOT_SUPPORTED", "This operation cannot be undone.", None))?;
        self.check_resolve(e, path, "after")?;
        let file = self.root.join(path);
        match std::fs::read(self.resolve_dir(&e.id).join("before").join(path)) {
            Ok(bytes) => write_atomic(&file, &bytes)?,
            // The file did not exist before the resolution.
            Err(_) => {
                let _ = std::fs::remove_file(&file);
            }
        }
        self.git.run(&self.root, ["update-index", "--force-remove", "--", path.as_str()])?;
        if !e.args.is_empty() {
            let mut input = e.args.join("\n");
            input.push('\n');
            self.git.run_with_input(&self.root, ["update-index", "--index-info"], input.as_bytes())?;
        }
        Ok(())
    }

    /// Redo of a resolution: applies the saved resolved content again and stages it.
    pub(super) fn redo_resolve(&self, e: &JournalEntry) -> Result<(), OpError> {
        let path = e.paths.first().ok_or_else(|| OpError::new("UNDO_NOT_SUPPORTED", "This operation cannot be redone.", None))?;
        self.check_resolve(e, path, "before")?;
        let after = std::fs::read(self.resolve_dir(&e.id).join("after").join(path))?;
        write_atomic(&self.root.join(path), &after)?;
        let spec = literal(path);
        self.git.run(&self.root, ["add", "--", spec.as_str()])?;
        Ok(())
    }

    /// Conflict prediction for Git < 2.38 with the old 3-argument `git merge-tree`.
    #[doc(hidden)]
    pub fn merge_conflicts_legacy(&self, source: &str) -> Result<Vec<String>, OpError> {
        let base = self.git.run(&self.root, ["merge-base", "HEAD", source])?;
        let base = String::from_utf8_lossy(&base).trim().to_owned();
        let out = self.git.run(&self.root, ["merge-tree", base.as_str(), "HEAD", source])?;
        Ok(parse_legacy_merge_tree(&String::from_utf8_lossy(&out)))
    }
}

/// Creates the parent directories of `file` and writes it.
fn write_file(file: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(file, bytes)
}

/// Writes `bytes` to a temporary file next to `file` and renames it over `file`, so a crash
/// leaves either the old or the new content. Keeps the permissions of the existing file.
fn write_atomic(file: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = file.with_file_name(format!(".{name}.atlaskraken-tmp"));
    write_file(&tmp, bytes)?;
    if let Ok(meta) = std::fs::metadata(file) {
        std::fs::set_permissions(&tmp, meta.permissions())?;
    }
    std::fs::rename(&tmp, file).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Files whose merged content contains conflict markers in old `git merge-tree` output.
/// shortcut: modify/delete conflicts (no markers) are not reported; upgrade with a gix 3-way merge if needed.
fn parse_legacy_merge_tree(out: &str) -> Vec<String> {
    let mut files: Vec<String> = vec![];
    let mut path = String::new();
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("  ") {
            // "  our    100644 <sha> <path>" (the path may contain spaces)
            if let Some(p) = rest.split_whitespace().nth(2).and_then(|sha| rest.split_once(sha)).map(|(_, p)| p.trim_start()) {
                path = p.to_owned();
            }
        } else if line.starts_with("+<<<<<<< ") && !files.contains(&path) {
            files.push(path.clone());
        }
    }
    files
}

#[cfg(test)]
mod tests {
    #[test]
    fn legacy_merge_tree_lists_files_with_markers_only() {
        let out = "changed in both\n  base   100644 aaa a b.txt\n  our    100644 bbb a b.txt\n  their  100644 ccc a b.txt\n@@ -1 +1,5 @@\n+<<<<<<< .our\n m\n+=======\n+f\n+>>>>>>> .their\nmerged\n  result 100644 ddd c.txt\n  our    100644 eee c.txt\n@@ -1 +1 @@\n-x\n+y\n";
        assert_eq!(super::parse_legacy_merge_tree(out), vec!["a b.txt".to_string()]);
    }
}
