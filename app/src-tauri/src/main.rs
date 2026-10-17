// SPDX-License-Identifier: Apache-2.0
//! AtlasGit Tauri shell. The UI can only reach the commands below
//! (SDD §4.1). All Git access stays in the core.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod askpass;
mod diagnostics;
mod prefs;
mod session;
mod settings;
mod shared_stash;
mod watch_slots;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ak_core::cache;
use ak_core::graph::{self, Layout, Topology};
use ak_core::ops::journal::JournalEntry;
use ak_core::ops::{ConflictVersions, MergePreview, StashInfo, TodoItem};
use ak_core::ops::{Engine, OpError, OpOutcome, Operation};
use ak_core::read::{CliReader, CommitDetails, RepoReader, RowText};
use ak_core::read_gix::GixReader;
use ak_gitcli::parse::{RefKind, StatusReport};
use ak_gitcli::{Git, Mode};
use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager, State};

/// What stays in memory per repository: topology and layout only. Commit text
/// (summary, author) is fetched from Git for the rows on screen.
struct Repo {
    reader: CliReader,
    engine: Option<Engine>,
    shas: Vec<String>,
    topo: Topology,
    layout: Layout,
}

#[derive(Default)]
struct AppState {
    repos: Mutex<HashMap<u32, Arc<Repo>>>,
    /// One file watcher per open repository, kept across refreshes (PERF-01).
    watchers: Mutex<watch_slots::WatcherSlots<SharedWatcher>>,
    askpass: Arc<askpass::Pending>,
    /// Repositories whose "Shared stash" tab is open (reported by the UI).
    shared_tab_open: Mutex<HashSet<u32>>,
    next_id: Mutex<u32>,
    initial_path: Option<PathBuf>,
}

/// Errors cross the IPC boundary as `{ code, message, detail?, suggestion? }` (SDD §4.4).
#[derive(Debug, Serialize)]
struct UiError {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
}

impl UiError {
    fn new(code: &'static str, e: impl ToString) -> Self {
        Self { code, message: e.to_string(), detail: None, suggestion: None }
    }
}

impl From<OpError> for UiError {
    fn from(e: OpError) -> Self {
        Self { code: e.code, message: e.message, detail: e.detail, suggestion: e.suggestion }
    }
}

type Result<T> = std::result::Result<T, UiError>;

/// Runs blocking git work off the async runtime (PERF-05): a slow read must not hold a runtime thread.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| UiError::new("READ_FAILED", e))?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RefInfo {
    name: String,
    kind: RefKind,
    row: Option<u32>,
    is_head: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenedRepo {
    repo_id: u32,
    root: String,
    name: String,
    head: Option<String>,
    branch: Option<String>,
    row_count: u32,
    lane_count: u16,
    refs: Vec<RefInfo>,
    git_version: String,
    read_only_reason: Option<String>,
}

/// Returns freed heap pages to the OS after the transient full commit list is dropped.
fn trim_memory() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    unsafe {
        extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        malloc_trim(0);
    }
}

/// Lines that matter to the working-tree view: ignores Git's own bookkeeping.
fn relevant(path: &Path, root: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else { return true };
    let mut parts = rel.components().map(|c| c.as_os_str().to_string_lossy());
    if parts.next().as_deref() != Some(".git") {
        return true;
    }
    matches!(parts.next().as_deref(), Some("index") | Some("HEAD") | Some("refs") | Some("MERGE_HEAD"))
}

type SharedWatcher = Arc<Mutex<notify::RecommendedWatcher>>;

enum WatchMsg {
    Changed,
    NewDir(PathBuf),
}

/// Watches the working tree and tells the UI (debounced) when it changed; emits `watch-failed` (repo id) when it cannot be set up.
fn start_watcher(app: AppHandle, repo_id: u32, root: &Path) -> Option<SharedWatcher> {
    let w = setup_watcher(app.clone(), repo_id, root);
    if w.is_none() {
        let _ = app.emit("watch-failed", repo_id);
    }
    w
}

