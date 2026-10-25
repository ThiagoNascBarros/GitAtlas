// SPDX-License-Identifier: Apache-2.0
//! Shell settings kept as JSON: a global file in the app config directory and an optional
//! per-repository override in `<git-dir>/gitatlas/settings.json`. Both are written only when the
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

/// UI preferences (`theme`, `zoom`, `language`, `animations`). A scope (the global file or a
/// repository file) holds only the keys it sets. The effective value of a key is the repository's,
/// else the global one, else the default (`resolve_prefs`).
///
/// Reserved for other features: the UI does not read these yet, they are never validated here, and
/// every write keeps them as they are. Shapes for the agent that implements them:
/// - `gitFlow`: `{ "enabled": bool, "mainBranch": str, "developBranch": str,
///   "prefixes": { "feature": str, "release": str, "fix": str, "hotfix": str } }`
/// - `branchNaming`: `{ "template": str, "slug": { "lowercase": bool, "separator": str } }`
pub const PREF_KEYS: [&str; 4] = ["theme", "zoom", "language", "animations"];

/// Zoom bounds (80 % to 150 %, the range the layout supports).
pub const ZOOM_MIN: f64 = 0.8;
pub const ZOOM_MAX: f64 = 1.5;

/// Checks one preference value and returns it as it is stored.
pub fn validate_pref(key: &str, value: &Value) -> Result<Value, String> {
    let allowed: &[&str] = match key {
        "theme" => &["system", "atlas-dark", "atlas-light"],
        "language" => &["system", "en", "pt-BR"],
        "animations" => &["on", "reduced"],
        "zoom" => {
            return value
                .as_f64()
                .filter(|z| (ZOOM_MIN..=ZOOM_MAX).contains(z))
                .map(|z| json!(z))
                .ok_or_else(|| format!("zoom must be a number from {ZOOM_MIN} to {ZOOM_MAX}"));
        }
        _ => return Err(format!("unknown setting: {key}")),
    };
    match value.as_str() {
        Some(s) if allowed.contains(&s) => Ok(value.clone()),
        _ => Err(format!("{key} must be one of: {}", allowed.join(", "))),
    }
}

/// Default for a preference key when no scope sets it.
fn default_pref(key: &str) -> Value {
    match key {
        "theme" | "language" => json!("system"),
        "zoom" => json!(1.0),
        _ => json!("on"),
    }
}

/// The preferences one scope sets. Invalid values read as unset.
pub fn scope_prefs(settings: &Value) -> Map<String, Value> {
    PREF_KEYS.iter().filter_map(|k| validate_pref(k, settings.get(*k)?).ok().map(|v| ((*k).to_string(), v))).collect()
}

/// Effective preferences: the repository's value, else the global one, else the default.
pub fn resolve_prefs(repo: &Map<String, Value>, global: &Map<String, Value>) -> Value {
    let pick = |k: &str| repo.get(k).or_else(|| global.get(k)).cloned().unwrap_or_else(|| default_pref(k));
    Value::Object(PREF_KEYS.iter().map(|k| ((*k).to_string(), pick(k))).collect())
}

