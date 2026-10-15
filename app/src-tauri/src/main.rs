// SPDX-License-Identifier: Apache-2.0
//! AtlasKraken Tauri shell. The UI can only reach the commands below
//! (SDD §4.1). All Git access stays in the core.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod askpass;

use std::collections::HashMap;
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
use tauri::{AppHandle, Emitter, State};

/// What stays in memory per repository: topology and layout only. Commit text
/// (summary, author) is fetched from Git for the rows on screen.
struct Repo {
    reader: CliReader,
    engine: Option<Engine>,
    shas: Vec<String>,
    topo: Topology,
    layout: Layout,
    _watcher: Option<SharedWatcher>,
}

#[derive(Default)]
struct AppState {
    repos: Mutex<HashMap<u32, Arc<Repo>>>,
    askpass: Arc<askpass::Pending>,
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
    match parts.next().as_deref() {
        Some("index") | Some("HEAD") | Some("refs") | Some("MERGE_HEAD") => true,
        _ => false,
    }
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

fn is_ignored_dir(root: &Path, name: &str) -> bool {
    std::process::Command::new("git")
        .current_dir(root)
        .args(["check-ignore", "-q", "--", &format!("{name}/")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn setup_watcher(app: AppHandle, repo_id: u32, root: &Path) -> Option<SharedWatcher> {
    let (tx, rx) = std::sync::mpsc::channel::<WatchMsg>();
    let base = root.to_path_buf();
    let w = notify::recommended_watcher(move |ev: notify::Result<notify::Event>| {
        if let Ok(ev) = ev {
            // A new top-level directory: the root is watched non-recursively, so add it later
            // (not here: adding a watch from inside the event callback can deadlock the backend).
            if matches!(ev.kind, notify::EventKind::Create(_)) {
                for p in ev.paths.iter().filter(|p| p.parent() == Some(base.as_path()) && p.is_dir()) {
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
    // Watch each top-level entry except ignored dirs (node_modules, target...): saves inotify watches.
    let ignored: Vec<String> = std::process::Command::new("git")
        .current_dir(root)
        .args(["ls-files", "--others", "--ignored", "--exclude-standard", "--directory", "-z"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).split('\0').map(|d| d.trim_end_matches('/').to_string()).collect())
        .unwrap_or_default();
    {
        let mut g = w.lock().unwrap();
        g.watch(root, RecursiveMode::NonRecursive).ok()?;
        for e in std::fs::read_dir(root).ok()?.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().is_dir() && !ignored.contains(&name) {
                g.watch(&e.path(), RecursiveMode::Recursive).ok()?;
            }
        }
    }
    let weak = Arc::downgrade(&w);
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        let handle = |m: WatchMsg| {
            if let WatchMsg::NewDir(p) = m {
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if name != ".git" && !is_ignored_dir(&root, &name) {
                    if let Some(w) = weak.upgrade() {
                        let _ = w.lock().unwrap().watch(&p, RecursiveMode::Recursive);
                    }
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
    Ok((Repo { reader, engine, shas, topo, layout, _watcher: None }, opened))
}

#[tauri::command]
fn get_initial_path(state: State<'_, AppState>) -> Option<String> {
    state.initial_path.as_ref().map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
async fn repo_open(path: String, app: AppHandle, state: State<'_, AppState>) -> Result<OpenedRepo> {
    let (mut repo, opened) = load(&PathBuf::from(&path))?;
    let repo_id = {
        let mut id = state.next_id.lock().unwrap();
        *id += 1;
        *id
    };
    repo._watcher = start_watcher(app, repo_id, repo.reader.root());
    state.repos.lock().unwrap().insert(repo_id, Arc::new(repo));
    Ok(OpenedRepo { repo_id, ..opened })
}

/// Re-reads history and refs after the repository changed (commit, undo, external edits).
#[tauri::command]
async fn repo_refresh(repo_id: u32, app: AppHandle, state: State<'_, AppState>) -> Result<OpenedRepo> {
    let root = get(&state, repo_id)?.reader.root().to_path_buf();
    let (mut repo, opened) = load(&root)?;
    repo._watcher = start_watcher(app, repo_id, &root);
    state.repos.lock().unwrap().insert(repo_id, Arc::new(repo));
    Ok(OpenedRepo { repo_id, ..opened })
}

#[tauri::command]
fn repo_close(repo_id: u32, state: State<'_, AppState>) {
    state.repos.lock().unwrap().remove(&repo_id);
}

fn get(state: &AppState, repo_id: u32) -> Result<Arc<Repo>> {
    state.repos.lock().unwrap().get(&repo_id).cloned().ok_or_else(|| UiError::new("UNKNOWN_REPO", repo_id))
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
    let wanted: Vec<&str> = r.shas[s..e].iter().map(String::as_str).collect();
    r.reader.rows_text(&wanted).map_err(|e| UiError::new("READ_FAILED", e))
}

#[tauri::command]
async fn commit_details(repo_id: u32, sha: String, state: State<'_, AppState>) -> Result<CommitDetails> {
    get(&state, repo_id)?.reader.commit_details(&sha).map_err(|e| UiError::new("READ_FAILED", e))
}

#[tauri::command]
async fn diff_file(repo_id: u32, sha: String, path: String, state: State<'_, AppState>) -> Result<String> {
    get(&state, repo_id)?.reader.file_diff(&sha, &path).map_err(|e| UiError::new("READ_FAILED", e))
}

#[tauri::command]
async fn diff_worktree(
    repo_id: u32,
    path: String,
    staged: bool,
    untracked: bool,
    state: State<'_, AppState>,
) -> Result<String> {
    get(&state, repo_id)?
        .reader
        .worktree_diff(&path, staged, untracked)
        .map_err(|e| UiError::new("READ_FAILED", e))
}

#[tauri::command]
async fn status_get(repo_id: u32, state: State<'_, AppState>) -> Result<StatusWithMerge> {
    let r = get(&state, repo_id)?;
    let status = r.reader.status().map_err(|e| UiError::new("READ_FAILED", e))?;
    let in_progress = engine(&r)?.in_progress();
    Ok(StatusWithMerge { status, merging: in_progress.is_some(), in_progress })
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
    Ok(engine(&r)?.merge_preview(&source)?)
}

#[tauri::command]
async fn stash_list(repo_id: u32, state: State<'_, AppState>) -> Result<Vec<StashInfo>> {
    let r = get(&state, repo_id)?;
    Ok(engine(&r)?.stashes()?)
}

#[tauri::command]
async fn op_merge(repo_id: u32, source: String, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Merge { source })
}

#[tauri::command]
async fn op_merge_abort(repo_id: u32, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::MergeAbort)
}

#[tauri::command]
async fn conflict_versions(repo_id: u32, path: String, state: State<'_, AppState>) -> Result<ConflictVersions> {
    let r = get(&state, repo_id)?;
    Ok(engine(&r)?.conflict_versions(&path)?)
}

#[tauri::command]
async fn op_resolve_with(repo_id: u32, path: String, content: String, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::ResolveWith { path, content })
}

#[tauri::command]
async fn op_resolve(repo_id: u32, path: String, theirs: bool, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::ResolveConflict { path, theirs })
}

#[tauri::command]
async fn op_stash(repo_id: u32, action: String, index: usize, message: Option<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    let op = match action.as_str() {
        "push" => Operation::StashPush { message, include_untracked: true },
        "apply" => Operation::StashApply { index },
        "pop" => Operation::StashPop { index },
        _ => Operation::StashDrop { index },
    };
    run_op(&state, repo_id, op)
}

fn run_op(state: &AppState, repo_id: u32, op: Operation) -> Result<OpOutcome> {
    let r = get(state, repo_id)?;
    Ok(engine(&r)?.run(&op)?)
}

#[tauri::command]
async fn op_stage(repo_id: u32, paths: Vec<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Stage { paths })
}

#[tauri::command]
async fn op_unstage(repo_id: u32, paths: Vec<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Unstage { paths })
}

#[tauri::command]
async fn op_stage_hunk(
    repo_id: u32,
    path: String,
    hunk: usize,
    unstage: bool,
    state: State<'_, AppState>,
) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::StageHunk { path, hunk, unstage })
}

#[tauri::command]
async fn op_discard(repo_id: u32, paths: Vec<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Discard { paths })
}

#[tauri::command]
async fn op_commit(repo_id: u32, message: String, amend: bool, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Commit { message, amend })
}

#[tauri::command]
async fn op_branch_create(repo_id: u32, name: String, at: Option<String>, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::BranchCreate { name, at })
}

#[tauri::command]
async fn op_branch_rename(repo_id: u32, old: String, new: String, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::BranchRename { old, new })
}