/// Directory key used by the watcher: path relative to the repository root, with a trailing `/`.
fn dir_key(root: &Path, dir: &Path) -> String {
    let rel = dir.strip_prefix(root).unwrap_or(dir);
    format!("{}/", rel.to_string_lossy().replace('\\', "/"))
}

fn is_ignored_dir(root: &Path, key: &str) -> bool {
    std::process::Command::new("git")
        .current_dir(root)
        .args(["check-ignore", "-q", "--", key])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Ignored directories at any depth (`packages/a/node_modules/`), from one git call (PERF-07).
fn ignored_dirs(root: &Path) -> HashSet<String> {
    std::process::Command::new("git")
        .current_dir(root)
        .args(["ls-files", "--others", "--ignored", "--exclude-standard", "--directory", "-z"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).split('\0').filter(|d| d.ends_with('/')).map(str::to_owned).collect())
        .unwrap_or_default()
}

/// `dir` and the directories below it that git does not ignore (symlinks are not followed).
fn collect_watch_dirs(root: &Path, dir: &Path, ignored: &HashSet<String>, out: &mut Vec<PathBuf>) {
    out.push(dir.to_path_buf());
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let path = e.path();
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) && !ignored.contains(&dir_key(root, &path)) {
            collect_watch_dirs(root, &path, ignored, out);
        }
    }
}

/// Watches the directories from [`collect_watch_dirs`], one non-recursive watch each: ignored trees
/// such as `packages/*/node_modules` cost no inotify watches.
fn watch_tree(g: &mut notify::RecommendedWatcher, root: &Path, ignored: &HashSet<String>) -> Option<()> {
    let mut dirs = Vec::new();
    collect_watch_dirs(root, root, ignored, &mut dirs);
    for d in &dirs {
        g.watch(d, RecursiveMode::NonRecursive).ok()?;
    }
    Some(())
}

/// A directory created while watching: watched unless git ignores it, and so are the folders in it.
fn adopt_new_dir(w: &SharedWatcher, root: &Path, dir: &Path) {
    // `.git` is bookkeeping (see `relevant`): never adopted, so its churn does not refresh the view.
    if dir.file_name().is_some_and(|n| n == ".git") || is_ignored_dir(root, &dir_key(root, dir)) {
        return;
    }
    if diagnostics::lock(w).watch(dir, RecursiveMode::NonRecursive).is_err() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                adopt_new_dir(w, root, &e.path());
            }
        }
    }
}

fn setup_watcher(app: AppHandle, repo_id: u32, root: &Path) -> Option<SharedWatcher> {
    let (tx, rx) = std::sync::mpsc::channel::<WatchMsg>();
    let base = root.to_path_buf();
    let w = notify::recommended_watcher(move |ev: notify::Result<notify::Event>| {
        if let Ok(ev) = ev {
            // A new directory (at any depth, outside git's bookkeeping): watch it later
            // (not here: adding a watch from inside the event callback can deadlock the backend).
            if matches!(ev.kind, notify::EventKind::Create(_)) {
                for p in ev.paths.iter().filter(|p| relevant(p, &base) && p.is_dir()) {
                    let _ = tx.send(WatchMsg::NewDir(p.clone()));
                }
            }
            if ev.paths.iter().any(|p| relevant(p, &base)) {
                let _ = tx.send(WatchMsg::Changed);
            }
        }
    })
    .ok()?;
    let w = Arc::new(Mutex::new(w));
    // Every directory git does not ignore, at any depth (node_modules and target trees are skipped).
    let ignored = ignored_dirs(root);
    {
        let mut g = diagnostics::lock(&w);
        watch_tree(&mut g, root, &ignored)?;
    }
    let weak = Arc::downgrade(&w);
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        let handle = |m: WatchMsg| {
            if let WatchMsg::NewDir(p) = m {
                if let Some(w) = weak.upgrade() {
                    adopt_new_dir(&w, &root, &p);
                }
            }
        };
        while let Ok(m) = rx.recv() {
            handle(m);
            // Collapse bursts (checkout, build output) into one refresh.
            while let Ok(m) = rx.recv_timeout(Duration::from_millis(350)) {
                handle(m);
            }
            let _ = app.emit("repo-changed", repo_id);
        }
    });
    Some(w)
}

