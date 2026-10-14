// SPDX-License-Identifier: Apache-2.0
//! AtlasKraken Tauri shell. The UI can only reach the commands below
//! (SDD §4.1, Phase 1 read-only subset). All Git access stays in the core.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use ak_core::graph::{self, Layout, Topology};
use ak_core::read::{CliReader, CommitDetails, RepoReader};
use ak_gitcli::parse::{CommitRecord, RefKind, StatusReport};
use ak_gitcli::{Git, Mode};
use serde::Serialize;
use tauri::ipc::Response;
use tauri::State;

struct Repo {
    reader: CliReader,
    commits: Vec<CommitRecord>,
    topo: Topology,
    layout: Layout,
}

#[derive(Default)]
struct AppState {
    repos: Mutex<HashMap<u32, Repo>>,
    next_id: Mutex<u32>,
    initial_path: Option<PathBuf>,
}

/// Errors cross the IPC boundary as `{ code, message }` (SDD §4.4).
#[derive(Debug, Serialize)]
struct UiError {
    code: &'static str,
    message: String,
}

impl UiError {
    fn new(code: &'static str, e: impl ToString) -> Self {
        Self { code, message: e.to_string() }
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RowMeta<'a> {
    sha: &'a str,
    summary: &'a str,
    author: &'a str,
    email: &'a str,
    time: i64,
}

#[tauri::command]
fn get_initial_path(state: State<'_, AppState>) -> Option<String> {
    state.initial_path.as_ref().map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
fn repo_open(path: String, state: State<'_, AppState>) -> Result<OpenedRepo> {
    // Phase 1 is read-only by design (roadmap constraint).
    let git = Git::discover(Mode::ReadOnly).map_err(|e| UiError::new("GIT_NOT_FOUND", e))?;
    let caps = git.caps().clone();
    let reader = CliReader::open(git, &PathBuf::from(&path)).map_err(|e| UiError::new("NOT_A_REPO", e))?;
    let commits = reader.commits().map_err(|e| UiError::new("READ_FAILED", e))?;
    let refs = reader.refs().map_err(|e| UiError::new("READ_FAILED", e))?;
    let status = reader.status().map_err(|e| UiError::new("READ_FAILED", e))?;
    let head = reader.head().map_err(|e| UiError::new("READ_FAILED", e))?;
    let topo = Topology::from_commits(commits.iter().map(|c| (c.sha.as_str(), c.parents.as_slice())));
    let layout = graph::layout(&topo);

    let row_of: HashMap<&str, u32> = commits.iter().enumerate().map(|(i, c)| (c.sha.as_str(), i as u32)).collect();
    let refs = refs
        .into_iter()
        .map(|r| RefInfo { row: row_of.get(r.target.as_str()).copied(), name: r.short, kind: r.kind, is_head: r.is_head })
        .collect();
    let root = reader.root().to_path_buf();
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
        read_only_reason: (!caps.supported()).then(|| "Git older than 2.20: read-only".to_owned()),
    };
    let mut id = state.next_id.lock().unwrap();
    *id += 1;
    let repo_id = *id;
    state.repos.lock().unwrap().insert(repo_id, Repo { reader, commits, topo, layout });
    Ok(OpenedRepo { repo_id, ..opened })
}

#[tauri::command]
fn repo_close(repo_id: u32, state: State<'_, AppState>) {
    state.repos.lock().unwrap().remove(&repo_id);
}

fn with_repo<T>(state: &AppState, repo_id: u32, f: impl FnOnce(&Repo) -> Result<T>) -> Result<T> {
    let repos = state.repos.lock().unwrap();
    let repo = repos.get(&repo_id).ok_or_else(|| UiError::new("UNKNOWN_REPO", repo_id))?;
    f(repo)
}

/// Binary RowWindow (SDD §4.3).
#[tauri::command]
fn graph_window(repo_id: u32, start: u32, count: u32, state: State<'_, AppState>) -> Result<Response> {
    with_repo(&state, repo_id, |r| Ok(Response::new(graph::encode_window(&r.topo, &r.layout, start, count.min(2000)))))
}

#[tauri::command]
fn graph_rows(repo_id: u32, start: u32, count: u32, state: State<'_, AppState>) -> Result<serde_json::Value> {
    with_repo(&state, repo_id, |r| {
        let s = (start as usize).min(r.commits.len());
        let e = (s + count.min(2000) as usize).min(r.commits.len());
        let rows: Vec<RowMeta> = r.commits[s..e]
            .iter()
            .map(|c| RowMeta { sha: &c.sha, summary: &c.summary, author: &c.author, email: &c.email, time: c.time })
            .collect();
        serde_json::to_value(rows).map_err(|e| UiError::new("INTERNAL", e))
    })
}

#[tauri::command]
fn commit_details(repo_id: u32, sha: String, state: State<'_, AppState>) -> Result<CommitDetails> {
    with_repo(&state, repo_id, |r| r.reader.commit_details(&sha).map_err(|e| UiError::new("READ_FAILED", e)))
}

#[tauri::command]
fn diff_file(repo_id: u32, sha: String, path: String, state: State<'_, AppState>) -> Result<String> {
    with_repo(&state, repo_id, |r| r.reader.file_diff(&sha, &path).map_err(|e| UiError::new("READ_FAILED", e)))
}

#[tauri::command]
fn status_get(repo_id: u32, state: State<'_, AppState>) -> Result<StatusReport> {
    with_repo(&state, repo_id, |r| r.reader.status().map_err(|e| UiError::new("READ_FAILED", e)))
}

fn main() {
    let initial_path = std::env::args_os().nth(1).map(PathBuf::from);
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { initial_path, ..Default::default() })
        .invoke_handler(tauri::generate_handler![
            get_initial_path,
            repo_open,
            repo_close,
            graph_window,
            graph_rows,
            commit_details,
            diff_file,
            status_get
        ])
        .run(tauri::generate_context!())
        .expect("error while running AtlasKraken");
}