#[tauri::command]
async fn op_branch_delete(repo_id: u32, name: String, force: bool, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::BranchDelete { name, force })
}

#[tauri::command]
async fn op_checkout(repo_id: u32, target: String, is_branch: bool, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Checkout { target, is_branch })
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
    tauri::async_runtime::spawn_blocking(move || {
        let e = engine(&r)?;
        Ok(askpass::with_bridge(&app, pending, repo_id, |env| e.run_with_env(&op, env))?)
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
    run_op(&state, repo_id, op)
}

#[tauri::command]
async fn rebase_todo(repo_id: u32, onto: String, state: State<'_, AppState>) -> Result<Vec<TodoItem>> {
    let r = get(&state, repo_id)?;
    Ok(engine(&r)?.rebase_todo(&onto)?)
}

#[tauri::command]
async fn op_rebase(repo_id: u32, onto: String, todo: Option<Vec<TodoItem>>, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::Rebase { onto, todo })
}

#[tauri::command]
async fn op_rebase_continue(repo_id: u32, state: State<'_, AppState>) -> Result<OpOutcome> {
    run_op(&state, repo_id, Operation::RebaseContinue)
}

#[tauri::command]
async fn op_undo(repo_id: u32, id: String, state: State<'_, AppState>) -> Result<OpOutcome> {
    let r = get(&state, repo_id)?;
    Ok(engine(&r)?.undo(&id)?)
}