fn load(path: &Path) -> Result<(Repo, OpenedRepo)> {
    let git = Git::discover(Mode::ReadWrite).map_err(|e| UiError::new("GIT_NOT_FOUND", e))?;
    let caps = git.caps().clone();
    let reader = CliReader::open(git.clone(), path).map_err(|e| UiError::new("NOT_A_REPO", e))?;
    let refs = reader.refs().map_err(|e| UiError::new("READ_FAILED", e))?;
    let head = reader.head().map_err(|e| UiError::new("READ_FAILED", e))?;
    // Commit list: reuse the on-disk cache while no ref moved (fingerprint match).
    // `.git` may be a file (worktrees, submodules): ask Git where the directory really is.
    let git_dir = git
        .run(reader.root(), ["rev-parse", "--absolute-git-dir"])
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o).trim()))
        .unwrap_or_else(|_| reader.root().join(".git"));
    let fp = cache::fingerprint(
        &refs.iter().map(|r| (r.name.clone(), r.target.clone())).collect::<Vec<_>>(),
        head.as_deref(),
    );
    let cache_file = cache::cache_path(&git_dir);
    let commits = match cache_file.as_deref().and_then(|p| cache::load(p, fp)) {
        Some(c) => c,
        None => {
            // History via gitoxide (same output as the CLI); fall back to the CLI on error.
            let fresh = GixReader::open(git.clone(), reader.root())
                .ok()
                .and_then(|g| g.commits().ok())
                .map(Ok)
                .unwrap_or_else(|| reader.commits())
                .map_err(|e| UiError::new("READ_FAILED", e))?;
            if let Some(p) = cache_file.as_deref() {
                cache::save(p, fp, &fresh);
            }
            fresh
        }
    };
    let status = reader.status().map_err(|e| UiError::new("READ_FAILED", e))?;
    let topo = Topology::from_commits(commits.iter().map(|c| (c.sha.as_str(), c.parents.as_slice())));
    let layout = graph::layout(&topo);

    let row_of: HashMap<&str, u32> = commits.iter().enumerate().map(|(i, c)| (c.sha.as_str(), i as u32)).collect();
    let root = reader.root().to_path_buf();
    let engine = Engine::open(git, &root).ok();
    // Each stash is shown as a label on the commit it was taken on (`refs/stash` alone only names the newest).
    let stashes = engine.as_ref().and_then(|e| e.stashes().ok()).unwrap_or_default();
    let refs = refs
        .into_iter()
        .filter(|r| r.kind != RefKind::Stash)
        .map(|r| RefInfo { row: row_of.get(r.target.as_str()).copied(), name: r.short, kind: r.kind, is_head: r.is_head })
        .chain(stashes.iter().map(|s| RefInfo {
            row: row_of.get(s.base.as_str()).copied(),
            name: format!("stash@{{{}}}: {}", s.index, s.message),
            kind: RefKind::Stash,
            is_head: false,
        }))
        .collect();
    let read_only_reason = if !caps.supported() {
        Some("Git older than 2.20: read-only".to_owned())
    } else {
        None
    };
    let opened = OpenedRepo {
        repo_id: 0,
        name: root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        root: root.to_string_lossy().into_owned(),
        head,
        branch: status.branch,
        row_count: commits.len() as u32,
        lane_count: layout.lane_count,
        refs,
        git_version: format!("{}.{}.{}", caps.version.major, caps.version.minor, caps.version.patch),
        read_only_reason,
    };
    let shas: Vec<String> = commits.iter().map(|c| c.sha.clone()).collect();
    drop(commits);
    trim_memory();
    Ok((Repo { reader, engine, shas, topo, layout }, opened))
}

