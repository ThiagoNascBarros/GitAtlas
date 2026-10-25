// SPDX-License-Identifier: Apache-2.0
//! Shared Stash commands (docs/specs/shared-stash.md §5, §6). Network operations run on
//! `spawn_blocking` with the askpass bridge, like Fetch/Pull/Push; the tab reports whether it is
//! open so a plain Fetch/Pull knows whether to download shared stashes too (`sharedStash.fetchMode`).

use std::path::PathBuf;

use ak_core::ops::lock::{self, Lock};
use ak_core::ops::{Engine, KeptSharedStash, OpOutcome, Operation, RemoteStatus, SharedStashInfo, SharedStashOutcome};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::settings::{self, FetchMode};
use crate::{askpass, engine, get, AppState, Result, UiError};

/// Emitted (repo id) when shared stashes were fetched or changed, so the tab can reload.
const CHANGED_EVENT: &str = "shared-stash-changed";

fn global_settings_path(app: &AppHandle) -> Option<PathBuf> {
    crate::prefs::global_path(app)
}

fn repo_settings_path(e: &Engine) -> PathBuf {
    crate::prefs::repo_path(e.git_dir())
}

fn modes(app: &AppHandle, e: &Engine) -> (Option<FetchMode>, Option<FetchMode>) {
    let repo = settings::fetch_mode(&settings::read(&repo_settings_path(e)));
    let global = global_settings_path(app).and_then(|p| settings::fetch_mode(&settings::read(&p)));
    (repo, global)
}

fn effective_mode(app: &AppHandle, e: &Engine) -> FetchMode {
    let (repo, global) = modes(app, e);
    settings::resolve_fetch_mode(repo, global)
}

/// Whether a Fetch/Pull of this repository should also fetch shared stashes (spec §5.1).
pub fn wants_fetch_with_net_op(app: &AppHandle, state: &AppState, repo_id: u32, e: &Engine) -> bool {
    let tab_open = crate::diagnostics::lock(&state.shared_tab_open).contains(&repo_id);
    settings::fetch_with_net_op(effective_mode(app, e), tab_open)
}

/// Runs after a successful Fetch/Pull inside the same askpass bridge. Failures are not errors of
/// the Fetch/Pull itself; they are reported per remote in the returned outcome.
pub fn fetch_after_net_op(e: &Engine, env: &[(String, String)], app: &AppHandle, repo_id: u32) -> Option<SharedStashOutcome> {
    let out = e.run_with_env(&Operation::SharedStashFetch { remote: None }, env).ok().and_then(|o| o.shared_stash);
    let _ = app.emit(CHANGED_EVENT, repo_id);
    out
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| UiError::new("GIT_FAILED", e))?
}

