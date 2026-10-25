// SPDX-License-Identifier: Apache-2.0
//! Session state in the app config directory (`session.json`): the open tabs, the active tab and
//! the recent repositories. Written atomically on each change; a missing or damaged file reads as
//! an empty session. No Tauri dependency, so its logic is unit-tested on its own.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// How many recent repositories are kept (most recent first).
pub const RECENTS_MAX: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recent {
    pub path: String,
    pub name: String,
    pub last_opened: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Session {
    /// Repository roots, in tab order.
    pub tabs: Vec<String>,
    /// Index into `tabs` of the tab that was in front.
    pub active: usize,
    pub recents: Vec<Recent>,
}

/// A session file as read from disk. Missing or unreadable files read as an empty session.
pub fn read(path: &Path) -> Session {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Atomic write, like the settings files.
pub fn write(path: &Path, session: &Session) -> std::io::Result<()> {
    let value = serde_json::to_value(session).map_err(std::io::Error::other)?;
    crate::settings::write(path, &value)
}

/// Moves `path` to the front (most recent first), dropping its older entry, and keeps at most
/// `RECENTS_MAX` entries.
pub fn touch_recent(recents: &mut Vec<Recent>, path: &str, name: &str, now: u64) {
    recents.retain(|r| r.path != path);
    recents.insert(0, Recent { path: path.to_string(), name: name.to_string(), last_opened: now });
    recents.truncate(RECENTS_MAX);
}

pub fn remove_recent(recents: &mut Vec<Recent>, path: &str) {
    recents.retain(|r| r.path != path);
}

/// The tabs that can still be restored (`exists` tells whether a root is still there), and the
/// index of the tab that should be in front among them. If the active tab is gone, the index points
/// at the tab that followed it, clamped to the last one.
pub fn restorable(session: &Session, exists: impl Fn(&str) -> bool) -> (Vec<String>, usize) {
    let mut tabs = Vec::new();
    let mut before_active = 0;
    for (i, tab) in session.tabs.iter().enumerate() {
        if !exists(tab) {
            continue;
        }
        if i < session.active {
            before_active += 1;
        }
        tabs.push(tab.clone());
    }
    let active = before_active.min(tabs.len().saturating_sub(1));
    (tabs, active)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recent(path: &str, at: u64) -> Recent {
        Recent { path: path.into(), name: path.into(), last_opened: at }
    }

    #[test]
    fn touching_moves_to_front_without_duplicates() {
        let mut list = vec![recent("/a", 1), recent("/b", 2)];
        touch_recent(&mut list, "/b", "b", 3);
        assert_eq!(list, vec![Recent { path: "/b".into(), name: "b".into(), last_opened: 3 }, recent("/a", 1)]);
    }

    #[test]
    fn list_is_capped() {
        let mut list = Vec::new();
        for i in 0..(RECENTS_MAX as u64 + 5) {
            touch_recent(&mut list, &format!("/r{i}"), "r", i);
        }
        assert_eq!(list.len(), RECENTS_MAX);
        assert_eq!(list[0].path, format!("/r{}", RECENTS_MAX as u64 + 4));
        assert!(!list.iter().any(|r| r.path == "/r0"), "the oldest entries are dropped");
    }

    #[test]
    fn remove_drops_only_that_path() {
        let mut list = vec![recent("/a", 1), recent("/b", 2)];
        remove_recent(&mut list, "/a");
        assert_eq!(list, vec![recent("/b", 2)]);
        remove_recent(&mut list, "/missing");
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn restore_skips_missing_tabs_and_keeps_the_active_one() {
        let s = Session { tabs: vec!["/a".into(), "/gone".into(), "/c".into()], active: 2, recents: vec![] };
        let (tabs, active) = restorable(&s, |p| p != "/gone");
        assert_eq!(tabs, vec!["/a".to_string(), "/c".to_string()]);
        assert_eq!(active, 1, "/c is still in front");
    }

    #[test]
    fn restore_points_at_the_next_tab_when_the_active_one_is_gone() {
        let s = Session { tabs: vec!["/a".into(), "/gone".into(), "/c".into()], active: 1, recents: vec![] };
        let (tabs, active) = restorable(&s, |p| p != "/gone");
        assert_eq!(tabs.len(), 2);
        assert_eq!(active, 1, "/c takes the place of the missing tab");
        let last = Session { tabs: vec!["/a".into(), "/gone".into()], active: 1, recents: vec![] };
        assert_eq!(restorable(&last, |p| p != "/gone").1, 0);
    }

    #[test]
    fn restore_of_nothing_is_empty() {
        let s = Session { tabs: vec!["/gone".into()], active: 0, recents: vec![] };
        assert_eq!(restorable(&s, |_| false), (vec![], 0));
        assert_eq!(restorable(&Session::default(), |_| true), (vec![], 0));
    }

    #[test]
    fn files_round_trip_and_bad_files_read_empty() {
        let dir = std::env::temp_dir().join(format!("gitatlas-session-{}", std::process::id()));
        let file = dir.join("session.json");
        assert_eq!(read(&file), Session::default(), "missing file");
        let s = Session { tabs: vec!["/a".into()], active: 0, recents: vec![recent("/a", 7)] };
        write(&file, &s).unwrap();
        assert_eq!(read(&file), s);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"lastOpened\""), "camelCase keys on disk");
        std::fs::write(&file, "{not json").unwrap();
        assert_eq!(read(&file), Session::default(), "corrupt file");
        std::fs::write(&file, "[1]").unwrap();
        assert_eq!(read(&file), Session::default(), "not an object");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