#[tauri::command]
fn get_initial_path(state: State<'_, AppState>) -> Option<String> {
    state.initial_path.as_ref().map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
async fn repo_open(path: String, app: AppHandle, state: State<'_, AppState>) -> Result<OpenedRepo> {
    let (repo, opened) = blocking(move || load(&PathBuf::from(&path)))
        .await
        .inspect_err(|e| tracing::warn!(code = e.code, "falha ao abrir repositório"))?;
    let repo_id = {
        let mut id = diagnostics::lock(&state.next_id);
        *id += 1;
        *id
    };
    let root = repo.reader.root().to_path_buf();
    let watch_app = app.clone();
    let watcher = blocking(move || Ok(start_watcher(watch_app, repo_id, &root))).await?;
    diagnostics::lock(&state.watchers).ensure(repo_id, move || watcher);
    diagnostics::lock(&state.repos).insert(repo_id, Arc::new(repo));
    let opened = OpenedRepo { repo_id, ..opened };
    // Recents are a convenience: a failure to save them never fails the open.
    let _ = update_session(&app, |s| session::touch_recent(&mut s.recents, &opened.root, &opened.name, now_secs()));
    Ok(opened)
}

/// Serializes read-modify-write of `session.json`.
static SESSION_WRITE: Mutex<()> = Mutex::new(());

fn session_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("session.json"))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Loads `session.json`, applies `change` and writes it back.
fn update_session(app: &AppHandle, change: impl FnOnce(&mut session::Session)) -> std::io::Result<()> {
    let _g = SESSION_WRITE.lock().unwrap_or_else(|p| p.into_inner());
    let path = session_path(app).ok_or_else(|| std::io::Error::other("no config directory"))?;
    let mut s = session::read(&path);
    change(&mut s);
    session::write(&path, &s)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentView {
    #[serde(flatten)]
    recent: session::Recent,
    /// Whether the folder is still there (a missing one is listed, but cannot be opened).
    exists: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionView {
    /// Tabs whose folder still exists, in order.
    tabs: Vec<String>,
    /// Index of the tab to show first, within `tabs`.
    active: usize,
    recents: Vec<RecentView>,
}

/// What the UI needs at startup and after changes: the tabs to reopen and the recent repositories.
#[tauri::command]
fn session_get(app: AppHandle) -> SessionView {
    let s = session_path(&app).map(|p| session::read(&p)).unwrap_or_default();
    let exists = |p: &str| Path::new(p).is_dir();
    let (tabs, active) = session::restorable(&s, exists);
    let recents = s.recents.into_iter().map(|recent| RecentView { exists: exists(recent.path.as_str()), recent }).collect();
    SessionView { tabs, active, recents }
}

/// The open tabs (repository roots, in order) and the active index, after each change.
#[tauri::command]
fn session_save_tabs(tabs: Vec<String>, active: usize, app: AppHandle) -> Result<()> {
    update_session(&app, |s| {
        s.tabs = tabs;
        s.active = active;
    })
    .map_err(|e| UiError::new("IO_ERROR", e))
}

#[tauri::command]
fn recents_remove(path: String, app: AppHandle) -> Result<()> {
    update_session(&app, |s| session::remove_recent(&mut s.recents, &path)).map_err(|e| UiError::new("IO_ERROR", e))
}

/// Re-reads history and refs after the repository changed (commit, undo, external edits).
/// The file watcher is kept: the root does not change, so it is not started again (PERF-01).
#[tauri::command]
async fn repo_refresh(repo_id: u32, app: AppHandle, state: State<'_, AppState>) -> Result<OpenedRepo> {
    let root = get(&state, repo_id)?.reader.root().to_path_buf();
    let watch_root = root.clone();
    let (repo, opened) = blocking(move || load(&root)).await?;
    // Start the watcher only if this repository has none yet (the slot survives refreshes).
    if !diagnostics::lock(&state.watchers).contains(repo_id) {
        let watcher = blocking(move || Ok(start_watcher(app, repo_id, &watch_root))).await?;
        diagnostics::lock(&state.watchers).ensure(repo_id, move || watcher);
    }
    diagnostics::lock(&state.repos).insert(repo_id, Arc::new(repo));
    Ok(OpenedRepo { repo_id, ..opened })
}

#[tauri::command]
fn repo_close(repo_id: u32, state: State<'_, AppState>) {
    diagnostics::lock(&state.repos).remove(&repo_id);
    diagnostics::lock(&state.watchers).remove(repo_id);
    diagnostics::lock(&state.shared_tab_open).remove(&repo_id);
}

fn get(state: &AppState, repo_id: u32) -> Result<Arc<Repo>> {
    diagnostics::lock(&state.repos).get(&repo_id).cloned().ok_or_else(|| UiError::new("UNKNOWN_REPO", repo_id))
}

fn engine(r: &Repo) -> Result<&Engine> {
    r.engine.as_ref().ok_or_else(|| UiError::new("READ_ONLY", "This repository is open in read-only mode."))
}

/// Binary RowWindow (SDD §4.3).
#[tauri::command]
fn graph_window(repo_id: u32, start: u32, count: u32, state: State<'_, AppState>) -> Result<Response> {
    let r = get(&state, repo_id)?;
    Ok(Response::new(graph::encode_window(&r.topo, &r.layout, start, count.min(2000))))
}

#[tauri::command]
async fn graph_rows(repo_id: u32, start: u32, count: u32, state: State<'_, AppState>) -> Result<Vec<RowText>> {
    let r = get(&state, repo_id)?;
    let s = (start as usize).min(r.shas.len());
    let e = (s + count.min(2000) as usize).min(r.shas.len());
    let wanted: Vec<String> = r.shas[s..e].to_vec();
    blocking(move || {
        let wanted: Vec<&str> = wanted.iter().map(String::as_str).collect();
        r.reader.rows_text(&wanted).map_err(|e| UiError::new("READ_FAILED", e))
    })
    .await
}

#[tauri::command]
async fn commit_details(repo_id: u32, sha: String, state: State<'_, AppState>) -> Result<CommitDetails> {
    let r = get(&state, repo_id)?;
    blocking(move || r.reader.commit_details(&sha).map_err(|e| UiError::new("READ_FAILED", e))).await
}

#[tauri::command]
async fn diff_file(repo_id: u32, sha: String, path: String, state: State<'_, AppState>) -> Result<String> {
    let r = get(&state, repo_id)?;
    blocking(move || r.reader.file_diff(&sha, &path).map_err(|e| UiError::new("READ_FAILED", e))).await
}

#[tauri::command]
async fn diff_worktree(
    repo_id: u32,
    path: String,
    staged: bool,
    untracked: bool,
    state: State<'_, AppState>,
) -> Result<String> {
    let r = get(&state, repo_id)?;
    blocking(move || r.reader.worktree_diff(&path, staged, untracked).map_err(|e| UiError::new("READ_FAILED", e))).await
}

#[tauri::command]
async fn status_get(repo_id: u32, state: State<'_, AppState>) -> Result<StatusWithMerge> {
    let r = get(&state, repo_id)?;
    blocking(move || {
        let status = r.reader.status().map_err(|e| UiError::new("READ_FAILED", e))?;
        let in_progress = engine(&r)?.in_progress();
        Ok(StatusWithMerge { status, merging: in_progress.is_some(), in_progress })
    })
    .await
}

#[derive(Serialize)]
struct StatusWithMerge {
    #[serde(flatten)]
    status: StatusReport,
    merging: bool,
    /// "merge", "cherry-pick", "revert" or "rebase" while one is stopped on conflicts.
    in_progress: Option<&'static str>,
}

#[tauri::command]
async fn merge_preview(repo_id: u32, source: String, state: State<'_, AppState>) -> Result<MergePreview> {
    let r = get(&state, repo_id)?;
    blocking(move || Ok(engine(&r)?.merge_preview(&source)?)).await
}

#[tauri::command]
async fn stash_list(repo_id: u32, state: State<'_, AppState>) -> Result<Vec<StashInfo>> {
    let r = get(&state, repo_id)?;
    blocking(move || Ok(engine(&r)?.stashes()?)).await
}

#[tauri::command]
async fn op_merge(repo_id: u32, source: String, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Merge { source }).await
}

#[tauri::command]
async fn op_merge_abort(repo_id: u32, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::MergeAbort).await
}