/// Runs a Shared Stash operation off the async runtime with credentials prompts routed to the UI.
async fn net(repo_id: u32, op: Operation, app: AppHandle, state: &AppState) -> Result<OpOutcome> {
    let r = get(state, repo_id)?;
    let pending = state.askpass.clone();
    let handle = app.clone();
    let out = blocking(move || {
        let e = engine(&r)?;
        Ok(askpass::with_bridge(&handle, pending, repo_id, |env| e.run_with_env(&op, env))?)
    })
    .await;
    let _ = app.emit(CHANGED_EVENT, repo_id);
    out
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedStashListing {
    stashes: Vec<SharedStashInfo>,
    kept: Vec<KeptSharedStash>,
    remotes: Vec<RemoteStatus>,
    default_remote: Option<String>,
    fetch_mode: FetchMode,
}

/// Reads the local mirrors only; no network.
#[tauri::command]
pub async fn shared_stash_list(repo_id: u32, app: AppHandle, state: State<'_, AppState>) -> Result<SharedStashListing> {
    let r = get(&state, repo_id)?;
    blocking(move || {
        let e = engine(&r)?;
        Ok(SharedStashListing {
            stashes: e.shared_stashes()?,
            kept: e.shared_stash_kept()?,
            remotes: e.shared_stash_remotes(),
            default_remote: e.shared_stash_default_remote(),
            fetch_mode: effective_mode(&app, e),
        })
    })
    .await
}

/// Downloads the shared stashes of `remote`, or of all remotes; one failing remote does not stop the others.
#[tauri::command]
pub async fn shared_stash_fetch(repo_id: u32, remote: Option<String>, app: AppHandle, state: State<'_, AppState>) -> Result<OpOutcome> {
    net(repo_id, Operation::SharedStashFetch { remote }, app, &state).await
}

#[tauri::command]
pub async fn shared_stash_publish(
    repo_id: u32,
    stash_sha: String,
    title: String,
    remote: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<OpOutcome> {
    net(repo_id, Operation::SharedStashPublish { stash_sha, title, remote }, app, &state).await
}

/// New content (`stash_sha`) and/or a new title; the part that is `null` stays as it was.
#[tauri::command]
pub async fn shared_stash_update(
    repo_id: u32,
    id: String,
    remote: String,
    stash_sha: Option<String>,
    title: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<OpOutcome> {
    net(repo_id, Operation::SharedStashUpdate { id, remote, stash_sha, title }, app, &state).await
}

#[tauri::command]
pub async fn shared_stash_unshare(repo_id: u32, id: String, remote: String, app: AppHandle, state: State<'_, AppState>) -> Result<OpOutcome> {
    net(repo_id, Operation::SharedStashUnshare { id, remote }, app, &state).await
}

#[tauri::command]
pub async fn shared_stash_reshare(repo_id: u32, id: String, remote: String, app: AppHandle, state: State<'_, AppState>) -> Result<OpOutcome> {
    net(repo_id, Operation::SharedStashReshare { id, remote }, app, &state).await
}

/// Local only: forgets the kept copy of an unshared stash.
#[tauri::command]
pub async fn shared_stash_discard_keep(repo_id: u32, id: String, app: AppHandle, state: State<'_, AppState>) -> Result<OpOutcome> {
    let r = get(&state, repo_id)?;
    let out = blocking(move || Ok(engine(&r)?.run(&Operation::SharedStashDiscardKeep { id })?)).await;
    let _ = app.emit(CHANGED_EVENT, repo_id);
    out
}

/// Apply may fetch the stash's base branch when the base commit is missing, hence the bridge.
#[tauri::command]
pub async fn shared_stash_apply(repo_id: u32, id: String, remote: String, app: AppHandle, state: State<'_, AppState>) -> Result<OpOutcome> {
    net(repo_id, Operation::SharedStashApply { id, remote }, app, &state).await
}

#[tauri::command]
pub async fn shared_stash_branch(
    repo_id: u32,
    id: String,
    remote: String,
    name: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<OpOutcome> {
    net(repo_id, Operation::SharedStashBranch { id, remote, name }, app, &state).await
}

/// "Retry check": forgets the cached answer of the custom-refs probe (one remote, or all).
#[tauri::command]
pub async fn shared_stash_probe_reset(repo_id: u32, remote: Option<String>, app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let r = get(&state, repo_id)?;
    let out = blocking(move || Ok(engine(&r)?.shared_stash_probe_reset(remote.as_deref())?)).await;
    let _ = app.emit(CHANGED_EVENT, repo_id);
    out
}

/// The tab tells the shell whether it is open (spec §5.1: in `panel` mode a Fetch/Pull while it is open also fetches shared stashes).
#[tauri::command]
pub fn shared_stash_tab_open(repo_id: u32, open: bool, state: State<'_, AppState>) {
    let mut tabs = crate::diagnostics::lock(&state.shared_tab_open);
    if open {
        tabs.insert(repo_id);
    } else {
        tabs.remove(&repo_id);
    }
}

/// Dismisses the credentials prompts a Shared Stash operation of this repository is waiting on, so
/// Git gives up with an authentication error. A running Git process itself is not killed.
#[tauri::command]
pub fn shared_stash_cancel(repo_id: u32, app: AppHandle, state: State<'_, AppState>) {
    for id in state.askpass.cancel_repo(repo_id) {
        let _ = app.emit("credentials-cancel", id);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchModeSettings {
    /// What applies to this repository now.
    effective: FetchMode,
    /// This repository's own choice, if it made one.
    repo: Option<FetchMode>,
    /// The global default, if one was set.
    global: Option<FetchMode>,
}

#[tauri::command]
pub async fn shared_stash_settings_get(repo_id: u32, app: AppHandle, state: State<'_, AppState>) -> Result<FetchModeSettings> {
    let r = get(&state, repo_id)?;
    let (repo, global) = modes(&app, engine(&r)?);
    Ok(FetchModeSettings { effective: settings::resolve_fetch_mode(repo, global), repo, global })
}

/// `scope` is `"repo"` or `"global"`; a `null` mode removes that scope's choice.
#[tauri::command]
pub async fn shared_stash_settings_set(
    repo_id: u32,
    scope: String,
    mode: Option<FetchMode>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<FetchModeSettings> {
    let r = get(&state, repo_id)?;
    let e = engine(&r)?;
    let path = match scope.as_str() {
        "repo" => repo_settings_path(e),
        "global" => global_settings_path(&app).ok_or_else(|| UiError::new("IO_ERROR", "No settings directory is available."))?,
        _ => return Err(UiError::new("GIT_FAILED", "unknown settings scope")),
    };
    // One writer at a time per repository (the same lock the core reserves for repository settings).
    let _g = lock::try_acquire(e.root(), &[Lock::RepoSettings]).map_err(|_| UiError {
        code: "REPO_BUSY",
        message: "Another operation is running on this repository.".into(),
        detail: None,
        suggestion: Some("Wait for it to finish and try again.".into()),
    })?;
    let _global = crate::prefs::lock_global();
    let mut value = settings::read(&path);
    settings::set_fetch_mode(&mut value, mode);
    settings::write(&path, &value).map_err(|err| UiError::new("IO_ERROR", err))?;
    let (repo, global) = modes(&app, e);
    Ok(FetchModeSettings { effective: settings::resolve_fetch_mode(repo, global), repo, global })
}
