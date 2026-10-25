// SPDX-License-Identifier: Apache-2.0
//! Tauri commands for the UI preferences (`theme`, `zoom`, `language`, `animations`). The global
//! scope is `settings.json` in the app config directory; the repository scope is
//! `<git-dir>/gitatlas/settings.json`. The rules live in `settings.rs`.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use ak_core::ops::lock::{self, Lock};
use serde::Serialize;
use serde_json::{Map, Value};
use tauri::{AppHandle, Manager, State};

use crate::settings;
use crate::{engine, get, AppState, Result, UiError};

/// Serializes read-modify-write of the global file (preferences and the shared stash fetch mode).
static GLOBAL_WRITE: Mutex<()> = Mutex::new(());

pub fn lock_global() -> MutexGuard<'static, ()> {
    GLOBAL_WRITE.lock().unwrap_or_else(|p| p.into_inner())
}

pub fn global_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}

pub fn repo_path(git_dir: &Path) -> PathBuf {
    git_dir.join("gitatlas").join("settings.json")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefsView {
    /// What the global scope sets (only its own keys).
    global: Map<String, Value>,
    /// What the repository scope sets, when a repository is given.
    repo: Option<Map<String, Value>>,
    /// Every key with its effective value (repository, else global, else default).
    effective: Value,
}

/// The repository's root and git directory, when `repo_id` names an open repository.
fn repo_dirs(state: &AppState, repo_id: Option<u32>) -> Result<Option<(PathBuf, PathBuf)>> {
    let Some(id) = repo_id else { return Ok(None) };
    let r = get(state, id)?;
    let e = engine(&r)?;
    Ok(Some((e.root().to_path_buf(), e.git_dir().to_path_buf())))
}

fn view(app: &AppHandle, git_dir: Option<&Path>) -> PrefsView {
    let global = global_path(app).map(|p| settings::scope_prefs(&settings::read(&p))).unwrap_or_default();
    let repo = git_dir.map(|d| settings::scope_prefs(&settings::read(&repo_path(d))));
    let empty = Map::new();
    let effective = settings::resolve_prefs(repo.as_ref().unwrap_or(&empty), &global);
    PrefsView { global, repo, effective }
}

#[tauri::command]
pub async fn prefs_get(repo_id: Option<u32>, app: AppHandle, state: State<'_, AppState>) -> Result<PrefsView> {
    let dirs = repo_dirs(&state, repo_id)?;
    Ok(view(&app, dirs.as_ref().map(|(_, g)| g.as_path())))
}

/// `scope` is `"global"` or `"repo"`. A `null` value removes that key from the scope.
#[tauri::command]
pub async fn prefs_set(
    repo_id: Option<u32>,
    scope: String,
    patch: Map<String, Value>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PrefsView> {
    let dirs = repo_dirs(&state, repo_id)?;
    let path = match (scope.as_str(), &dirs) {
        ("global", _) => global_path(&app).ok_or_else(|| UiError::new("IO_ERROR", "No settings directory is available."))?,
        ("repo", Some((_, git_dir))) => repo_path(git_dir),
        ("repo", None) => return Err(UiError::new("INVALID_SETTING", "No repository is open for this setting.")),
        _ => return Err(UiError::new("INVALID_SETTING", "Unknown settings scope.")),
    };
    let _global = lock_global();
    // One writer at a time per repository, the same lock the shared stash settings use.
    let _repo = match (&dirs, scope.as_str()) {
        (Some((root, _)), "repo") => Some(lock::try_acquire(root, &[Lock::RepoSettings]).map_err(|_| UiError {
            code: "REPO_BUSY",
            message: "Another operation is running on this repository.".into(),
            detail: None,
            suggestion: Some("Wait for it to finish and try again.".into()),
        })?),
        _ => None,
    };
    let mut value = settings::read(&path);
    settings::apply_prefs_patch(&mut value, &patch).map_err(|m| UiError::new("INVALID_SETTING", m))?;
    settings::write(&path, &value).map_err(|err| UiError::new("IO_ERROR", err))?;
    Ok(view(&app, dirs.as_ref().map(|(_, g)| g.as_path())))
}