#[tauri::command]
async fn conflict_versions(repo_id: u32, path: String, state: State<'_, AppState>) -> Result<ConflictVersions> {
    let r = get(&state, repo_id)?;
    blocking(move || Ok(engine(&r)?.conflict_versions(&path)?)).await
}

#[tauri::command]
async fn op_resolve_with(repo_id: u32, path: String, content: String, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::ResolveWith { path, content }).await
}

/// Opens a file of the repository in an external tool, without waiting for it to close.
/// `ATLASGIT_EDITOR` (if set) is run with the file as its argument; otherwise the desktop
/// default application is used (`xdg-open`). Terminal editors need `ATLASGIT_EDITOR` to be a
/// wrapper that opens a terminal, since the app has no terminal of its own.
#[tauri::command]
async fn open_external(repo_id: u32, path: String, state: State<'_, AppState>) -> Result<()> {
    let r = get(&state, repo_id)?;
    let root = engine(&r)?.root().to_path_buf();
    let rel = Path::new(&path);
    if path.is_empty() || !rel.components().all(|c| matches!(c, std::path::Component::Normal(_))) {
        return Err(UiError::new("INVALID_PATH", "That path is not inside the repository."));
    }
    let program = match std::env::var("ATLASGIT_EDITOR") {
        Ok(cmd) if !cmd.trim().is_empty() => cmd,
        _ => "xdg-open".to_owned(),
    };
    let mut child = std::process::Command::new(&program)
        .arg(root.join(rel))
        .current_dir(&root)
        .spawn()
        .map_err(|e| UiError::new("EXTERNAL_TOOL_FAILED", format!("Could not start {program}: {e}")))?;
    // Reap the child in the background so it does not linger as a zombie.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[tauri::command]
async fn op_resolve(repo_id: u32, path: String, theirs: bool, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::ResolveConflict { path, theirs }).await
}