#[tauri::command]
async fn op_redo(repo_id: u32, state: State<'_, AppState>) -> Result<OpOutcome> {
    let r = get(&state, repo_id)?;
    Ok(engine(&r)?.redo_last()?)
}

#[tauri::command]
async fn journal_list(repo_id: u32, state: State<'_, AppState>) -> Result<Vec<JournalEntry>> {
    let r = get(&state, repo_id)?;
    Ok(engine(&r)?.journal())
}

#[tauri::command]
async fn journal_prune(repo_id: u32, keep: usize, state: State<'_, AppState>) -> Result<usize> {
    let r = get(&state, repo_id)?;
    Ok(engine(&r)?.prune_journal(keep)?)
}

fn main() {
    // Started by Git/ssh as the askpass program: forward the prompt to the running app and exit.
    if let Ok(sock) = std::env::var(askpass::SOCK_ENV) {
        std::process::exit(askpass::client(&sock));
    }
    let initial_path = std::env::args_os().nth(1).map(PathBuf::from);
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { initial_path, ..Default::default() })
        .invoke_handler(tauri::generate_handler![
            get_initial_path,
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
            op_branch_rename,
            op_branch_delete,
            op_checkout,
            merge_preview,
            stash_list,
            op_merge,
            op_merge_abort,
            op_resolve,
            conflict_versions,
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
            journal_prune
        ])
        .run(tauri::generate_context!())
        .expect("error while running AtlasKraken");
}
