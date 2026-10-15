// SPDX-License-Identifier: Apache-2.0
//! Shell settings kept as JSON: a global file in the app config directory and an optional
//! per-repository override in `<git-dir>/atlasgit/settings.json`. Both are written only when the
//! user changes a setting. Unknown keys are preserved, so a newer version's settings survive.
//!
//! This module has no Tauri dependency, so its logic is unit-tested on its own.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// `sharedStash.fetchMode` (docs/specs/shared-stash.md §5.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FetchMode {
    /// Fetch shared stashes only while the "Shared stash" tab is open (default).
    #[default]
    Panel,
    /// Also on every Fetch/Pull, with the tab closed.
    Always,
}

impl FetchMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Panel => "panel",
            Self::Always => "always",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "panel" => Some(Self::Panel),
            "always" => Some(Self::Always),
            _ => None,
        }
    }
}

/// Repository setting, else global default, else `panel`.
pub fn resolve_fetch_mode(repo: Option<FetchMode>, global: Option<FetchMode>) -> FetchMode {
    repo.or(global).unwrap_or_default()
}

/// Whether a plain Fetch/Pull should also download shared stashes. In `panel` mode that happens
/// only while the tab is open (it asks for fresh data anyway); `always` ignores the tab.
pub fn fetch_with_net_op(mode: FetchMode, tab_open: bool) -> bool {
    mode == FetchMode::Always || tab_open
}

/// A settings file as read from disk. Missing or unreadable files read as empty.
pub fn read(path: &Path) -> Value {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).filter(Value::is_object).unwrap_or_else(|| json!({}))
}

pub fn fetch_mode(settings: &Value) -> Option<FetchMode> {
    settings.pointer("/sharedStash/fetchMode")?.as_str().and_then(FetchMode::parse)
}

/// Sets (or, with `None`, removes) `sharedStash.fetchMode`, keeping everything else.
pub fn set_fetch_mode(settings: &mut Value, mode: Option<FetchMode>) {
    if !settings.is_object() {
        *settings = json!({});
    }
    let root = settings.as_object_mut().expect("object");
    let section = root.entry("sharedStash").or_insert_with(|| Value::Object(Map::new()));
    if !section.is_object() {
        *section = Value::Object(Map::new());
    }
    let section = section.as_object_mut().expect("object");
    match mode {
        Some(m) => {
            section.insert("fetchMode".into(), Value::String(m.as_str().into()));
        }
        None => {
            section.remove("fetchMode");
        }
    }
}

/// Atomic write (temp file + rename); creates the directory.
pub fn write(path: &Path, settings: &Value) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_fetches_with_fetch_or_pull_only_while_the_tab_is_open() {
        // Acceptance criterion 2: mode `panel`, tab closed -> a plain Fetch does not download shared stashes.
        assert!(!fetch_with_net_op(FetchMode::Panel, false));
        assert!(fetch_with_net_op(FetchMode::Panel, true));
        // Mode `always`: plain Fetch downloads them, tab open or not.
        assert!(fetch_with_net_op(FetchMode::Always, false));
        assert!(fetch_with_net_op(FetchMode::Always, true));
    }

    #[test]
    fn repo_beats_global_beats_panel() {
        use FetchMode::*;
        assert_eq!(resolve_fetch_mode(None, None), Panel);
        assert_eq!(resolve_fetch_mode(None, Some(Always)), Always);
        assert_eq!(resolve_fetch_mode(Some(Panel), Some(Always)), Panel);
        assert_eq!(resolve_fetch_mode(Some(Always), Some(Panel)), Always);
    }

    #[test]
    fn mode_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&FetchMode::Always).unwrap(), "\"always\"");
        assert_eq!(serde_json::from_str::<FetchMode>("\"panel\"").unwrap(), FetchMode::Panel);
        assert!(serde_json::from_str::<FetchMode>("\"sometimes\"").is_err());
    }

    #[test]
    fn set_get_and_clear_keep_unknown_keys() {
        let mut v = json!({ "theme": "dark", "sharedStash": { "other": 1 } });
        assert_eq!(fetch_mode(&v), None);
        set_fetch_mode(&mut v, Some(FetchMode::Always));
        assert_eq!(fetch_mode(&v), Some(FetchMode::Always));
        assert_eq!(v, json!({ "theme": "dark", "sharedStash": { "other": 1, "fetchMode": "always" } }));
        set_fetch_mode(&mut v, None);
        assert_eq!(v, json!({ "theme": "dark", "sharedStash": { "other": 1 } }));
    }

    #[test]
    fn garbage_values_read_as_unset_and_are_replaced_on_write() {
        assert_eq!(fetch_mode(&json!({ "sharedStash": { "fetchMode": "sometimes" } })), None);
        assert_eq!(fetch_mode(&json!({ "sharedStash": { "fetchMode": 3 } })), None);
        let mut v = json!([1, 2]);
        set_fetch_mode(&mut v, Some(FetchMode::Panel));
        assert_eq!(v, json!({ "sharedStash": { "fetchMode": "panel" } }));
        let mut v = json!({ "sharedStash": "oops" });
        set_fetch_mode(&mut v, Some(FetchMode::Always));
        assert_eq!(fetch_mode(&v), Some(FetchMode::Always));
    }

    #[test]
    fn files_round_trip_and_bad_files_read_empty() {
        let dir = std::env::temp_dir().join(format!("atlasgit-settings-{}", std::process::id()));
        let file = dir.join("nested").join("settings.json");
        assert_eq!(read(&file), json!({}), "missing file");
        let mut v = read(&file);
        set_fetch_mode(&mut v, Some(FetchMode::Always));
        write(&file, &v).unwrap();
        assert_eq!(fetch_mode(&read(&file)), Some(FetchMode::Always));
        std::fs::write(&file, "not json").unwrap();
        assert_eq!(read(&file), json!({}), "corrupt file");
        std::fs::write(&file, "[1]").unwrap();
        assert_eq!(read(&file), json!({}), "not an object");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