#[tauri::command]
async fn op_stash(repo_id: u32, action: String, index: usize, message: Option<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    let op = match action.as_str() {
        "push" => Operation::StashPush { message, include_untracked: true },
        "apply" => Operation::StashApply { index },
        "pop" => Operation::StashPop { index },
        _ => Operation::StashDrop { index },
    };
    run_op(&state, repo_id, op).await
}

async fn run_op(state: &AppState, repo_id: u32, op: Operation) -> Result<OpOutcome> {
    let r = get(state, repo_id)?;
    blocking(move || Ok(engine(&r)?.run(&op)?)).await
}

#[tauri::command]
async fn op_stage(repo_id: u32, paths: Vec<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Stage { paths }).await
}

#[tauri::command]
async fn op_unstage(repo_id: u32, paths: Vec<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Unstage { paths }).await
}

#[tauri::command]
async fn op_stage_hunk(
    repo_id: u32,
    path: String,
    hunk: usize,
    unstage: bool,
    state: State<'_, AppState>,
) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::StageHunk { path, hunk, unstage }).await
}

#[tauri::command]
async fn op_discard(repo_id: u32, paths: Vec<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Discard { paths }).await
}

#[tauri::command]
async fn op_commit(repo_id: u32, message: String, amend: bool, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Commit { message, amend }).await
}

#[tauri::command]
async fn op_branch_create(repo_id: u32, name: String, at: Option<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::BranchCreate { name, at }).await
}

#[tauri::command]
async fn op_branch_create_from(repo_id: u32, name: String, base: Option<String>, checkout: bool, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::BranchCreateFrom { name, base, checkout }).await
}

/// Name and base of a branch the "New branch" dialog is about to create (pure; no repository needed).
#[tauri::command]
fn branch_plan(kind: String, template: String, ticket: String, title: String, main: String, develop: String) -> Result<ak_core::ops::flow::BranchPlan> {
    let kind = ak_core::ops::flow::BranchKind::parse(&kind).ok_or_else(|| UiError::new("INVALID_ARGUMENT", "Unknown branch type."))?;
    Ok(ak_core::ops::flow::plan(kind, &template, &ticket, &title, &main, &develop))
}

#[tauri::command]
async fn op_branch_rename(repo_id: u32, old: String, new: String, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::BranchRename { old, new }).await
}