/// Applies a patch to one scope. A `null` removes that key. Every entry is checked first, so a
/// bad entry changes nothing. Keys outside `PREF_KEYS` are refused here (reserved sections are not
/// reachable through this path).
pub fn apply_prefs_patch(settings: &mut Value, patch: &Map<String, Value>) -> Result<(), String> {
    let mut edits = Vec::with_capacity(patch.len());
    for (key, value) in patch {
        if !PREF_KEYS.contains(&key.as_str()) {
            return Err(format!("unknown setting: {key}"));
        }
        let stored = if value.is_null() { None } else { Some(validate_pref(key, value)?) };
        edits.push((key.as_str(), stored));
    }
    if !settings.is_object() {
        *settings = json!({});
    }
    if let Some(root) = settings.as_object_mut() {
        for (key, stored) in edits {
            match stored {
                Some(v) => {
                    root.insert(key.to_string(), v);
                }
                None => {
                    root.remove(key);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod prefs_tests {
    use super::*;

    #[test]
    fn validates_each_key() {
        assert_eq!(validate_pref("theme", &json!("atlas-light")).unwrap(), json!("atlas-light"));
        assert!(validate_pref("theme", &json!("neon")).is_err());
        assert_eq!(validate_pref("language", &json!("pt-BR")).unwrap(), json!("pt-BR"));
        assert!(validate_pref("language", &json!("pt")).is_err());
        assert_eq!(validate_pref("animations", &json!("reduced")).unwrap(), json!("reduced"));
        assert!(validate_pref("animations", &json!(true)).is_err());
        assert!(validate_pref("nope", &json!("x")).is_err());
    }

    #[test]
    fn zoom_is_a_number_within_the_prd_bounds() {
        assert_eq!(validate_pref("zoom", &json!(1.25)).unwrap(), json!(1.25));
        assert_eq!(validate_pref("zoom", &json!(0.8)).unwrap(), json!(0.8));
        assert_eq!(validate_pref("zoom", &json!(1)).unwrap(), json!(1.0));
        assert!(validate_pref("zoom", &json!(0.79)).is_err());
        assert!(validate_pref("zoom", &json!(1.51)).is_err());
        assert!(validate_pref("zoom", &json!("1.0")).is_err());
    }

    #[test]
    fn repository_beats_global_beats_default() {
        let repo = scope_prefs(&json!({ "theme": "atlas-light" }));
        let global = scope_prefs(&json!({ "theme": "atlas-dark", "zoom": 1.5 }));
        assert_eq!(resolve_prefs(&repo, &global), json!({ "theme": "atlas-light", "zoom": 1.5, "language": "system", "animations": "on" }));
        assert_eq!(
            resolve_prefs(&Map::new(), &Map::new()),
            json!({ "theme": "system", "zoom": 1.0, "language": "system", "animations": "on" })
        );
    }

    #[test]
    fn invalid_stored_values_read_as_unset() {
        let scope = scope_prefs(&json!({ "theme": "neon", "zoom": 9, "language": "en", "other": 1 }));
        assert_eq!(scope, json!({ "language": "en" }).as_object().unwrap().clone());
    }

    #[test]
    fn patch_sets_and_null_removes_keeping_other_keys() {
        let mut v = json!({ "sharedStash": { "fetchMode": "always" }, "theme": "atlas-dark" });
        let patch = json!({ "zoom": 1.25, "theme": null });
        apply_prefs_patch(&mut v, patch.as_object().unwrap()).unwrap();
        assert_eq!(v, json!({ "sharedStash": { "fetchMode": "always" }, "zoom": 1.25 }));
    }

    #[test]
    fn a_bad_entry_changes_nothing() {
        let mut v = json!({ "theme": "atlas-dark" });
        let before = v.clone();
        let patch = json!({ "zoom": 1.5, "theme": "neon" });
        assert!(apply_prefs_patch(&mut v, patch.as_object().unwrap()).is_err());
        assert_eq!(v, before);
        let patch = json!({ "gitFlow": { "enabled": true } });
        assert!(apply_prefs_patch(&mut v, patch.as_object().unwrap()).is_err());
        assert_eq!(v, before);
    }

    #[test]
    fn reserved_sections_survive_every_write() {
        let reserved = json!({
            "gitFlow": { "enabled": true, "mainBranch": "main", "developBranch": "develop",
                         "prefixes": { "feature": "feature/", "release": "release/", "fix": "fix/", "hotfix": "hotfix/" } },
            "branchNaming": { "template": "{prefix}{title}", "slug": { "lowercase": true, "separator": "-" } }
        });
        let mut v = reserved.clone();
        apply_prefs_patch(&mut v, json!({ "animations": "reduced" }).as_object().unwrap()).unwrap();
        apply_prefs_patch(&mut v, json!({ "animations": null }).as_object().unwrap()).unwrap();
        assert_eq!(v["gitFlow"], reserved["gitFlow"]);
        assert_eq!(v["branchNaming"], reserved["branchNaming"]);
    }

    #[test]
    fn non_object_roots_are_replaced_by_a_patch() {
        let mut v = json!([1, 2]);
        apply_prefs_patch(&mut v, json!({ "language": "pt-BR" }).as_object().unwrap()).unwrap();
        assert_eq!(v, json!({ "language": "pt-BR" }));
    }
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
        let dir = std::env::temp_dir().join(format!("gitatlas-settings-{}", std::process::id()));
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
