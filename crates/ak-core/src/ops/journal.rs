// SPDX-License-Identifier: Apache-2.0
//! Operation journal: one JSON line per undoable operation, stored in
//! `<git-dir>/atlaskraken/journal.jsonl` (never in the working tree).

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    pub time: i64,
    /// `commit`, `commit_amend` or `discard`.
    pub kind: String,
    pub summary: String,
    pub branch: Option<String>,
    pub head_before: Option<String>,
    pub head_after: Option<String>,
    /// Ref keeping the pre-operation state reachable, if one was taken.
    pub snapshot_ref: Option<String>,
    /// git commands run (for the "show exact commands" journal view).
    pub commands: Vec<String>,
    pub undone: bool,
    /// When it was undone (ns since epoch); orders Redo. 0 while not undone.
    #[serde(default)]
    pub undone_at: i128,
    /// Discard: tracked files whose changes were backed up (restored on undo).
    #[serde(default)]
    pub paths: Vec<String>,
    /// Discard: untracked files that were deleted (restored on undo).
    #[serde(default)]
    pub untracked: Vec<String>,
}

pub struct Journal {
    path: PathBuf,
}

impl Journal {
    pub fn new(git_dir: &Path) -> Self {
        Self { path: git_dir.join("atlaskraken").join("journal.jsonl") }
    }

    pub fn append(&self, e: &JournalEntry) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&self.path)?;
        writeln!(f, "{}", serde_json::to_string(e).map_err(std::io::Error::other)?)
    }

    /// All entries, oldest first. Unreadable lines are skipped.
    pub fn list(&self) -> Vec<JournalEntry> {
        std::fs::read_to_string(&self.path)
            .unwrap_or_default()
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    /// Keeps only the newest `keep` entries; returns the dropped ones (oldest first).
    pub fn prune(&self, keep: usize) -> std::io::Result<Vec<JournalEntry>> {
        let mut entries = self.list();
        if entries.len() <= keep {
            return Ok(vec![]);
        }
        let kept = entries.split_off(entries.len() - keep);
        let text: String = kept
            .iter()
            .map(|e| serde_json::to_string(e).map(|s| s + "\n"))
            .collect::<Result<_, _>>()
            .map_err(std::io::Error::other)?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(entries)
    }

    /// Marks an entry as undone or redone (atomic rewrite). Returns false if the id is unknown.
    pub fn set_undone(&self, id: &str, undone: bool, at: i128) -> std::io::Result<bool> {
        let mut entries = self.list();
        let Some(e) = entries.iter_mut().find(|e| e.id == id) else { return Ok(false) };
        e.undone = undone;
        e.undone_at = if undone { at } else { 0 };
        let text: String = entries
            .iter()
            .map(|e| serde_json::to_string(e).map(|s| s + "\n"))
            .collect::<Result<_, _>>()
            .map_err(std::io::Error::other)?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(true)
    }
}