#[tauri::command]
async fn op_branch_delete(repo_id: u32, name: String, force: bool, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::BranchDelete { name, force }).await
}

#[tauri::command]
async fn op_checkout(repo_id: u32, target: String, is_branch: bool, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Checkout { target, is_branch }).await
}

/// Fetch, pull or push. Runs off the async runtime (it can wait on the network and on the user
/// answering a credentials prompt, which arrives through `credentials_answer`).
#[tauri::command]
async fn op_net(repo_id: u32, action: String, force: bool, name: Option<String>, app: AppHandle, state: State<'_, AppState>) -> Result<OpOutcome> {
    let r = get(&state, repo_id)?;
    let pending = state.askpass.clone();
    let op = match action.as_str() {
        "fetch" => Operation::Fetch,
        "pull" => Operation::Pull,
        "push" => Operation::Push { force },
        "push-tag" => Operation::PushTag { name: name.unwrap_or_default() },
        _ => return Err(UiError::new("GIT_FAILED", "unknown network action")),
    };
    // Fetch/Pull also download shared stashes when `sharedStash.fetchMode` is `always`, or the tab is open.
    let shared = matches!(action.as_str(), "fetch" | "pull")
        && engine(&r).map(|e| shared_stash::wants_fetch_with_net_op(&app, &state, repo_id, e)).unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || {
        let e = engine(&r)?;
        Ok(askpass::with_bridge(&app, pending, repo_id, |env| {
            let mut out = e.run_with_env(&op, env)?;
            if shared && !out.conflicted {
                out.shared_stash = shared_stash::fetch_after_net_op(e, env, &app, repo_id);
            }
            Ok::<OpOutcome, OpError>(out)
        })?)
    })
    .await
    .map_err(|e| UiError::new("GIT_FAILED", e))?
}

#[tauri::command]
fn credentials_answer(id: u64, value: Option<String>, state: State<'_, AppState>) {
    state.askpass.answer(id, value);
}

#[tauri::command]
async fn op_history(repo_id: u32, action: String, sha: String, mode: Option<String>, name: Option<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    let op = match action.as_str() {
        "cherry-pick" => Operation::CherryPick { sha },
        "revert" => Operation::Revert { sha },
        "reset" => Operation::Reset { sha, mode: mode.unwrap_or_default() },
        "tag" => Operation::TagCreate { name: name.unwrap_or_default(), at: sha, message: mode },
        "tag-delete" => Operation::TagDelete { name: name.unwrap_or_default() },
        _ => return Err(UiError::new("GIT_FAILED", "unknown history action")),
    };
    run_op(&state, repo_id, op).await
}

#[tauri::command]
async fn rebase_todo(repo_id: u32, onto: String, state: State<'_, AppState>) -> Result<Vec<TodoItem>> {
    let r = get(&state, repo_id)?;
    blocking(move || Ok(engine(&r)?.rebase_todo(&onto)?)).await
}

#[tauri::command]
async fn op_rebase(repo_id: u32, onto: String, todo: Option<Vec<TodoItem>>, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Rebase { onto, todo }).await
}

#[tauri::command]
async fn op_rebase_continue(repo_id: u32, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::RebaseContinue).await
}

#[tauri::command]
async fn op_undo(repo_id: u32, id: String, state: State<'_, AppState>) -> Result<OpOutcome> {
    let r = get(&state, repo_id)?;
    blocking(move || Ok(engine(&r)?.undo(&id)?)).await
}

#[tauri::command]
async fn op_redo(repo_id: u32, state: State<'_, AppState>) -> Result<OpOutcome> {
    let r = get(&state, repo_id)?;
    blocking(move || Ok(engine(&r)?.redo_last()?)).await
}

#[tauri::command]
async fn journal_list(repo_id: u32, state: State<'_, AppState>) -> Result<Vec<JournalEntry>> {
    let r = get(&state, repo_id)?;
    blocking(move || Ok(engine(&r)?.journal())).await
}

#[tauri::command]
async fn journal_prune(repo_id: u32, keep: usize, state: State<'_, AppState>) -> Result<usize> {
    let r = get(&state, repo_id)?;
    blocking(move || Ok(engine(&r)?.prune_journal(keep)?)).await
}

fn main() {
    // Started by Git/ssh as the askpass program: forward the prompt to the running app and exit.
    if let Ok(sock) = std::env::var(askpass::SOCK_ENV) {
        std::process::exit(askpass::client(&sock));
    }
    let initial_path = std::env::args_os().nth(1).map(PathBuf::from);
    let diag = diagnostics::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { initial_path, ..Default::default() })
        .manage(diag)
        .invoke_handler(tauri::generate_handler![
            get_initial_path,
            diagnostics_status,
            diagnostics_dismiss_crash,
            diagnostics_open_log_dir,
            repo_open,
            repo_refresh,
            repo_close,
            graph_window,
            graph_rows,
            commit_details,
            diff_file,
            diff_worktree,
            status_get,
            op_stage,
            op_unstage,
            op_stage_hunk,
            op_discard,
            op_commit,
            op_branch_create,
            op_branch_create_from,
            branch_plan,
            op_branch_rename,
            op_branch_delete,
            op_checkout,
            merge_preview,
            stash_list,
            op_merge,
            op_merge_abort,
            op_resolve,
            conflict_versions,
            open_external,
            op_resolve_with,
            op_stash,
            op_history,
            op_net,
            credentials_answer,
            rebase_todo,
            op_rebase,
            op_rebase_continue,
            op_undo,
            op_redo,
            journal_list,
            journal_prune,
            shared_stash::shared_stash_list,
            shared_stash::shared_stash_fetch,
            shared_stash::shared_stash_publish,
            shared_stash::shared_stash_update,
            shared_stash::shared_stash_unshare,
            shared_stash::shared_stash_reshare,
            shared_stash::shared_stash_discard_keep,
            shared_stash::shared_stash_apply,
            shared_stash::shared_stash_branch,
            shared_stash::shared_stash_probe_reset,
            shared_stash::shared_stash_tab_open,
            shared_stash::shared_stash_cancel,
            shared_stash::shared_stash_settings_get,
            shared_stash::shared_stash_settings_set,
            prefs::prefs_get,
            prefs::prefs_set,
            session_get,
            session_save_tabs,
            recents_remove
        ])
        .build(tauri::generate_context!())
        .expect("error while building AtlasGit")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<diagnostics::Diagnostics>().finish();
            }
        });
}

#[tauri::command]
fn diagnostics_status(diag: State<'_, diagnostics::Diagnostics>) -> diagnostics::DiagnosticsStatus {
    diag.status()
}

#[tauri::command]
fn diagnostics_dismiss_crash(diag: State<'_, diagnostics::Diagnostics>) {
    diag.dismiss_crash();
}

#[tauri::command]
fn diagnostics_open_log_dir(app: AppHandle, diag: State<'_, diagnostics::Diagnostics>) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    let dir = diag.status().log_dir;
    app.opener().open_path(dir, None::<&str>).map_err(|e| UiError::new("OPEN_FAILED", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignored_trees_at_any_depth_are_not_watched() {
        let root = std::env::temp_dir().join(format!("atlas-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("packages/a/node_modules/x")).unwrap();
        std::fs::create_dir_all(root.join("packages/a/src")).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join(".gitignore"), "node_modules/\n").unwrap();
        let init = std::process::Command::new("git").arg("-C").arg(&root).args(["init", "-q"]).status().unwrap();
        assert!(init.success());
        let ignored = ignored_dirs(&root);
        assert!(ignored.contains("packages/a/node_modules/"), "{ignored:?}");
        let mut dirs = Vec::new();
        collect_watch_dirs(&root, &root, &ignored, &mut dirs);
        let keys: Vec<String> = dirs.iter().map(|d| dir_key(&root, d)).collect();
        assert!(keys.contains(&"packages/a/src/".to_owned()), "{keys:?}");
        assert!(keys.contains(&"docs/".to_owned()), "{keys:?}");
        assert!(!keys.iter().any(|k| k.contains("node_modules")), "{keys:?}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
